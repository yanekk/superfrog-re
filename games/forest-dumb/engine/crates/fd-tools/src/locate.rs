//! Where the owner's two zips are (`docs/01-disk-location.md`). A test that needs them and cannot
//! find them fails with the error from here rather than skipping.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The folder holding the two zips, from the process's environment and git.
pub fn disk_dir() -> Result<PathBuf, String> {
    let env = std::env::var("FOREST_DUMB_DISKS").ok();
    locate(env.as_deref(), &main_checkout()?)
}

/// Where the two zips live: `FOREST_DUMB_DISKS`, else `games/forest-dumb/` in the main checkout,
/// else its root. A place without both zips is passed over; finding none names every place tried.
pub fn locate(env: Option<&str>, checkout: &Path) -> Result<PathBuf, String> {
    let mut looked = Vec::new();
    match env.filter(|v| !v.is_empty()) {
        Some(v) => match missing(Path::new(v)) {
            None => return Ok(PathBuf::from(v)),
            Some(why) => looked.push(format!("FOREST_DUMB_DISKS={v}: {why}")),
        },
        None => looked.push("FOREST_DUMB_DISKS: not set".to_string()),
    }
    for dir in [checkout.join("games/forest-dumb"), checkout.to_path_buf()] {
        match missing(&dir) {
            None => return Ok(dir),
            Some(why) => looked.push(format!("main checkout {}: {why}", dir.display())),
        }
    }
    Err(format!(
        "Forest Dumb Forever's zips not found. Looked in:\n  - {}\nPut the two \
         `Forest Dumb Forever!!! ...(Disk N of 2)[cr WT].zip` files in games/forest-dumb/ in the \
         main checkout (its root also works), or point FOREST_DUMB_DISKS at their folder.",
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

/// The paths in `dir` of the zip of each disk, by the `disk N of 2` their names carry.
pub fn zip_paths(dir: &Path) -> Result<[PathBuf; 2], String> {
    let names = zip_names(dir).map_err(|e| format!("{}: cannot read it ({e})", dir.display()))?;
    let one = |n: u8| -> Result<PathBuf, String> {
        let hits: Vec<&String> = names
            .iter()
            .filter(|f| f.to_lowercase().contains(&format!("disk {n} of 2")))
            .collect();
        match hits[..] {
            [only] => Ok(dir.join(only)),
            [] => Err(format!("{}: no zip of disk {n} of 2", dir.display())),
            // Picking one would make the inventory depend on the folder's listing order.
            _ => Err(format!(
                "{}: {} zips of disk {n} of 2: {hits:?}",
                dir.display(),
                hits.len()
            )),
        }
    };
    Ok([one(1)?, one(2)?])
}

fn zip_names(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let name = e?.file_name().to_string_lossy().into_owned();
        if name.to_lowercase().ends_with(".zip") {
            names.push(name);
        }
    }
    Ok(names)
}

/// `None` if `dir` holds exactly one zip of each disk, else what is wrong with it.
fn missing(dir: &Path) -> Option<String> {
    zip_paths(dir).err().map(|e| {
        // The path is already in the caller's line.
        e.strip_prefix(&format!("{}: ", dir.display()))
            .map(str::to_string)
            .unwrap_or(e)
    })
}
