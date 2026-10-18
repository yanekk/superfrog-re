//! Every file the app writes, written so a reader sees the old file or the new one and never half
//! of one (DESIGN §2.11, §3.5): a temporary name in the same folder, flushed, then renamed over.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Write `bytes` to `path` through a temporary file in the same folder: written, fsynced, then
/// renamed over `path`. The folder is made if it is missing.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_then(path, bytes, |_| Ok(()))
}

/// The temporary name `path` is written under before the rename.
fn temporary(path: &Path) -> PathBuf {
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    dir.join(format!(".{name}.tmp-{}", std::process::id()))
}

/// [`atomic_write`], calling `before_rename` with the temporary file once it is on disk: an `Err`
/// from it stands for the write being cut off there (a crash, a full disk), and nothing is renamed.
fn write_then(
    path: &Path,
    bytes: &[u8],
    before_rename: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let tmp = temporary(path);
    let written = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        before_rename(&tmp)?;
        std::fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sf-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_write_replaces_the_file_and_leaves_no_temporary() {
        let d = dir("atomic");
        let f = d.join("sub/x.bin");
        atomic_write(&f, b"one").unwrap();
        atomic_write(&f, b"two").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"two");
        assert_eq!(std::fs::read_dir(d.join("sub")).unwrap().count(), 1);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_write_cut_off_between_writing_and_renaming_leaves_the_old_file_intact() {
        let d = dir("atomic-cut");
        let f = d.join("settings.toml");
        atomic_write(&f, b"old").unwrap();
        let mut seen = None;
        let e = write_then(&f, b"new and longer", |tmp| {
            // The new bytes are wholly on disk under the temporary name, the old file untouched.
            seen = Some((std::fs::read(tmp).unwrap(), std::fs::read(&f).unwrap()));
            Err(io::Error::other("power lost"))
        })
        .unwrap_err();
        assert_eq!(e.to_string(), "power lost");
        assert_eq!(seen, Some((b"new and longer".to_vec(), b"old".to_vec())));
        assert_eq!(std::fs::read(&f).unwrap(), b"old");
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1, "the temporary is gone");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_crash_s_leftover_temporary_does_not_stop_the_next_write() {
        let d = dir("atomic-left");
        let f = d.join("progress.toml");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(temporary(&f), b"half").unwrap();
        atomic_write(&f, b"whole").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"whole");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
