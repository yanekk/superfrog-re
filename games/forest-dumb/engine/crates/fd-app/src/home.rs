//! The app-data folder (DESIGN §2.7): `--home`, then `FOREST_DUMB_HOME`, then
//! `~/Library/Application Support/Forest-Dumb-RE`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn resolve(arg: Option<&Path>, env: Option<OsString>, user_home: Option<OsString>) -> PathBuf {
    if let Some(a) = arg {
        return a.to_path_buf();
    }
    if let Some(e) = env.filter(|e| !e.is_empty()) {
        return PathBuf::from(e);
    }
    PathBuf::from(user_home.unwrap_or_else(|| "/".into()))
        .join("Library/Application Support/Forest-Dumb-RE")
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
        assert_eq!(
            resolve(None, Some("/b".into()), user.clone()),
            PathBuf::from("/b")
        );
        assert_eq!(
            resolve(None, Some("".into()), user),
            PathBuf::from("/Users/u/Library/Application Support/Forest-Dumb-RE")
        );
    }
}
