//! Route repair (DESIGN §2.7): `--follow` turns an old route into a guide path and an archive seed;
//! and a route's trace, the frog's cell, coins left and switches pressed every 8 frames, which the
//! guide is made of. The original's trace is the shell's (`trace::trace_original`); both sample
//! through `Sampler`, so a trace means the same whichever machine played it.

use sf_core::input::FrameInput;
use sf_core::layout::h0;
use sf_core::{Game, State};

use super::archive::COINS_LEFT;
use super::explore::Options;
use super::facts::{Cell, Facts, frog_cell};
use super::guide::GuidePoint;
use super::map::pressed;

/// `level_state` (h0+$a06): −2 while the level is played.
const LEVEL_STATE: u32 = h0(0xa06);
const PLAYING: u32 = 0xffff_fffe;
/// A trace samples the state before every this many frames.
pub const TRACE_EVERY: u32 = 8;

/// One sample of a played route: the frog's cell, the coins left, the switches pressed
/// (`Facts::switches` bits, as `CellKey::switches`). The switches deviate from the task's
/// `(Cell, u16)` (owner, 2026-10-22): see `guide`.
pub type TracePoint = (Cell, u16, u32);

/// Points sampled every 8 frames from a run the shell played: frog cell, coins left, switches.
pub fn points_from_trace(trace: &[TracePoint]) -> Vec<GuidePoint> {
    trace
        .iter()
        .map(|&(cell, coins, switches)| GuidePoint {
            cell,
            coins_left: Some(coins),
            switches: Some(switches),
        })
        .collect()
}

/// `--follow`: the old route's inputs as T08's `seed_inputs`, its trace as the guide with its pace
/// (replacing `opt.guide`; `sf-plan` re-applies an explicit `--guide` after it, DESIGN §2.7).
pub fn follow_options(
    opt: Options,
    old_inputs: Vec<FrameInput>,
    old_trace: &[TracePoint],
) -> Options {
    Options {
        seed_inputs: Some(old_inputs),
        guide: points_from_trace(old_trace),
        guide_pace: Some(TRACE_EVERY),
        ..opt
    }
}

/// A trace's samples, from states the caller steps: before each 8th frame (frame 0 first), while
/// the level is played. Samples before the level's play begins (a sync point a frame early) are
/// skipped; the first sample after it ends stops the trace.
#[derive(Debug)]
pub struct Sampler<'a> {
    facts: &'a Facts,
    pub points: Vec<TracePoint>,
    played: bool,
}

impl<'a> Sampler<'a> {
    pub fn new(facts: &'a Facts) -> Sampler<'a> {
        Sampler {
            facts,
            points: Vec::new(),
            played: false,
        }
    }

    /// Whether frame `frame`'s state is sampled: the caller builds `st` only then.
    pub fn due(frame: u32) -> bool {
        frame % TRACE_EVERY == 0
    }

    /// Takes a due state; false once the level has ended, the trace complete.
    pub fn sample(&mut self, st: &State) -> bool {
        if st.l(LEVEL_STATE) != PLAYING {
            return !self.played;
        }
        self.played = true;
        self.points
            .push((frog_cell(st), st.w(COINS_LEFT), pressed(self.facts, st)));
        true
    }
}

/// A route played on the engine from `start`, sampled every 8 frames (pure, core::repair): T10's
/// rest-of-plan guide, T12's app-timing guide. `facts` (the level's) places its switches.
pub fn trace_engine(start: &Game, facts: &Facts, inputs: &[FrameInput]) -> Vec<TracePoint> {
    let mut g = start.clone();
    let mut s = Sampler::new(facts);
    for (k, &x) in inputs.iter().enumerate() {
        if Sampler::due(k as u32) && !s.sample(g.state()) {
            return s.points;
        }
        g.step_unseen(x);
    }
    if Sampler::due(inputs.len() as u32) {
        s.sample(g.state());
    }
    s.points
}
