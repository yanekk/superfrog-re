//! Level work the original's main program does across frames between two levels (the restart's
//! `level_init_wN`, T37): the engine does it at once, then replays its time and its writes on the
//! front end's beam.
//!
//! The ported routines say what they cost as they run ([`super::LevelCtx::spend`], the blits by
//! their words); with the context's trace kept, that gives each Paula write's place in CPU cycles
//! from the work's start ([`Timeline::of`]). A [`Timeline`] spends those cycles on the lines the
//! main program has (all of a line's: the screen is blank meanwhile) and makes each write where it
//! falls.

use crate::front::{BEAM_CYCLES_PER_LINE, Cx, FRAME_LINES, Yield};
use crate::game::Job;
use crate::level::beam::Tick;

/// Work done at once, replayed on the beam: the cycles before each write and the write, then the
/// cycles after the last. Its memory is compared once the original is out of it (`job`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Timeline {
    /// `(cycles before it, register, value)`.
    steps: Vec<(u32, u16, u16)>,
    /// Cycles after the last write.
    tail: u32,
    at: usize,
    /// Cycles of the current step still to spend.
    left: u32,
    /// Cycles of the beam's current line already used.
    spare: u32,
    job: Option<Job>,
}

impl Timeline {
    /// The timeline of a context's `trace` (with `costs`) and its Paula `writes` (register, value).
    pub fn of(trace: &[Tick], costs: &[u32], writes: &[(u16, u16)], job: Job) -> Timeline {
        let mut steps = Vec::new();
        let mut cycles = 0u32;
        let mut last = 0u32;
        for t in trace {
            match *t {
                Tick::Cpu(k, n) => cycles += n * costs[k as usize],
                Tick::Blit(slots) => cycles += 2 * slots,
                Tick::Wait(lines) => cycles += lines as u32 * BEAM_CYCLES_PER_LINE,
                Tick::Write(k) => {
                    if let Some(&(reg, value)) = writes.get(k as usize) {
                        steps.push((cycles - last, reg, value));
                        last = cycles;
                    }
                }
            }
        }
        let left = steps.first().map_or(cycles, |s| s.0);
        Timeline {
            steps,
            tail: cycles - last,
            at: 0,
            left,
            spare: 0,
            job: Some(job),
        }
    }

    /// The cycles in all.
    pub fn cycles(&self) -> u32 {
        self.steps.iter().map(|s| s.0).sum::<u32>() + self.tail
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        if let Some(j) = self.job.take() {
            cx.jobs.push(j);
        }
        loop {
            while self.left > 0 {
                if cx.line >= FRAME_LINES - 1 {
                    // Past the frame's end: on after the next vertical blank.
                    cx.line = FRAME_LINES - 1;
                    self.spare = 0;
                    return Yield::Poll;
                }
                let free = BEAM_CYCLES_PER_LINE - self.spare;
                if self.left < free {
                    self.spare += self.left;
                    self.left = 0;
                } else {
                    self.left -= free;
                    self.spare = 0;
                    cx.line += 1;
                }
            }
            let Some(&(_, reg, value)) = self.steps.get(self.at) else {
                return Yield::Return;
            };
            cx.wr(reg, value);
            self.at += 1;
            self.left = self.steps.get(self.at).map_or(self.tail, |s| s.0);
            if self.left == 0 && self.at >= self.steps.len() {
                return Yield::Return;
            }
        }
    }
}
