//! Packs the release `paddock` into `paddock.app`, next to it in the build directory, and with
//! `--install` copies it to `~/Applications`. Signs and verifies the whole bundle with
//! PADDOCK_SIGN_IDENTITY, a local Apple Development identity, or ad-hoc as a fallback.
//! Not notarised (DESIGN §13, P5-23).
//!
//! ```sh
//! cargo build --release && cargo run --release --example bundle -- --install
//! ```
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const IDENTIFIER: &str = "dev.paddock.app";

fn apple_development_identity(output: &str) -> Option<&str> {
    output.lines().find_map(|line| {
        let (prefix, name) = line.split_once('"')?;
        let (name, suffix) = name.rsplit_once('"')?;
        if !name.starts_with("Apple Development:") || !suffix.trim().is_empty() {
            return None;
        }
        let mut fields = prefix.split_whitespace();
        fields.next()?.strip_suffix(')')?.parse::<usize>().ok()?;
        let hash = fields.next()?;
        (hash.len() == 40 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && fields.next().is_none())
            .then_some(hash)
    })
}

fn sign_bundle(app: &Path) -> Result<()> {
    let identity = if let Some(identity) =
        std::env::var_os("PADDOCK_SIGN_IDENTITY").filter(|value| !value.is_empty())
    {
        if identity != "-" {
            println!("signing with PADDOCK_SIGN_IDENTITY");
        }
        identity
    } else {
        let output = Command::new("/usr/bin/security")
            .args(["find-identity", "-v", "-p", "codesigning"])
            .output()
            .context("finding code signing identities")?;
        ensure!(
            output.status.success(),
            "security find-identity failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        match apple_development_identity(&String::from_utf8_lossy(&output.stdout)) {
            Some(hash) => {
                println!("signing with Apple Development identity");
                hash.into()
            }
            None => "-".into(),
        }
    };
    if identity == "-" {
        eprintln!("signing ad-hoc: 屏幕录制等授权每次重新安装后会失效。");
    }
    let status = Command::new("/usr/bin/codesign")
        .args(["--force", "--sign"])
        .arg(identity)
        .args(["--identifier", IDENTIFIER, "--timestamp=none"])
        .arg(app)
        .status()
        .context("signing paddock.app")?;
    ensure!(
        status.success(),
        "codesign signing failed ({status}); see diagnostics above"
    );
    let status = Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict", "--verbose=2"])
        .arg(app)
        .status()
        .context("verifying paddock.app signature")?;
    ensure!(
        status.success(),
        "codesign verification failed ({status}); see diagnostics above"
    );
    Ok(())
}

fn main() -> Result<()> {
    let install = std::env::args().skip(1).any(|arg| arg == "--install");
    // This example runs from `<target>/release/examples/`; paddock is in `<target>/release/`.
    let exe = std::env::current_exe()?;
    let release = exe
        .parent()
        .and_then(Path::parent)
        .context("where the build directory is")?;
    let binary = release.join("paddock");
    ensure!(
        binary.exists(),
        "{} is missing: run `cargo build --release` first",
        binary.display()
    );

    let app = release.join("paddock.app");
    if app.exists() {
        fs::remove_dir_all(&app)?;
    }
    let contents = app.join("Contents");
    fs::create_dir_all(contents.join("MacOS"))?;
    fs::create_dir_all(contents.join("Resources"))?;
    fs::copy(&binary, contents.join("MacOS/paddock"))?;
    fs::write(contents.join("Info.plist"), info_plist())?;
    write_icon(release, &contents.join("Resources/AppIcon.icns"))?;
    sign_bundle(&app)?;
    println!("built {}", app.display());

    if install {
        let home = std::env::var("HOME").context("HOME")?;
        let target = PathBuf::from(home).join("Applications/paddock.app");
        if target.exists() {
            // Only ever replace an earlier paddock.app, never something else of that name.
            let plist = fs::read_to_string(target.join("Contents/Info.plist")).unwrap_or_default();
            if !plist.contains(IDENTIFIER) {
                bail!(
                    "{} exists and is not paddock; leaving it alone",
                    target.display()
                );
            }
            fs::remove_dir_all(&target)?;
        }
        fs::create_dir_all(target.parent().unwrap())?;
        copy_dir(&app, &target)?;
        println!("installed {}", target.display());
    }
    Ok(())
}

fn info_plist() -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>paddock</string>
    <key>CFBundleDisplayName</key><string>paddock</string>
    <key>CFBundleIdentifier</key><string>{IDENTIFIER}</string>
    <key>CFBundleExecutable</key><string>paddock</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>{version}</string>
    <key>CFBundleVersion</key><string>{version}</string>
    <key>LSMinimumSystemVersion</key><string>13.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#
    )
}

/// Every size macOS asks of an icon, drawn directly rather than resampled, then joined into an
/// `.icns` by the system's `iconutil`.
fn write_icon(release: &Path, icns: &Path) -> Result<()> {
    let set = release.join("AppIcon.iconset");
    if set.exists() {
        fs::remove_dir_all(&set)?;
    }
    fs::create_dir_all(&set)?;
    for base in [16, 32, 128, 256, 512] {
        for (scale, suffix) in [(1, ""), (2, "@2x")] {
            let size = base * scale;
            let file = fs::File::create(set.join(format!("icon_{base}x{base}{suffix}.png")))?;
            let mut encoder = png::Encoder::new(file, size, size);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()?
                .write_image_data(&paddock::icon::icon(size))?;
        }
    }
    let status = Command::new("/usr/bin/iconutil")
        .arg("-c")
        .arg("icns")
        .arg(&set)
        .arg("-o")
        .arg(icns)
        .status()
        .context("running iconutil")?;
    ensure!(status.success(), "iconutil failed");
    fs::remove_dir_all(&set)?;
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_first_apple_development_identity() {
        // Generated synthetic values, never identities from the local keychain.
        let other = "0".repeat(40);
        let first = "1".repeat(40);
        let second = "2".repeat(40);
        let output = format!(
            "  1) {other} \"Apple Distribution: <placeholder>\"\n\
               2) {first} \"Apple Development: <placeholder>\"\n\
               3) {second} \"Apple Development: <placeholder>\"\n\
               3 valid identities found\n"
        );
        assert_eq!(apple_development_identity(&output), Some(first.as_str()));
    }

    #[test]
    fn ignores_invalid_identities() {
        let invalid = "3".repeat(40);
        let valid = "4".repeat(40);
        let output = format!(
            "  1) {invalid} \"Apple Development: <placeholder>\" (CSSMERR_TP_CERT_EXPIRED)\n\
               2) {valid} \"Apple Development: <placeholder>\"\n"
        );
        assert_eq!(apple_development_identity(&output), Some(valid.as_str()));
        assert_eq!(
            apple_development_identity(output.lines().next().unwrap()),
            None
        );
    }

    #[test]
    fn no_development_identity() {
        let hash = "5".repeat(40);
        for output in [
            String::new(),
            "  0 valid identities found\n".into(),
            format!(
                "  1) {hash} \"Apple Distribution: <placeholder>\"\n  1 valid identities found\n"
            ),
        ] {
            assert_eq!(apple_development_identity(&output), None);
        }
    }
}
