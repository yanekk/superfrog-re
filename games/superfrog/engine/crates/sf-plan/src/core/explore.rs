//! The explorer, now the toolkit's (`amiga_plan::explore`), wrapped to start from a `Game` and
//! Superfrog's `Facts` as before.

use std::ops::{Deref, DerefMut};

pub use amiga_plan::explore::*;
use sf_core::Game;

use super::facts::{Facts, Frog};

/// A round's part result over Superfrog's states.
pub type PartResult = amiga_plan::explore::PartResult<Frog>;

/// The toolkit's explorer on Superfrog; derefs to it.
pub struct Explorer(pub amiga_plan::Explorer<Frog>);

impl Explorer {
    pub fn new(start: Game, facts: Facts, opt: Options) -> Explorer {
        let plan = facts.plan();
        Explorer(amiga_plan::Explorer::new(
            Frog::new(start, &facts),
            plan,
            opt,
        ))
    }
}

impl Deref for Explorer {
    type Target = amiga_plan::Explorer<Frog>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Explorer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
