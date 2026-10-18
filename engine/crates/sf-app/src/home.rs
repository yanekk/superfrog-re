//! The app-data folder and the asset cache in it (DESIGN §2.2, §3.5). The launch decision and the
//! picker are `import.rs`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use sf_assets::{CACHE_FORMAT, GameData};
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
}
