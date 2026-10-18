//! The single-instance lock (DESIGN §2.11): `lock` in the app-data folder, holding the PID of the
//! copy running on it. Two copies would race each other's writes to progress and settings, so the
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

pub fn path(home: &Path) -> PathBuf {
    home.join("lock")
}

/// The lock, held while this lives.
#[derive(Debug)]
pub struct Lock {
    _file: File,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LockError {
    /// Another copy holds it: its PID, when the file names one.
    Held(Option<u32>),
    Io(String),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Held(Some(pid)) => write!(
                f,
                "Superfrog is already running (process {pid}). Only one copy can run at a time."
            ),
            LockError::Held(None) => {
                write!(f, "Superfrog is already running. Only one copy can run at a time.")
            }
            LockError::Io(e) => write!(f, "The app's lock file could not be opened: {e}"),
        }
    }
}

impl Lock {
    pub fn acquire(home: &Path) -> Result<Lock, LockError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sf-app-lock-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_second_lock_is_refused_with_the_holder_s_pid_until_the_first_is_dropped() {
        let h = home("two");
        let first = Lock::acquire(&h).unwrap();
        assert_eq!(
            std::fs::read_to_string(path(&h)).unwrap(),
            format!("{}\n", std::process::id())
        );
        // flock locks belong to the open file, so a second open in this process is refused too.
        let e = Lock::acquire(&h).unwrap_err();
        assert_eq!(e, LockError::Held(Some(std::process::id())));
        assert!(e.to_string().contains("already running"), "{e}");
        drop(first);
        Lock::acquire(&h).unwrap();
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn a_stale_lock_naming_a_dead_pid_is_taken_over() {
        let h = home("stale");
        std::fs::create_dir_all(&h).unwrap();
        std::fs::write(path(&h), "999999\n").unwrap();
        let _l = Lock::acquire(&h).unwrap();
        assert_eq!(
            std::fs::read_to_string(path(&h)).unwrap(),
            format!("{}\n", std::process::id())
        );
        std::fs::remove_dir_all(&h).unwrap();
    }
}
