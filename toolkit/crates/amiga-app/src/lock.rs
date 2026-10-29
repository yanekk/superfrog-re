//! The single-instance lock: `lock` in the app-data folder, holding the PID of the copy running on
//! it. Two copies would race each other's writes to progress and settings, so the
//! second one shows a message and quits.
//!
//! The file is held with `flock`, which the system lets go of when the holder exits however it
//! exits, so a stale lock (its PID dead, after a crash or `kill -9`) is free and the next launch
//! takes it over. The PID in it is for the message. The file is written in place, not through
//! `atomic_write`: a rename would put a new, unlocked file in its place.

use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

pub fn path(dir: &Path) -> PathBuf {
    dir.join("lock")
}

/// The lock, held while this lives.
#[derive(Debug)]
pub struct Lock {
    _file: File,
}

#[derive(Debug, PartialEq, Eq)]
enum LockError {
    /// Another copy holds it: its PID, when the file names one.
    Held(Option<u32>),
    Io(String),
}

impl LockError {
    /// What the person is told, naming the app.
    fn message(&self, app_name: &str) -> String {
        match self {
            LockError::Held(Some(pid)) => {
                format!("{app_name} is already running (process {pid}). Only one copy can run at a time.")
            }
            LockError::Held(None) => format!("{app_name} is already running. Only one copy can run at a time."),
            LockError::Io(e) => format!("The app's lock file could not be opened: {e}"),
        }
    }
}

/// Take the lock in `dir`; `Err` is the message to show, naming `app_name`.
pub fn acquire(dir: &Path, app_name: &str) -> Result<Lock, String> {
    try_acquire(dir).map_err(|e| e.message(app_name))
}

fn try_acquire(home: &Path) -> Result<Lock, LockError> {
    let io = |e: std::io::Error| LockError::Io(e.to_string());
    std::fs::create_dir_all(home).map_err(io)?;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path(home))
        .map_err(io)?;
    // SAFETY: flock on a descriptor this function owns.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EWOULDBLOCK) {
            return Err(io(err));
        }
        let mut text = String::new();
        let _ = file.read_to_string(&mut text);
        return Err(LockError::Held(text.trim().parse().ok()));
    }
    file.set_len(0).map_err(io)?;
    file.rewind().map_err(io)?;
    writeln!(file, "{}", std::process::id()).map_err(io)?;
    file.sync_all().map_err(io)?;
    Ok(Lock { _file: file })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("amiga-app-lock-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_second_lock_is_refused_with_the_holder_s_pid_until_the_first_is_dropped() {
        let h = home("two");
        let first = try_acquire(&h).unwrap();
        assert_eq!(
            std::fs::read_to_string(path(&h)).unwrap(),
            format!("{}\n", std::process::id())
        );
        // flock locks belong to the open file, so a second open in this process is refused too.
        let e = try_acquire(&h).unwrap_err();
        assert_eq!(e, LockError::Held(Some(std::process::id())));
        let m = acquire(&h, "Some Game").unwrap_err();
        let pid = std::process::id();
        assert_eq!(m, format!("Some Game is already running (process {pid}). Only one copy can run at a time."));
        drop(first);
        acquire(&h, "Some Game").unwrap();
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn a_stale_lock_naming_a_dead_pid_is_taken_over() {
        let h = home("stale");
        std::fs::create_dir_all(&h).unwrap();
        std::fs::write(path(&h), "999999\n").unwrap();
        let _l = acquire(&h, "Some Game").unwrap();
        assert_eq!(
            std::fs::read_to_string(path(&h)).unwrap(),
            format!("{}\n", std::process::id())
        );
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn the_message_names_the_app() {
        assert_eq!(
            LockError::Held(None).message("Some Game"),
            "Some Game is already running. Only one copy can run at a time."
        );
    }
}
