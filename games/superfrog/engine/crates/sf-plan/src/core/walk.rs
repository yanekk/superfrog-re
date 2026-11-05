//! The walk compass, now the toolkit's (`amiga_plan::walk`), with Superfrog-typed wrappers.

pub use amiga_plan::walk::*;
use sf_core::Game;

pub use super::facts::live_coins;
use super::facts::{Facts, Frog};

/// The walk compass from `g`, a start state, on `facts`' level: as the explorer builds it.
pub fn compass(g: &Game, facts: &Facts) -> WalkCompass {
    WalkCompass::new(&Frog::new(g.clone(), facts), &facts.plan())
}
