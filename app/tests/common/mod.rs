//! Helpers for the tests taken from Saddle (`tests/common/mod.rs` at commit `df1c727`), plus a
//! temporary directory so the tests need no extra crate.
#![allow(dead_code)]
use std::{
    fs,
    io::ErrorKind,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

pub fn script(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path.to_str().unwrap().into()
}

/// A fresh directory under the system temp directory, removed when dropped.
pub struct TempDir(PathBuf);
impl TempDir {
    pub fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub fn tempdir() -> TempDir {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "paddock-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    // Canonical, so scripts that `cd` next to themselves agree with the test about paths.
    TempDir(path.canonicalize().unwrap())
}

/// Like [`tempdir`], short enough for a control socket's path (see [`short_dir`]).
pub fn short_tempdir() -> TempDir {
    let path = short_dir("pdt", 0o755);
    TempDir(path.canonicalize().unwrap())
}

/// Makes a fresh directory `<prefix>-<random>` with `mode`, short enough for a control socket's
/// path: under `/tmp`, or under `TMPDIR` when a sandbox refuses `/tmp` (fence gives agents their
/// own `TMPDIR`). macOS socket paths hold under 104 bytes, and the tests put at most
/// `/paddock-ctl/<16 hex>.sock` below this directory.
pub fn short_dir(prefix: &str, mode: u32) -> PathBuf {
    let name = format!("{prefix}-{}", paddock::control::random_id().unwrap());
    let make = |base: PathBuf| {
        let path = base.join(&name);
        fs::DirBuilder::new()
            .mode(mode)
            .create(&path)
            .map(|()| path)
    };
    let path = match make("/tmp".into()) {
        Err(error) if error.kind() == ErrorKind::PermissionDenied => make(std::env::temp_dir()),
        made => made,
    }
    .unwrap_or_else(|error| panic!("cannot make {name} under /tmp or TMPDIR: {error}"));
    let length = path.canonicalize().unwrap().as_os_str().len();
    assert!(
        length + "/paddock-ctl/0123456789abcdef.sock".len() < 104,
        "TMPDIR too long for a control socket: {}",
        path.display()
    );
    path
}
