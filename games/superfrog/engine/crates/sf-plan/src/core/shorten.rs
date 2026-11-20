//! Route shortening, now the toolkit's (`amiga_plan::shorten`), with Superfrog-typed wrappers.

pub use amiga_plan::shorten::*;
use sf_core::Game;
use sf_core::input::FrameInput;

use super::facts::{Facts, Frog};

/// The shortcut pass's bookkeeping over Superfrog's states.
pub type Shortcut = amiga_plan::shorten::Shortcut<Frog>;

/// The pass over `route`, an exit from `start` (`Shortcut::new`).
pub fn shortcut(start: &Game, facts: &Facts, route: Vec<FrameInput>, rounds: u64) -> Shortcut {
    Shortcut::new(
        &Frog::new(start.clone(), facts),
        &facts.plan(),
        route,
        rounds,
    )
}

/// The toolkit's `replay_exit` from a `Game`.
pub fn replay_exit(start: &Game, inputs: &[FrameInput], frames: &mut u64) -> Option<(usize, u8)> {
    amiga_plan::shorten::replay_exit(&Frog::unswitched(start.clone()), inputs, frames)
}
