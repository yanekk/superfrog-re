//! Test-side locator for the owner's four Superfrog disk images (DESIGN §5, "Disk images").
//!
//! Looks in `SUPERFROG_DISKS` first, then in the main checkout (the parent of the git common
//! dir, so it works from any worktree). A test that needs the disks and cannot find them fails
//! with the error from here rather than skipping.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The folder holding the four disk images.
pub fn disk_dir() -> Result<PathBuf, String> {
    locate(std::env::var_os("SUPERFROG_DISKS"), main_checkout())
}

/// `disk_dir` with its two inputs passed in, so tests need not touch the process environment.
pub fn locate(env: Option<OsString>, checkout: Result<PathBuf, String>) -> Result<PathBuf, String> {
    let mut looked = Vec::new();
    match env.filter(|v| !v.is_empty()) {
        Some(v) => {
            let dir = PathBuf::from(v);
            match missing_disks(&dir) {
                None => return Ok(dir),
                Some(why) => looked.push(format!("SUPERFROG_DISKS={}: {why}", dir.display())),
            }
        }
        None => looked.push("SUPERFROG_DISKS: not set".to_string()),
    }
    match checkout {
        Ok(dir) => match missing_disks(&dir) {
            None => return Ok(dir),
            Some(why) => looked.push(format!("main checkout {}: {why}", dir.display())),
        },
        Err(e) => looked.push(format!("main checkout: {e}")),
    }
    Err(format!(
        "Superfrog disk images not found. Looked in:\n  - {}\nPut the four `Superfrog (1993)(Team 17)(Disk N of 4)...` \
         .zip or .adf files in the main checkout root, or point SUPERFROG_DISKS at their folder.",
        looked.join("\n  - ")
    ))
}

/// `<git common dir>/..`: the main checkout, whichever worktree this runs in.
pub fn main_checkout() -> Result<PathBuf, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let common = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    common
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("git common dir {} has no parent", common.display()))
}

/// `None` if `dir` holds all four disks (zipped or raw), else what is wrong with it.
fn missing_disks(dir: &Path) -> Option<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => return Some(format!("cannot read it ({e})")),
    };
    let names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_lowercase())
        .filter(|n| n.ends_with(".zip") || n.ends_with(".adf"))
        .collect();
    let missing: Vec<String> = (1..=4)
        .filter(|n| !names.iter().any(|f| f.contains(&format!("disk {n} of 4"))))
        .map(|n| n.to_string())
        .collect();
    if missing.is_empty() {
        None
    } else {
        Some(format!("no disk {} of 4", missing.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_disks(tag: &str, which: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sf-disks-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for n in which {
            std::fs::write(
                dir.join(format!(
                    "Superfrog (1993)(Team 17)(Disk {n} of 4)[cr CSL].zip"
                )),
                b"",
            )
            .unwrap();
        }
        dir
    }

    #[test]
    fn honours_superfrog_disks() {
        let env = fake_disks("env", &[1, 2, 3, 4]);
        let other = fake_disks("other", &[1, 2, 3, 4]);
        assert_eq!(locate(Some(env.clone().into()), Ok(other)), Ok(env));
    }

    #[test]
    fn falls_back_to_main_checkout() {
        let checkout = fake_disks("checkout", &[1, 2, 3, 4]);
        assert_eq!(locate(None, Ok(checkout.clone())), Ok(checkout.clone()));
        let incomplete = fake_disks("incomplete", &[1, 3]);
        assert_eq!(
            locate(Some(incomplete.into()), Ok(checkout.clone())),
            Ok(checkout)
        );
    }

    #[test]
    fn errs_naming_both_places() {
        let a = fake_disks("err-a", &[1, 3]);
        let b = fake_disks("err-b", &[]);
        let e = locate(Some(a.clone().into()), Ok(b.clone())).unwrap_err();
        assert!(
            e.contains(&format!(
                "SUPERFROG_DISKS={}: no disk 2, 4 of 4",
                a.display()
            )),
            "{e}"
        );
        assert!(
            e.contains(&format!(
                "main checkout {}: no disk 1, 2, 3, 4 of 4",
                b.display()
            )),
            "{e}"
        );
        let e = locate(None, Err("git rev-parse failed: not a repo".into())).unwrap_err();
        assert!(
            e.contains("SUPERFROG_DISKS: not set")
                && e.contains("main checkout: git rev-parse failed"),
            "{e}"
        );
    }

    #[test]
    fn finds_the_real_disks() {
        let dir = disk_dir().unwrap_or_else(|e| panic!("{e}"));
        assert!(missing_disks(&dir).is_none());
    }
}
