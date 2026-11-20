//! What `fd-app` and the tests that drive it share: the state hash its capture's `hashes.txt`
//! holds, so a test computes it on a headless [`Game`] the same way; and for the translated
//! program (translator T37) the window its frames are shown through, its state hash, and
//! `--level N`'s way through the title (a world start's writes too), so a test drives a headless
//! run the same way.

pub mod audio;

use std::hash::{Hash, Hasher};

use fd_core::Game;

/// The capture's file of state hashes: `N hash` a line, hash in hex, for each frame N (0 the start).
pub const HASHES: &str = "hashes.txt";

/// The game's play state as a number: its scene, variables, objects and collision plane, what
/// [`Game`]'s equality compares (the data aside, which is the same for a run). Stable within one
/// build, which is all a test of that build needs.
pub fn state_hash(g: &Game) -> u64 {
    // DefaultHasher::new() has fixed keys: two processes of one build agree.
    let mut h = std::collections::hash_map::DefaultHasher::new();
    g.scene().hash(&mut h);
    g.state().hash(&mut h);
    g.objects().hash(&mut h);
    g.plane().hash(&mut h);
    h.finish()
}

/// One line of [`HASHES`].
pub fn hash_line(n: u64, g: &Game) -> String {
    format!("{n} {:016x}\n", state_hash(g))
}

/// What the app says, and exits 1 on, when it is built with no translated program (translator
/// DESIGN §2.10, §2.14): the stub tree, every run of which stops at once.
pub const NOT_GENERATED: &str = "The translated program is not built in: generate it with \
`python3 -I games/forest-dumb/tools/translate.py --build` from the repository's root, then build \
the app again. `--engine hand` plays level 1 on the hand engine meanwhile.";

/// The translated program's window, `--level N`'s way through the title and where play is, from
/// `fd_core::recording` since translator T39: a bug replay starts the same way the app does.
pub use fd_core::recording::{
    CANVAS, LOAD_WORLD, TO_LEVEL_1_FRAMES, canvas, in_level_1, level_in_ram, location_in_ram,
    start_reached, to_level_1_input, world_start_bytes, world_start_writes,
};

/// Where a start of `level` with the cheat as given begins, as a recording names it: W1-1 with the
/// cheat off is level 1's old start, no write at `load_world` (what `--level 1` was before the
/// title plan, so its recordings read and compare as before); any other is a world start, its
/// writes made as `load_world` is entered ([`world_start_bytes`]).
pub fn world_start(level: fd_core::recording::LevelId, cheat: bool) -> fd_core::recording::StartPoint {
    use fd_core::recording::{LevelId, StartPoint};
    match (level, cheat) {
        (LevelId::W1_1, false) => StartPoint::Level1,
        _ => StartPoint::World { level, cheat },
    }
}

/// The translated program's state as a number: chip RAM then slow RAM at a frame's end, what a
/// headless run of `fd-translated` holds at the same frame. Stable within one build.
pub fn memory_hash(chip: &[u8], slow: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    h.write(chip);
    h.write(slow);
    h.finish()
}

/// One line of [`HASHES`] for the translated program.
pub fn memory_hash_line(n: u64, chip: &[u8], slow: &[u8]) -> String {
    format!("{n} {:016x}\n", memory_hash(chip, slow))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_1_without_the_cheat_is_level_1_s_old_start_and_every_other_a_world_start() {
        use fd_core::recording::{LevelId, StartPoint};
        assert_eq!(world_start(LevelId::W1_1, false), StartPoint::Level1);
        for l in LevelId::ALL {
            assert_eq!(world_start(l, true), StartPoint::World { level: l, cheat: true });
            assert_eq!(world_start(l, false).level(), Some(l));
        }
        assert_eq!(
            world_start(LevelId::W4_1, false),
            StartPoint::World { level: LevelId::W4_1, cheat: false }
        );
    }

    #[test]
    fn the_message_names_the_command_that_generates_the_tree() {
        assert!(NOT_GENERATED.contains("python3 -I games/forest-dumb/tools/translate.py --build"));
        assert!(NOT_GENERATED.contains("--engine hand"));
    }
}
