//! The app-data folder and the asset cache in it (DESIGN §2.2, §3.5).
//!
//! T22 only loads the cache, importing it from a folder of disk images named on the command line
//! when it is missing or unusable; the disk picker and remembered disks are T23's.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use sf_assets::{CACHE_FORMAT, GameData, LaunchFacts, cache_decode, cache_encode, decide};
use sf_disk::Offered;

/// The app-data folder: `--home`, else `SUPERFROG_HOME`, else
/// `~/Library/Application Support/Superfrog-RE`.
pub fn resolve(arg: Option<&Path>, env: Option<OsString>, user_home: Option<OsString>) -> PathBuf {
    if let Some(a) = arg {
        return a.to_path_buf();
    }
    if let Some(e) = env.filter(|e| !e.is_empty()) {
        return PathBuf::from(e);
    }
    real_home(user_home)
}

/// The real app-data folder, under the user's home.
pub fn real_home(user_home: Option<OsString>) -> PathBuf {
    PathBuf::from(user_home.unwrap_or_else(|| "/".into()))
        .join("Library/Application Support/Superfrog-RE")
}

pub fn cache_path(home: &Path) -> PathBuf {
    home.join(format!("cache/assets-v{CACHE_FORMAT}.bin"))
}

/// Write `bytes` to `path` so a reader sees the old file or the new one, never half of one
/// (DESIGN §2.11): a temporary name in the same folder, flushed, then renamed over.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// The game data from `home`'s cache; when it is missing or unusable and `disks` names a folder,
/// imported from the disk images there and cached. `Err` says why, for the player.
pub fn load(home: &Path, disks: Option<&Path>) -> Result<GameData, String> {
    let path = cache_path(home);
    let cache = match std::fs::read(&path) {
        Ok(bytes) => Some(cache_decode(&bytes)),
        Err(_) => None,
    };
    let cache_err = match cache {
        Some(Ok(data)) => return Ok(data),
        Some(Err(e)) => Some(Err(e)),
        None => None,
    };
    let Some(dir) = disks else {
        let facts = LaunchFacts {
            cache: cache_err,
            remembered_disks: None,
        };
        let why = match decide(&facts) {
            sf_assets::LaunchAction::AskForDisks { reason }
            | sf_assets::LaunchAction::Reimport { reason } => reason,
            sf_assets::LaunchAction::UseCache => unreachable!("there is no usable cache"),
        };
        return Err(format!(
            "{why}\n(For now, pass --cache-from-disks DIR naming the folder that holds them.)"
        ));
    };
    let data = import(dir)?;
    write_atomic(&path, &cache_encode(&data, build()))
        .map_err(|e| format!("could not write the game data to {}: {e}", path.display()))?;
    Ok(data)
}

/// The engine build that writes a cache.
pub fn build() -> &'static str {
    concat!("superfrog ", env!("CARGO_PKG_VERSION"))
}

/// Decode the four disk images in `dir`: every `.zip` and `.adf` there is offered, read-only.
pub fn import(dir: &Path) -> Result<GameData, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files = Vec::new();
    for e in entries {
        let path = e.map_err(|e| e.to_string())?.path();
        let lower = path.to_string_lossy().to_lowercase();
        if lower.ends_with(".zip") || lower.ends_with(".adf") {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            files.push(Offered { name, bytes });
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    let set = sf_disk::identify(&files).map_err(|errors| {
        let mut why = format!("the disk images in {} cannot be used:", dir.display());
        for e in errors {
            why.push_str(&format!("\n  {e}"));
        }
        why
    })?;
    let chunks = sf_assets::unpack(&set).map_err(|e| e.to_string())?;
    sf_assets::decode(&chunks).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_beats_the_variable_which_beats_the_real_folder() {
        let user = Some(OsString::from("/Users/u"));
        assert_eq!(
            resolve(Some(Path::new("/a")), Some("/b".into()), user.clone()),
            PathBuf::from("/a")
        );
        assert_eq!(resolve(None, Some("/b".into()), user.clone()), PathBuf::from("/b"));
        assert_eq!(
            resolve(None, Some("".into()), user.clone()),
            PathBuf::from("/Users/u/Library/Application Support/Superfrog-RE")
        );
    }

    #[test]
    fn the_cache_is_named_for_its_format() {
        assert_eq!(
            cache_path(Path::new("/h")),
            PathBuf::from(format!("/h/cache/assets-v{CACHE_FORMAT}.bin"))
        );
    }

    #[test]
    fn an_atomic_write_replaces_the_file_and_leaves_no_temporary() {
        let dir = std::env::temp_dir().join(format!("sf-app-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let f = dir.join("sub/x.bin");
        write_atomic(&f, b"one").unwrap();
        write_atomic(&f, b"two").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"two");
        let names: Vec<_> = std::fs::read_dir(dir.join("sub")).unwrap().collect();
        assert_eq!(names.len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_cache_and_no_disks_says_what_to_do() {
        let dir = std::env::temp_dir().join(format!("sf-app-nocache-{}", std::process::id()));
        let e = load(&dir, None).unwrap_err();
        assert!(e.contains("--cache-from-disks"), "{e}");
        assert!(!dir.exists(), "loading wrote into the home");
    }
}
