// Adapted from ranch `crates/corral/src/skills.rs` at commit
// `240b045cb95be18f01bef4d0a8218ce074e2618e` (originally From Saddle at `a31dea2`).
// Paddock-only fixed destinations, explicit ownership on overwrite and removal,
// injected home/confirmation for isolated tests, and no new dependencies.
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{
    fs,
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
};

#[path = "skills_files.rs"]
mod files;

const BODY: &str = include_str!("../resources/skills/paddock/SKILL.md");
const MARKER: &str = "<!-- paddock-skill:";

#[derive(Default)]
struct Options {
    remove: bool,
    dry: bool,
    yes: bool,
}

pub fn run(args: Vec<String>) -> i32 {
    let result =
        execute(args).unwrap_or_else(|e| crate::control::error("install_skills", format!("{e:#}")));
    println!("{result}");
    if result["ok"] == true { 0 } else { 1 }
}
fn execute(args: Vec<String>) -> Result<Value> {
    if args == ["--help"] || args == ["-h"] {
        return Ok(
            json!({"ok":true,"help":"paddock install-skills [--yes] [--dry-run] [--remove]\nWrites only owned paddock skills in ~/.claude/skills and ~/.agents/skills."}),
        );
    }
    let mut options = Options::default();
    for arg in args {
        let flag = match arg.as_str() {
            "--yes" => &mut options.yes,
            "--dry-run" => &mut options.dry,
            "--remove" => &mut options.remove,
            _ => bail!("unknown install-skills option {arg}"),
        };
        anyhow::ensure!(!*flag, "duplicate option {arg}");
        *flag = true;
    }
    let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
    run_at(&home, &options, |plan| {
        eprintln!("paddock install-skills: {}", plan["action"]);
        for item in plan["items"].as_array().unwrap() {
            eprintln!(
                "  [{}] {}",
                item["status"].as_str().unwrap(),
                item["path"].as_str().unwrap()
            );
        }
        if options.yes {
            return Ok(true);
        }
        if !std::io::stdin().is_terminal() {
            return Ok(false);
        }
        eprint!("确认以上改动？[y/N] ");
        std::io::stderr().flush()?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
    })
}
fn linked(path: &Path, home: &Path) -> Result<bool> {
    for part in path.ancestors() {
        match fs::symlink_metadata(part) {
            Ok(m) if m.file_type().is_symlink() => return Ok(true),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        if part == home {
            break;
        }
    }
    Ok(false)
}
fn status(path: &Path, home: &Path, remove: bool) -> Result<&'static str> {
    if linked(path, home)? {
        return Ok("foreign");
    }
    match fs::symlink_metadata(path) {
        Ok(m) if !m.is_file() => return Ok("foreign"),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(if remove { "absent" } else { "create" });
        }
        Err(e) => return Err(e.into()),
    }
    let bytes = fs::read(path)?;
    let Ok(current) = std::str::from_utf8(&bytes) else {
        return Ok("foreign");
    };
    if remove {
        Ok(if current.contains(MARKER) {
            "remove"
        } else {
            "foreign"
        })
    } else {
        Ok(if current == BODY {
            "same"
        } else if current.contains(MARKER) {
            "overwrite"
        } else {
            "foreign"
        })
    }
}
fn run_at(
    home: &Path,
    options: &Options,
    confirm: impl FnOnce(&Value) -> Result<bool>,
) -> Result<Value> {
    run_at_with(home, options, confirm, |_| Ok(()))
}
fn run_at_with(
    home: &Path,
    options: &Options,
    confirm: impl FnOnce(&Value) -> Result<bool>,
    mut before_publish: impl FnMut(&Path) -> Result<()>,
) -> Result<Value> {
    anyhow::ensure!(
        home.is_absolute() && home.is_dir(),
        "HOME must be an existing absolute directory"
    );
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    let mut snapshots = Vec::new();
    for (agent, base) in [("claude", ".claude"), ("codex", ".agents")] {
        let path = home.join(base).join("skills/paddock/SKILL.md");
        snapshots.push(files::Snapshot::capture(home, &path)?);
        let state = status(&path, home, options.remove)?;
        if state == "foreign" {
            warnings.push(format!(
                "{} is not an owned regular skill file; left in place",
                path.display()
            ));
        }
        items.push(json!({"agent":agent,"path":path,"status":state}));
    }
    let mut result = json!({"ok":true,"action":if options.remove {"remove"} else {"install"},
        "dry_run":options.dry,"written":false,"items":items,"warnings":warnings});
    let todo: Vec<_> = items
        .iter()
        .zip(&snapshots)
        .filter(|(i, _)| {
            matches!(
                i["status"].as_str(),
                Some("create" | "overwrite" | "remove")
            )
        })
        .collect();
    if options.dry || todo.is_empty() {
        return Ok(result);
    }
    if !confirm(&result)? {
        result["ok"] = json!(false);
        result["error"] = json!({"code":"confirmation_required","message":"nothing written; confirm interactively or pass --yes"});
        return Ok(result);
    }
    for (item, snapshot) in todo {
        let path = PathBuf::from(item["path"].as_str().unwrap());
        anyhow::ensure!(
            status(&path, home, options.remove)? == item["status"].as_str().unwrap(),
            "skill changed after planning; rerun install-skills"
        );
        files::apply(home, &path, snapshot, options.remove, &mut before_publish)
            .with_context(|| format!("refused skill change: {}", path.display()))?;
    }
    result["written"] = json!(true);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn home() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "paddock-skills-{}",
            crate::control::random_id().unwrap()
        ));
        fs::create_dir(&path).unwrap();
        path
    }
    #[test]
    fn foreign_files_are_never_overwritten() {
        let home = home();
        let path = home.join(".claude/skills/paddock/SKILL.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "my own skill").unwrap();
        let result = run_at(&home, &Options::default(), |_| Ok(true)).unwrap();
        assert_eq!(result["items"][0]["status"], "foreign");
        assert_eq!(fs::read_to_string(path).unwrap(), "my own skill");
        assert_eq!(result["warnings"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn dry_run_decline_confirm_owned_updates_and_removal() {
        let home = home();
        let path = home.join(".agents/skills/paddock/SKILL.md");
        let dry = run_at(
            &home,
            &Options {
                dry: true,
                ..Default::default()
            },
            |_| panic!("dry run must not confirm"),
        )
        .unwrap();
        assert_eq!(dry["written"], false);
        assert!(!home.join(".agents").exists());
        let decline = run_at(&home, &Options::default(), |_| Ok(false)).unwrap();
        assert_eq!(decline["ok"], false);
        assert!(!path.exists());
        let install = run_at(&home, &Options::default(), |plan| {
            assert_eq!(plan["items"].as_array().unwrap().len(), 2);
            assert!(!path.exists());
            Ok(true)
        })
        .unwrap();
        assert_eq!(install["written"], true);
        assert_eq!(fs::read_to_string(&path).unwrap(), BODY);
        let same = run_at(&home, &Options::default(), |_| panic!("nothing to write")).unwrap();
        assert_eq!(same["written"], false);
        fs::write(&path, format!("{MARKER} old -->\nold text")).unwrap();
        let update = run_at(&home, &Options::default(), |_| Ok(true)).unwrap();
        assert_eq!(update["items"][1]["status"], "overwrite");
        assert_eq!(fs::read_to_string(&path).unwrap(), BODY);
        let foreign = home.join(".claude/skills/paddock/SKILL.md");
        fs::write(&foreign, "user skill").unwrap();
        let remove = run_at(
            &home,
            &Options {
                remove: true,
                ..Default::default()
            },
            |_| Ok(true),
        )
        .unwrap();
        assert_eq!(remove["items"][0]["status"], "foreign");
        assert!(!path.exists());
        assert_eq!(fs::read_to_string(foreign).unwrap(), "user skill");
    }
    #[test]
    fn symlinks_foreign_binary_files_and_changed_plan_are_left_alone() {
        let home = home();
        let elsewhere = home.join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, home.join(".claude")).unwrap();
        let other = home.join(".agents/skills/paddock/SKILL.md");
        fs::create_dir_all(other.parent().unwrap()).unwrap();
        fs::write(&other, [0xff, 0xfe]).unwrap();
        let result = run_at(&home, &Options::default(), |_| panic!("foreign only")).unwrap();
        assert!(
            result["items"]
                .as_array()
                .unwrap()
                .iter()
                .all(|i| i["status"] == "foreign")
        );
        assert!(fs::read_dir(&elsewhere).unwrap().next().is_none());
        fs::write(&other, BODY).unwrap();
        let changed = run_at(
            &home,
            &Options {
                remove: true,
                ..Default::default()
            },
            |_| {
                fs::write(&other, "new user content")?;
                Ok(true)
            },
        );
        assert!(changed.is_err());
        assert_eq!(fs::read_to_string(&other).unwrap(), "new user content");
    }
    #[test]
    fn target_created_after_final_check_is_preserved() {
        let home = home();
        let path = home.join(".claude/skills/paddock/SKILL.md");
        let result = run_at_with(
            &home,
            &Options::default(),
            |_| Ok(true),
            |target| {
                fs::write(target, "FOREIGN AFTER FINAL CHECK")?;
                Ok(())
            },
        );
        assert!(
            result.is_err(),
            "must report the new target instead of overwriting it"
        );
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "FOREIGN AFTER FINAL CHECK"
        );
    }
    #[test]
    fn update_and_remove_refuse_files_swapped_after_final_check() {
        for remove in [false, true] {
            let home = home();
            let path = home.join(".claude/skills/paddock/SKILL.md");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, format!("{MARKER} old -->\nold skill")).unwrap();
            let result = run_at_with(
                &home,
                &Options {
                    remove,
                    ..Default::default()
                },
                |_| Ok(true),
                |target| {
                    fs::rename(target, target.with_extension("old"))?;
                    fs::write(target, "FOREIGN REPLACEMENT")?;
                    Ok(())
                },
            );
            assert!(result.is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), "FOREIGN REPLACEMENT");
            assert!(
                fs::read_to_string(path.with_extension("old"))
                    .unwrap()
                    .contains("old skill")
            );
        }
    }
    #[test]
    fn parent_swap_after_final_check_does_not_touch_replacement_directory() {
        for remove in [false, true] {
            let home = home();
            let parent = home.join(".claude/skills/paddock");
            fs::create_dir_all(&parent).unwrap();
            fs::write(
                parent.join("SKILL.md"),
                format!("{MARKER} old -->\nold skill"),
            )
            .unwrap();
            let result = run_at_with(
                &home,
                &Options {
                    remove,
                    ..Default::default()
                },
                |_| Ok(true),
                |target| {
                    let directory = target.parent().unwrap();
                    fs::rename(directory, directory.with_extension("saved"))?;
                    fs::create_dir(directory)?;
                    fs::write(target, "FOREIGN DIRECTORY")?;
                    Ok(())
                },
            );
            assert!(result.is_err());
            assert_eq!(
                fs::read_to_string(parent.join("SKILL.md")).unwrap(),
                "FOREIGN DIRECTORY"
            );
            assert!(
                fs::read_to_string(parent.with_extension("saved").join("SKILL.md"))
                    .unwrap()
                    .contains("old skill")
            );
        }
    }
    #[test]
    fn replaced_owned_file_during_confirmation_is_not_approved_by_marker_alone() {
        let home = home();
        let path = home.join(".claude/skills/paddock/SKILL.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("{MARKER} old -->\nold skill")).unwrap();
        let result = run_at(&home, &Options::default(), |_| {
            fs::rename(&path, path.with_extension("saved"))?;
            fs::write(&path, format!("{MARKER} other -->\nother owned file"))?;
            Ok(true)
        });
        assert!(result.is_err());
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("other owned file")
        );
    }
}
