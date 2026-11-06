//! What `fd-app` and the tests that drive it share: the state hash its capture's `hashes.txt`
//! holds, so a test computes it on a headless [`Game`] the same way.

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
