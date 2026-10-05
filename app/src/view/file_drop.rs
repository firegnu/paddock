//! Shell text for external file drops; no file contents are read.
use std::{fmt::Write, path::PathBuf};

pub(super) fn text(paths: &[PathBuf]) -> String {
    let mut text = String::new();
    for path in paths {
        let path = path.to_string_lossy();
        if path.chars().any(char::is_control) {
            // ANSI-C quoting (zsh/bash): never send a literal Enter, Tab, or escape
            // sequence, even when the receiving terminal has bracketed paste off.
            text.push_str("$'");
            for ch in path.chars() {
                match ch {
                    '\'' => text.push_str("\\'"),
                    '\\' => text.push_str("\\\\"),
                    ch if ch.is_control() => {
                        for byte in ch.encode_utf8(&mut [0; 4]).bytes() {
                            write!(text, "\\{byte:03o}").unwrap();
                        }
                    }
                    ch => text.push(ch),
                }
            }
        } else {
            text.push('\'');
            text.push_str(&path.replace('\'', "'\\''"));
        }
        text.push_str("' ");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::text;
    use std::path::PathBuf;

    #[test]
    fn ordinary_file_is_quoted_and_followed_by_a_space_without_enter() {
        assert_eq!(
            text(&[PathBuf::from("/tmp/image.png")]),
            "'/tmp/image.png' "
        );
    }

    #[test]
    fn spaces_quotes_dollars_backslashes_and_chinese_stay_literal() {
        let paths = [
            "/tmp/a b.png",
            "/tmp/a'b\"c.png",
            "/tmp/$x\\y.png",
            "/tmp/图片.png",
        ];
        let paths: Vec<_> = paths.into_iter().map(PathBuf::from).collect();
        let pasted = text(&paths);
        assert_eq!(
            pasted,
            "'/tmp/a b.png' '/tmp/a'\\''b\"c.png' '/tmp/$x\\y.png' '/tmp/图片.png' "
        );
        assert_eq!(
            shell_words::split(&pasted).unwrap(),
            [
                "/tmp/a b.png",
                "/tmp/a'b\"c.png",
                "/tmp/$x\\y.png",
                "/tmp/图片.png"
            ]
        );
    }

    #[test]
    fn control_characters_are_shell_escapes_and_never_terminal_input() {
        let path = "/tmp/图'\"$\\\n\r\t\x03\x1b[201~\x7f\u{85}7.png";
        let pasted = text(&[PathBuf::from(path)]);
        assert_eq!(
            pasted,
            "$'/tmp/图\\'\"$\\\\\\012\\015\\011\\003\\033[201~\\177\\302\\2057.png' "
        );
        assert!(!pasted.chars().any(char::is_control));
        // Let the local shells interpret only the generated argument text. The resulting
        // argument must be byte-for-byte the original filename, including its controls.
        for shell in ["/bin/zsh", "/bin/bash"] {
            let output = std::process::Command::new(shell)
                .args(["-f", "-c", &format!("printf '%s' {pasted}")])
                .output()
                .unwrap();
            assert!(output.status.success(), "{shell}: {:?}", output.stderr);
            assert_eq!(output.stdout, path.as_bytes(), "{shell}");
        }
    }

    #[test]
    fn dropped_paths_use_existing_paste_bytes_in_both_terminal_modes() {
        let pasted = text(&[
            PathBuf::from("/tmp/图 a.png"),
            PathBuf::from("/tmp/a\n.png"),
        ]);
        assert_eq!(
            crate::input::encode_paste(&pasted, false),
            "'/tmp/图 a.png' $'/tmp/a\\012.png' ".as_bytes()
        );
        assert_eq!(
            crate::input::encode_paste(&pasted, true),
            "\x1b[200~'/tmp/图 a.png' $'/tmp/a\\012.png' \x1b[201~".as_bytes()
        );
    }
}
