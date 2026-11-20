//! Route repair: following an old route turns it into a guide path and an archive seed; and a
//! route's trace, the player's cell, collectibles left and switches pressed every 8 frames, which
//! the guide is made of. A trace of the original machine is the game's shell's; both sample through
//! `Sampler`, so a trace means the same whichever machine played it.

use amiga_input::FrameInput;

use super::explore::Options;
use super::guide::GuidePoint;
use super::{Cell, Facts, PlanGame};

/// A trace samples the state before every this many frames.
pub const TRACE_EVERY: u32 = 8;

/// One sample of a played route: the player's cell, the collectibles left, the switches pressed
/// (`Facts::gates` bits, as `CellKey::switches`; see `guide` for why switches are in).
pub type TracePoint = (Cell, u16, u32);

/// Points sampled every 8 frames from a run the shell played: cell, collectibles left, switches.
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

/// Following an old route: its inputs as `seed_inputs`, its trace as the guide with its pace
/// (replacing `opt.guide`; a shell may re-apply an explicit guide after it).
pub fn follow_options(
    opt: Options,
    old_inputs: Vec<FrameInput>,
    old_trace: &[TracePoint],
) -> Options {
    Options {
        seed_inputs: Some(old_inputs),
        guide: points_from_trace(old_trace),
        guide_pace: true,
        ..opt
    }
}

/// A trace's samples, from states the caller steps: before each 8th frame (frame 0 first), while
/// the level is played. Samples before the level's play begins (a sync point a frame early) are
/// skipped; the first sample after it ends stops the trace.
#[derive(Debug, Default)]
pub struct Sampler {
    pub points: Vec<TracePoint>,
    played: bool,
}

impl Sampler {
    pub fn new() -> Sampler {
        Sampler::default()
    }

    /// Whether frame `frame`'s state is sampled: the caller builds `st` only then.
    pub fn due(frame: u32) -> bool {
        frame % TRACE_EVERY == 0
    }

    /// Takes a due state; false once the level has ended, the trace complete.
    pub fn sample<G: PlanGame>(&mut self, g: &G, facts: &Facts) -> bool {
        self.take(g.playing(), || {
            (g.cell(), g.collectibles_left(), g.pressed(facts))
        })
    }

    /// `sample` from what a state reads, for a machine that is not a `PlanGame` (the original):
    /// whether the level is played, and the point, read only then.
    pub fn take(&mut self, playing: bool, point: impl FnOnce() -> TracePoint) -> bool {
        if !playing {
            return !self.played;
        }
        self.played = true;
        self.points.push(point());
        true
    }
}

/// A route played on the engine from `start`, sampled every 8 frames: a repair's rest-of-plan
/// guide, or a guide in another timing. `facts` (the level's) places its switches.
pub fn trace_engine<G: PlanGame>(
    start: &G,
    facts: &Facts,
    inputs: &[FrameInput],
) -> Vec<TracePoint> {
    let mut g = start.clone();
    let mut s = Sampler::new();
    for (k, &x) in inputs.iter().enumerate() {
        if Sampler::due(k as u32) && !s.sample(&g, facts) {
            return s.points;
        }
        g.step(x);
    }
    if Sampler::due(inputs.len() as u32) {
        s.sample(&g, facts);
    }
    s.points
}
