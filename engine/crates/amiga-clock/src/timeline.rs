//! Work done at once, replayed on the beam: CPU cycles spent on whole lines (all of a line's: the
//! screen is blank meanwhile), each write made where it falls, a yield at the frame's last line.

use crate::{FRAME_LINES, LINE_CYCLES};

/// The beam a [`Timeline`] moves: its line, the writes it makes, the job it hands over.
pub trait TimelineBeam<J> {
    fn line(&self) -> u16;
    fn set_line(&mut self, line: u16);
    fn write(&mut self, reg: u16, value: u16);
    fn job(&mut self, job: J);
}

/// Where a [`Timeline::run`] stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineYield {
    /// At the frame's last line: on after the next vertical blank.
    Poll,
    /// Every write made and every cycle spent.
    Return,
}

/// The cycles before each write and the write, then the cycles after the last, with the job the
/// work belongs to (handed to the beam as the run starts).
///
/// The fields' order is the save state's (bincode lays them out in order).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Timeline<J> {
    /// `(cycles before it, register, value)`.
    steps: Vec<(u32, u16, u16)>,
    /// Cycles after the last write.
    tail: u32,
    at: usize,
    /// Cycles of the current step still to spend.
    left: u32,
    /// Cycles of the beam's current line already used.
    spare: u32,
    job: Option<J>,
}

impl<J> Timeline<J> {
    pub fn new(steps: Vec<(u32, u16, u16)>, tail: u32, job: J) -> Self {
        let left = steps.first().map_or(tail, |s| s.0);
        Timeline {
            steps,
            tail,
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

    pub fn run<B: TimelineBeam<J>>(&mut self, beam: &mut B) -> TimelineYield {
        if let Some(j) = self.job.take() {
            beam.job(j);
        }
        loop {
            while self.left > 0 {
                if beam.line() >= FRAME_LINES - 1 {
                    // Past the frame's end: on after the next vertical blank.
                    beam.set_line(FRAME_LINES - 1);
                    self.spare = 0;
                    return TimelineYield::Poll;
                }
                let free = LINE_CYCLES - self.spare;
                if self.left < free {
                    self.spare += self.left;
                    self.left = 0;
                } else {
                    self.left -= free;
                    self.spare = 0;
                    beam.set_line(beam.line() + 1);
                }
            }
            let Some(&(_, reg, value)) = self.steps.get(self.at) else {
                return TimelineYield::Return;
            };
            beam.write(reg, value);
            self.at += 1;
            self.left = self.steps.get(self.at).map_or(self.tail, |s| s.0);
            if self.left == 0 && self.at >= self.steps.len() {
                return TimelineYield::Return;
            }
        }
    }
}
