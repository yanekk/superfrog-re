//! Route repair, now the toolkit's (`amiga_plan::repair`), with Superfrog-typed wrappers: a trace
//! sampled off a `State`, so the original's memory (`trace::trace_original`) and the engine
//! sample alike.

use std::ops::{Deref, DerefMut};

pub use amiga_plan::repair::*;
use sf_core::input::FrameInput;
use sf_core::{Game, State};

use super::facts::{self, Facts, Frog, frog_cell};

/// The toolkit's `Sampler`, fed `State`s of `facts`' level.
#[derive(Debug)]
pub struct Sampler<'a> {
    facts: &'a Facts,
    inner: amiga_plan::repair::Sampler,
}

impl<'a> Sampler<'a> {
    pub fn new(facts: &'a Facts) -> Sampler<'a> {
        Sampler {
            facts,
            inner: amiga_plan::repair::Sampler::new(),
        }
    }

    /// Whether frame `frame`'s state is sampled: the caller builds `st` only then.
    pub fn due(frame: u32) -> bool {
        amiga_plan::repair::Sampler::due(frame)
    }

    /// Takes a due state; false once the level has ended, the trace complete.
    pub fn sample(&mut self, st: &State) -> bool {
        let facts = self.facts;
        self.inner.take(facts::playing(st), || {
            (
                frog_cell(st),
                facts::coins_left(st),
                facts::pressed(facts, st),
            )
        })
    }
}

impl Deref for Sampler<'_> {
    type Target = amiga_plan::repair::Sampler;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Sampler<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// A route played on the engine from `start`, sampled every 8 frames (the toolkit's
/// `trace_engine`): T10's rest-of-plan guide, T12's app-timing guide.
pub fn trace_engine(start: &Game, facts: &Facts, inputs: &[FrameInput]) -> Vec<TracePoint> {
    amiga_plan::repair::trace_engine(&Frog::new(start.clone(), facts), &facts.plan(), inputs)
}
