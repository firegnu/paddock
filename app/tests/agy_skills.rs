//! The Settings skill flow and headless CLI, with an isolated HOME and fake tools.
mod common;

use paddock::integrations::{self, Skills};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command, sync::atomic::AtomicBool};

#[test]
fn agy_skills_include_paddock_and_install_only_after_confirmation() {
    // A child test process isolates HOME/PATH without changing the parallel test runner.
    const CHILD: &str = "PADDOCK_TEST_AGY_SKILLS_HOME";
    let Some(home) = std::env::var_os(CHILD) else {
        let home = common::tempdir();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "agy_skills_include_paddock_and_install_only_after_confirmation",
                "--nocapture",
            ])
            .env(CHILD, home.path())
            .env("HOME", home.path())
            .env("PATH", home.path())
            .env_remove("CLAUDE_CONFIG_DIR")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    };
    let home = PathBuf::from(home);
    let skill = home.join(".gemini/config/skills/paddock/SKILL.md");
    let corral = common::script(
        &home,
        "corral",
        &format!(
            "#!/bin/sh\n[ \"$*\" = 'integration status' ] || exit 1\nprintf '%s\\n' '{}'\n",
            json!({"ok":true,"agents":[{
                "kind":"agy","path":"/fake/agy","hooks":"global",
                "skills":[{"target":"gemini","path":home.join(".gemini/config/skills/corral/SKILL.md"),"status":"same"}]
            }]})
        ),
    );
    common::script(
        &home,
        "ranch",
        &format!(
            "#!/bin/sh\n[ \"$*\" = 'dispatch install-skills --dry-run' ] || exit 1\nprintf '%s\\n' '{}'\n",
            json!({"ok":true,"items":[{
                "agent":"gemini","path":home.join(".gemini/config/skills/corral-dispatch/SKILL.md"),"status":"same"
            }]})
        ),
    );
    let cancel = AtomicBool::new(false);
    let report = integrations::read(&corral, &cancel).unwrap();
    let rows = integrations::rows(&report);
    assert_eq!(rows[0].skills, Skills::Act { install: false });
    assert_eq!(rows[0].note.as_deref(), Some("paddock missing"));
    let plan = integrations::plan(&report, "agy").unwrap();
    assert_eq!(plan.steps, [("paddock", "gemini".into())]);
    assert_eq!(plan.files, [(skill.display().to_string(), false)]);
    assert!(!skill.exists(), "reading and planning must not write");

    // Settings calls install only after the person confirms this file list.
    integrations::install(&corral, &plan, &cancel).unwrap();
    let body = fs::read_to_string(&skill).unwrap();
    assert!(body.contains("由 paddock install-skills 写入"));
    assert!(!home.join(".claude").exists());
    assert!(!home.join(".agents").exists());
    let report = integrations::read(&corral, &cancel).unwrap();
    assert_eq!(integrations::rows(&report)[0].skills, Skills::UpToDate);
    assert_eq!(integrations::plan(&report, "agy"), None);

    fs::remove_file(&skill).unwrap();
    let cli = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_paddock"))
            .arg("install-skills")
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let dry = cli(&["--dry-run"]);
    assert_eq!(dry["items"].as_array().unwrap().len(), 3);
    assert_eq!(dry["items"][2]["agent"], "gemini");
    assert!(!skill.exists());
    assert_eq!(cli(&["--yes"])["written"], true);
    for base in [".claude", ".agents", ".gemini/config"] {
        assert_eq!(
            fs::read_to_string(home.join(base).join("skills/paddock/SKILL.md")).unwrap(),
            body
        );
    }
    assert!(
        cli(&["--help"])["help"]
            .as_str()
            .unwrap()
            .contains("~/.gemini/config/skills")
    );
}
