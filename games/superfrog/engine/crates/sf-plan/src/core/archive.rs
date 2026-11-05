//! The explorer's archive, now the toolkit's (`amiga_plan::archive`), with Superfrog-typed
//! wrappers: a state's key and coin distance read off a `State`, and replay on a `Game`.

use amiga_plan::PlanGame;
pub use amiga_plan::archive::*;
use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, State};

use super::facts::{self, CoinKind, Facts, Frog, frog_cell};

/// The archive over Superfrog's states.
pub type Archive = amiga_plan::archive::Archive<Frog>;
pub type Offer<'a> = amiga_plan::archive::Offer<'a, Frog>;

/// `st`'s key, `via` hints passed: as the toolkit's `cell_key` reads a `Frog`.
pub fn cell_key(facts: &Facts, st: &State, via: u8) -> CellKey {
    let (c, r) = frog_cell(st);
    CellKey {
        col: c / 2,
        row: r / 2,
        band: band(facts::coins_left(st)),
        switches: facts::pressed(facts, st),
        wings: facts::wings(st),
        via,
    }
}

/// Straight distance in cells from the frog to the nearest object coin still in the map: as the
/// toolkit's `coin_dist` reads a `Frog`.
pub fn coin_dist(facts: &Facts, st: &State) -> f64 {
    let (c, r) = frog_cell(st);
    facts
        .coins
        .iter()
        .filter(|k| matches!(k.kind, CoinKind::Object(_)))
        .filter(|k| facts::coin_live(st, k.cell))
        .map(|k| (k.cell.0.abs_diff(c) + k.cell.1.abs_diff(r)) as f64)
        .fold(f64::MAX, f64::min)
}

/// A state's bytes as the archive counts them (`Frog::state_bytes`).
pub fn state_size(g: &Game) -> u64 {
    Frog::unswitched(g.clone()).state_bytes()
}

/// Plays joystick bytes on `g`, as the toolkit's `replay` plays them on a `Frog`.
pub fn replay(g: &mut Game, joys: &[u8]) {
    for &j in joys {
        g.step_unseen(FrameInput::joy(Joy(j)));
    }
}
