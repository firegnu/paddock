//! Skill publication bound to opened directories. A path replacement must never
//! turn an owned-file update into an overwrite or deletion of somebody else's file.
use super::{BODY, MARKER};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::{CStr, CString},
    fs::{self, File, Metadata},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
    mode: u32,
    // Directory contents may change while retaining the same directory identity.
    contents: Option<(u64, i64, i64)>,
}
impl Identity {
    fn of(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            mode: m.mode(),
            contents: m.is_file().then(|| (m.len(), m.mtime(), m.mtime_nsec())),
        }
    }
}
pub(super) struct Snapshot(Vec<(PathBuf, Option<Identity>)>);
impl Snapshot {
    pub(super) fn capture(home: &Path, target: &Path) -> Result<Self> {
        let mut nodes = Vec::new();
        for path in target.ancestors() {
            nodes.push((path.to_owned(), identity(path)?));
            if path == home {
                break;
            }
        }
        Ok(Self(nodes))
    }
    fn expected(&self, path: &Path) -> Option<&Identity> {
        self.0
            .iter()
            .find(|(p, _)| p == path)
            .and_then(|(_, id)| id.as_ref())
    }
    fn check(&self) -> Result<()> {
        for (path, expected) in &self.0 {
            ensure!(
                identity(path)? == *expected,
                "skill file or directory changed: {}",
                path.display()
            );
        }
        Ok(())
    }
}
fn identity(path: &Path) -> Result<Option<Identity>> {
    match fs::symlink_metadata(path) {
        Ok(m) => Ok(Some(Identity::of(&m))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn open_at(fd: i32, name: &CStr, flags: i32) -> Result<File> {
    // O_NONBLOCK avoids waiting on a raced-in FIFO; the caller checks file type.
    let opened = unsafe {
        libc::openat(
            fd,
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600,
        )
    };
    ensure!(
        opened >= 0,
        "open skill object: {}",
        std::io::Error::last_os_error()
    );
    Ok(unsafe { File::from_raw_fd(opened) })
}
struct Directory(Vec<(PathBuf, File)>);
impl Directory {
    fn open(home: &Path, parent: &Path, snapshot: &Snapshot) -> Result<Self> {
        let name = CString::new(home.as_os_str().as_bytes())?;
        let root = open_at(libc::AT_FDCWD, &name, libc::O_RDONLY | libc::O_DIRECTORY)?;
        ensure!(
            snapshot.expected(home) == Some(&Identity::of(&root.metadata()?)),
            "skill home changed"
        );
        let mut result = Self(vec![(home.to_owned(), root)]);
        let mut path = home.to_owned();
        for part in parent.strip_prefix(home)?.components() {
            let name = CString::new(part.as_os_str().as_bytes())?;
            let fd = result.fd();
            path.push(part);
            let expected = snapshot.expected(&path);
            if expected.is_none() {
                ensure!(
                    unsafe { libc::mkdirat(fd, name.as_ptr(), 0o700) } == 0,
                    "skill directory appeared or could not be created: {}",
                    std::io::Error::last_os_error()
                );
            }
            let directory = open_at(fd, &name, libc::O_RDONLY | libc::O_DIRECTORY)?;
            if let Some(expected) = expected {
                ensure!(
                    *expected == Identity::of(&directory.metadata()?),
                    "skill directory identity changed: {}",
                    path.display()
                );
            }
            result.0.push((path.clone(), directory));
        }
        result.check()?;
        Ok(result)
    }
    fn fd(&self) -> i32 {
        self.0.last().unwrap().1.as_raw_fd()
    }
    fn check(&self) -> Result<()> {
        for (path, file) in &self.0 {
            ensure!(
                identity(path)? == Some(Identity::of(&file.metadata()?)),
                "skill directory changed: {}",
                path.display()
            );
        }
        Ok(())
    }
    fn rename_exclusive(&self, from: &CStr, to: &CStr) -> Result<()> {
        let result = unsafe {
            libc::renameatx_np(
                self.fd(),
                from.as_ptr(),
                self.fd(),
                to.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        ensure!(
            result == 0,
            "skill target appeared or changed; preserved existing files: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }
    fn unlink(&self, name: &CStr) -> Result<()> {
        ensure!(
            unsafe { libc::unlinkat(self.fd(), name.as_ptr(), 0) } == 0,
            "remove staged skill: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }
}

pub(super) fn apply(
    home: &Path,
    target: &Path,
    snapshot: &Snapshot,
    remove: bool,
    before_publish: &mut impl FnMut(&Path) -> Result<()>,
) -> Result<()> {
    snapshot.check()?;
    let directory = Directory::open(home, target.parent().unwrap(), snapshot)?;
    let expected = snapshot.0[0].1.as_ref();
    let name = c"SKILL.md";
    let check_owned = |name: &CStr| -> Result<()> {
        let mut file = open_at(directory.fd(), name, libc::O_RDONLY)?;
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file() && Some(&Identity::of(&metadata)) == expected,
            "skill file identity changed"
        );
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        ensure!(contents.contains(MARKER), "skill file is no longer owned");
        Ok(())
    };
    if expected.is_some() {
        check_owned(name)?;
    }
    let temp = CString::new(format!(".paddock-skill-{}", crate::control::random_id()?))?;
    if !remove {
        let mut file = open_at(
            directory.fd(),
            &temp,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        file.write_all(BODY.as_bytes())?;
        file.sync_all()?;
    }
    let result = (|| {
        directory.check()?;
        before_publish(target)?;
        // Operations below stay relative to the opened directory even if a
        // pathname is replaced after this check.
        directory.check()?;
        if expected.is_none() {
            directory.rename_exclusive(&temp, name)?;
            return directory.check();
        }
        let saved = CString::new(format!(
            ".paddock-skill-saved-{}",
            crate::control::random_id()?
        ))?;
        directory.rename_exclusive(name, &saved)?;
        // Moving is reversible; validate the *moved* object before deleting it.
        // This catches a replacement after the final pathname check as well.
        let commit = (|| {
            directory.check()?;
            check_owned(&saved)?;
            if !remove {
                directory.rename_exclusive(&temp, name)?;
            }
            Ok(())
        })();
        if let Err(error) = commit {
            if let Err(restore) = directory.rename_exclusive(&saved, name) {
                return Err(error).context(format!(
                    "{restore}; preserved displaced file as {}/{}",
                    target.parent().unwrap().display(),
                    saved.to_string_lossy()
                ));
            }
            return Err(error);
        }
        directory.unlink(&saved)?;
        directory.check()
    })();
    if !remove {
        let _ = directory.unlink(&temp);
    }
    result
}
