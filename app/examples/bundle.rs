//! Packs the release `paddock` into `paddock.app`, next to it in the build directory, and with
//! `--install` copies it to `~/Applications`. Unsigned and not notarised (DESIGN §7).
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
