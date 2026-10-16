//! Settings › Instructions, the side that only reads (DESIGN §13 P5-119): where each agent's
//! global instructions file is, and what is read from it, with made-up files in a temporary
//! home folder. Nothing here saves, backs up or creates a file.
mod common;

use paddock::instructions::{self, State};
use std::{ffi::OsString, fs, os::unix::fs::symlink, path::Path};

fn unset(_: &str) -> Option<OsString> {
    None
}

#[test]
fn each_agent_reads_its_one_file_under_home() {
    let home = Path::new("/h");
    let path = |kind| instructions::path(kind, home, unset).unwrap();
    assert_eq!(path("claude"), home.join(".claude/CLAUDE.md"));
    assert_eq!(path("codex"), home.join(".codex/AGENTS.md"));
    assert_eq!(path("pi"), home.join(".pi/agent/AGENTS.md"));
    assert_eq!(path("omp"), home.join(".omp/agent/AGENTS.md"));
    assert_eq!(path("amp"), home.join(".config/amp/AGENTS.md"));
    assert_eq!(path("agy"), home.join(".gemini/GEMINI.md"));
    // Every agent paddock hosts has one, in its order; nothing else does.
    assert_eq!(
        instructions::KINDS,
        ["claude", "codex", "pi", "omp", "amp", "agy"]
    );
    assert_eq!(instructions::path("zsh", home, unset), None);
}

#[test]
fn an_agents_own_folder_variable_moves_its_file() {
    let home = Path::new("/h");
    // Every variable set: only Claude Code's, Codex's and pi's own count.
    let set = |name: &str| {
        Some(OsString::from(match name {
            "CLAUDE_CONFIG_DIR" => "/claude",
            "CODEX_HOME" => "/codex",
            "PI_CODING_AGENT_DIR" => "/pi",
            _ => "/elsewhere",
        }))
    };
    let path = |kind| instructions::path(kind, home, set).unwrap();
    assert_eq!(path("claude"), Path::new("/claude/CLAUDE.md"));
    assert_eq!(path("codex"), Path::new("/codex/AGENTS.md"));
    assert_eq!(path("pi"), Path::new("/pi/AGENTS.md"));
    assert_eq!(path("omp"), home.join(".omp/agent/AGENTS.md"));
    assert_eq!(path("amp"), home.join(".config/amp/AGENTS.md"));
    assert_eq!(path("agy"), home.join(".gemini/GEMINI.md"));
    // Set but empty is as good as unset.
    let empty = |_: &str| Some(OsString::new());
    assert_eq!(
        instructions::path("claude", home, empty).unwrap(),
        home.join(".claude/CLAUDE.md")
    );
}

#[test]
fn a_file_not_there_yet_reads_as_missing_and_nothing_is_made() {
    let home = common::tempdir();
    for kind in ["pi", "omp"] {
        let path = instructions::path(kind, home.path(), unset).unwrap();
        let file = instructions::read(kind, path.clone());
        assert_eq!(file.state, State::Missing);
        assert!(!file.exists());
        assert_eq!((file.path, file.target), (path.clone(), path));
    }
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn a_file_reads_with_its_text_and_size() {
    let home = common::tempdir();
    let path = instructions::path("codex", home.path(), unset).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    // 9 + 18 × 200 + 1 bytes.
    let text = format!("# Rules\n\n{}\n", "Reply in Chinese. ".repeat(200));
    fs::write(&path, &text).unwrap();
    let file = instructions::read("codex", path.clone());
    assert!(file.exists());
    assert_eq!(file.target, path);
    let State::Text {
        text: read,
        size,
        modified,
    } = &file.state
    else {
        panic!("{:?}", file.state);
    };
    assert_eq!(read, &text);
    assert_eq!(*size, 3610);
    assert_eq!(instructions::size_label(*size), "3.6 KB");
    assert!(modified.is_some());

    // There, and empty.
    let path = instructions::path("agy", home.path(), unset).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "").unwrap();
    let file = instructions::read("agy", path);
    assert!(matches!(&file.state, State::Text { text, size: 0, .. } if text.is_empty()));
    assert_eq!(instructions::size_label(0), "Empty");
    assert_eq!(instructions::size_label(512), "512 bytes");
    assert_eq!(instructions::size_label(1_340_000), "1.3 MB");

    // There, but not text: said so, not shown.
    let path = instructions::path("amp", home.path(), unset).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
    let file = instructions::read("amp", path);
    assert!(file.exists());
    assert_eq!(file.state, State::Unreadable("it isn't UTF-8 text".into()));
}

#[test]
fn a_linked_file_is_read_where_the_link_leads() {
    let home = common::tempdir();
    let dotfiles = home.path().join("dotfiles");
    fs::create_dir_all(&dotfiles).unwrap();
    let real = dotfiles.join("CLAUDE.md");
    fs::write(&real, "# Linked\n").unwrap();

    // A link relative to its folder.
    let path = instructions::path("claude", home.path(), unset).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink("../dotfiles/CLAUDE.md", &path).unwrap();
    let file = instructions::read("claude", path.clone());
    assert_eq!(file.path, path);
    assert_eq!(fs::canonicalize(&file.target).unwrap(), real);
    assert!(fs::symlink_metadata(&file.target).unwrap().is_file());
    assert!(matches!(&file.state, State::Text { text, size: 9, .. } if text == "# Linked\n"));

    // A link to a link, both absolute: the file at the end.
    let between = home.path().join("between.md");
    symlink(&real, &between).unwrap();
    let path = instructions::path("codex", home.path(), unset).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink(&between, &path).unwrap();
    assert_eq!(instructions::read("codex", path).target, real);

    // A link to a file not made yet: missing, where it would be.
    let path = instructions::path("agy", home.path(), unset).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink(dotfiles.join("GEMINI.md"), &path).unwrap();
    let file = instructions::read("agy", path);
    assert_eq!(file.state, State::Missing);
    assert_eq!(file.target, dotfiles.join("GEMINI.md"));
}
