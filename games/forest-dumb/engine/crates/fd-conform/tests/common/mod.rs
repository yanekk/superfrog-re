//! A stand-in for the level engine, which T07 builds: a second copy of the original on the oracle,
//! started at the same sync point and stepped a pass per call, so the runner has an engine that
//! matches the original and can be made to differ in exactly one way. Shared by the level tests.

#![allow(dead_code)]

use std::ops::Range;

use amiga_chips::{Picture, Split};
use amiga_oracle::{Amiga, AmigaMemory, FrameInput, Rgb12Frame};
use fd_conform::{Case, Engine, Inputs, Live, OracleMem, PASS_END, PLAY_WINDOW, reach_case};
use fd_core::{Mem, Transplant, Var, layout_var};

/// The original's picture as an engine `Picture`: colour 0 of each line, rewritten by a split at
/// every pixel whose colour changes. Lossless, whatever the number of colours on a line.
pub fn picture_of(f: &Rgb12Frame) -> Picture {
    let mut p = Picture::blank(f.left as i32, f.top, f.width, f.height);
    for y in 0..f.height {
        let mut colour = f.pixel(0, y);
        p.palettes[y as usize][0] = colour;
        for x in 1..f.width {
            let c = f.pixel(x, y);
            if c != colour {
                p.splits.push(Split {
                    y,
                    x,
                    index: 0,
                    colour: c,
                });
                colour = c;
            }
        }
    }
    p
}

/// How the stand-in takes its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feed {
    /// The input the runner gives it.
    Given,
    /// The case's input `shift` frames off the frame it runs: +1 is a frame late, -1 early.
    Shifted(i32),
}

pub struct StandIn {
    am: Amiga,
    inputs: Inputs,
    feed: Feed,
    mem: Option<AmigaMemory>,
    pic: Option<Rgb12Frame>,
    passes: u32,
    /// Report `var` one higher than it is on this pass (counted from 1 after the transplant).
    pub bump: Option<(u32, &'static str)>,
    /// Paint `pixels` pixels of row `row` white on these passes.
    pub paint: Option<(Range<u32>, u16, u16)>,
    /// Raise an event on this pass.
    pub event: Option<u32>,
    events: Vec<String>,
}

fn irq() -> &'static Var {
    layout_var("irq_count").unwrap()
}

impl StandIn {
    pub fn new(case: &Case, feed: Feed) -> StandIn {
        StandIn {
            // Room for the look-ahead step past the case's last frame.
            am: reach_case(case, 4).unwrap_or_else(|e| panic!("{e}")),
            inputs: Inputs::load(case).unwrap_or_else(|e| panic!("{e}")),
            feed,
            mem: None,
            pic: None,
            passes: 0,
            bump: None,
            paint: None,
            event: None,
            events: Vec::new(),
        }
    }

    fn input(&self, frame: u32, given: FrameInput) -> FrameInput {
        match self.feed {
            Feed::Given => given,
            Feed::Shifted(k) => self.inputs.at((frame as i64 - k as i64) as u32),
        }
    }

    /// The step the original's pass `n` ended in, if it is this one.
    fn ended(&mut self, n: u32) -> bool {
        match self.am.watched_memory() {
            Some(m) if OracleMem(&m).value(irq()) == n => {
                self.pic = Some(self.am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1));
                self.mem = Some(m);
                true
            }
            _ => false,
        }
    }
}

impl Transplant for StandIn {
    /// Run the copy to the step end where the original's count is the transplanted pass's: the next
    /// step starts the pass after it, as on the runner's side.
    fn transplant(&mut self, mem: &dyn Mem) {
        let p0 = mem.value(irq());
        self.am.watch_write(Some(PASS_END));
        while Live(&self.am).value(irq()) < p0 {
            let f = self.am.frame();
            self.am.step(self.inputs.at(f)).unwrap();
        }
        assert_eq!(
            Live(&self.am).value(irq()),
            p0,
            "the stand-in passed the transplant"
        );
        self.passes = 0;
    }

    fn probe(&self, v: &Var) -> u32 {
        let m = self.mem.as_ref().expect("probed before a pass");
        let n = OracleMem(m).value(v);
        match self.bump {
            Some((pass, name)) if pass == self.passes && name == v.name => n + 1,
            _ => n,
        }
    }
}

impl Engine for StandIn {
    /// One step starts the next pass; its end is in this step or the next, which is run on a
    /// snapshot and taken back.
    fn pass(&mut self, given: FrameInput) {
        let f = self.am.frame();
        let input = self.input(f, given);
        self.am.step(input).unwrap();
        self.passes += 1;
        let n = Live(&self.am).value(irq());
        if !self.ended(n) {
            let snap = self.am.snapshot();
            let next = self.inputs.at(f + 1);
            let next = self.input(f + 1, next);
            self.am.step(next).unwrap();
            let seen = self.ended(n);
            self.am.restore(&snap);
            if !seen {
                // A step ended two passes and showed the first: the runner cannot see this end
                // either, so never compares it. Any state will do; take the step's end.
                self.mem = Some(self.am.memory());
                self.pic = Some(self.am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1));
            }
            self.am.watch_write(Some(PASS_END));
        }
        if self.event == Some(self.passes) {
            self.events.push("a test's event".to_string());
        }
    }

    fn picture(&self) -> Option<Picture> {
        let mut pic = self.pic.clone()?;
        if let Some((passes, row, pixels)) = &self.paint {
            if passes.contains(&self.passes) {
                let w = pic.width as usize;
                for x in 0..*pixels as usize {
                    let p = &mut pic.pixels[*row as usize * w + x];
                    // White, or black where the original is white: always a full step off.
                    *p = if *p == 0xfff { 0 } else { 0xfff };
                }
            }
        }
        Some(picture_of(&pic))
    }

    fn events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.events)
    }
}
