//! Save states (DESIGN §2.8, §3.5): Game → Save State ▸ 1–5 writes the whole game to
//! `states/slot-N.sfstate` in the app-data folder, Load State ▸ 1–5 reads it back. A state from
//! another build, or on other game data, is refused with a message and the game carries on
//! untouched.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_core::{DisplayData, Game, StateError};

use crate::atomic_write::atomic_write;

/// The slots the Game menu offers.
pub const SLOTS: u8 = 5;

/// Slot `slot`'s file.
pub fn path(home: &Path, slot: u8) -> PathBuf {
    home.join(format!("states/slot-{slot}.sfstate"))
}

/// Which slots hold a file, slot 1 first.
pub fn filled(home: &Path) -> [bool; SLOTS as usize] {
    std::array::from_fn(|i| path(home, i as u8 + 1).is_file())
}

/// Write `game` to slot `slot`, tagged with this build and `cache_hash`.
pub fn save(home: &Path, slot: u8, game: &Game, build: &str, cache_hash: &[u8; 32]) -> Result<(), String> {
    let p = path(home, slot);
    atomic_write(&p, &game.save(build, cache_hash))
        .map_err(|e| format!("Save state {slot} could not be written ({}): {e}", p.display()))
}

/// The game slot `slot` holds, or the message saying why it cannot be loaded.
pub fn load(
    home: &Path,
    slot: u8,
    display: Arc<DisplayData>,
    build: &str,
    cache_hash: &[u8; 32],
) -> Result<Game, String> {
    let p = path(home, slot);
    let bytes = match std::fs::read(&p) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("Save state {slot} is empty."));
        }
        Err(e) => return Err(format!("Save state {slot} could not be read ({}): {e}", p.display())),
    };
    Game::load_with(display, &bytes, build, cache_hash).map_err(|e| refusal(slot, &e, build))
}

/// Why slot `slot` was not loaded, for the player; `ours` is this build.
pub fn refusal(slot: u8, e: &StateError, ours: &str) -> String {
    let why = match e {
        StateError::NotAState => "is not a save state".to_string(),
        StateError::OtherBuild { theirs } => {
            format!("was made by another version of this game, {theirs}; this is {ours}")
        }
        StateError::OtherCache => "was made with other game data: the disks were imported again".to_string(),
        StateError::Corrupt => "is damaged".to_string(),
    };
    format!("Save state {slot} {why}. It was not loaded.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refusal_names_both_builds() {
        let m = refusal(3, &StateError::OtherBuild { theirs: "superfrog 0.1.0 (aaaa)".into() }, "superfrog 0.1.0 (bbbb)");
        assert_eq!(
            m,
            "Save state 3 was made by another version of this game, superfrog 0.1.0 (aaaa); \
             this is superfrog 0.1.0 (bbbb). It was not loaded."
        );
        assert_eq!(refusal(1, &StateError::Corrupt, "x"), "Save state 1 is damaged. It was not loaded.");
    }

    #[test]
    fn slots_are_files_in_the_states_folder() {
        let home = std::env::temp_dir().join(format!("sf-states-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        assert_eq!(path(&home, 2), home.join("states/slot-2.sfstate"));
        assert_eq!(filled(&home), [false; 5]);
        let game = Game::new(&sf_core::GameData::default(), sf_core::Start::PowerOn { seed: sf_core::Rng::default() });
        save(&home, 2, &game, "b", &[1; 32]).unwrap();
        assert_eq!(filled(&home), [false, true, false, false, false]);
        let display = Arc::new(DisplayData::default());
        assert!(load(&home, 2, display.clone(), "b", &[1; 32]).unwrap() == game);
        assert_eq!(load(&home, 1, display.clone(), "b", &[1; 32]).err().unwrap(), "Save state 1 is empty.");
        assert!(load(&home, 2, display, "other", &[1; 32]).err().unwrap().contains("this is other"));
        let _ = std::fs::remove_dir_all(&home);
    }
}
