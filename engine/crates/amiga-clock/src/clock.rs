//! A beam clock: where in the frame the CPU's and the blitter's work has brought the beam, on a
//! display whose fetches take a share of each line ([`Timing`]).

use crate::{BeamPos, LINE_CYCLES, LINE_SLOTS, is_refresh};
use core::marker::PhantomData;
use core::ops::Range;

/// A display's set-up, as the clock needs it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayTiming {
    /// Lines in the frame the clock wraps its fetch test at.
    pub frame_lines: u16,
    /// The lines the display fetches bitplanes on.
    pub fetch_lines: Range<u16>,
    /// The DMA slots of a fetching line the fetch window covers.
    pub fetch_slots: Range<u32>,
    /// Which slots of each 8 in the window the blitter may still have (bit `n`: slot `n`).
    pub blitter_free: u8,
    /// CPU cycles a fetching line takes from the CPU.
    pub stolen_cycles: u32,
}

/// A display's timing as a type, so a [`Clock`] carries it without carrying it in its bytes.
pub trait Timing: Copy + Default + Eq + core::hash::Hash + core::fmt::Debug {
    const TIMING: DisplayTiming;
}

/// The beam's position as the CPU sees it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(bound = "")]
pub struct Clock<T: Timing> {
    /// The line, counted from the frame's vertical blank; past the frame's end when work runs on
    /// past it.
    pub line: u16,
    /// How far into the line, in [`LINE_CYCLES`] (the beam's own pace, two a DMA slot).
    pub into: u32,
    // Zero-sized and skipped: a saved clock is its line and position, whatever its timing.
    #[serde(skip)]
    _t: PhantomData<T>,
}

impl<T: Timing> Clock<T> {
    /// The frame's start, where the vertical blank's interrupt is taken.
    pub const START: Self = Self::at(0, 0);

    /// `into` CPU cycles into `line`.
    pub const fn at(line: u16, into: u32) -> Self {
        Clock {
            line,
            into,
            _t: PhantomData,
        }
    }

    fn fetching(line: u16) -> bool {
        T::TIMING.fetch_lines.contains(&(line % T::TIMING.frame_lines))
    }

    /// CPU cycles a line gives the CPU.
    fn cpu_cycles(line: u16) -> u32 {
        if Self::fetching(line) {
            LINE_CYCLES - T::TIMING.stolen_cycles
        } else {
            LINE_CYCLES
        }
    }

    /// The CPU works `cycles`.
    pub fn cpu(&mut self, mut cycles: u32) {
        loop {
            let per = Self::cpu_cycles(self.line);
            // What is left of the line, in the CPU's cycles.
            let left = (LINE_CYCLES - self.into.min(LINE_CYCLES)) * per / LINE_CYCLES;
            if cycles < left {
                self.into += cycles * LINE_CYCLES / per;
                return;
            }
            cycles -= left;
            self.into = 0;
            self.line = self.line.saturating_add(1);
        }
    }

    /// Whether the blitter may have DMA slot `slot` of line `line`: not the memory refresh's
    /// ([`is_refresh`]), and on a fetching line inside the fetch window only the free slots of
    /// each 8. Audio and sprite fetches, which come and go, are not counted.
    pub fn blitter_slot(line: u16, slot: u32) -> bool {
        if is_refresh(slot) {
            return false;
        }
        let window = T::TIMING.fetch_slots;
        if Self::fetching(line) && window.contains(&slot) {
            return T::TIMING.blitter_free >> ((slot - window.start) % 8) & 1 != 0;
        }
        true
    }

    /// The blitter has the bus for `slots` DMA slots, the CPU waiting.
    pub fn blit(&mut self, mut slots: u32) {
        let line_slots = LINE_SLOTS as u32;
        let mut slot = self.into / 2;
        while slots > 0 {
            if slot >= line_slots {
                slot = 0;
                self.line = self.line.saturating_add(1);
            }
            if Self::blitter_slot(self.line, slot) {
                slots -= 1;
            }
            slot += 1;
        }
        self.into = 2 * slot;
        if self.into >= LINE_CYCLES {
            self.into = 0;
            self.line = self.line.saturating_add(1);
        }
    }

    /// A blit of `words` words with BLTCON0 `con0`, started (BLTSIZE written) here, cycle by
    /// cycle as the reference emulator's blitter runs it (vAmiga's `SlowBlitter`): two start-up
    /// cycles, then one micro-instruction a cycle, each that uses the bus waiting for a DMA slot
    /// free to it: per word a read for each of A, B and C it uses, then the pipelined D write of
    /// the word before (the first word's only waits for a free slot); after the last word one
    /// cycle needing no bus, then the last word's D. Returns where each word's D write was made;
    /// the clock is left where the blit ended.
    pub fn blit_cycles(&mut self, words: u32, con0: u16) -> Vec<u32> {
        let line_slots = LINE_SLOTS as u32;
        let reads = (con0 >> 9 & 7).count_ones();
        let mut slot = self.into / 2;
        let mut line = self.line;
        // The next cycle that may run an instruction: with `bus`, the next free one.
        let mut next = |bus: bool| loop {
            slot += 1;
            if slot >= line_slots {
                slot -= line_slots;
                line = line.saturating_add(1);
            }
            if !bus || Self::blitter_slot(line, slot) {
                return line as u32 * LINE_CYCLES + 2 * slot;
            }
        };
        // BLTSIZE takes effect a cycle after its write; the blit starts on the next two cycles
        // that find the bus free.
        next(false);
        next(true);
        next(true);
        let mut out = Vec::with_capacity(words as usize);
        for w in 0..words {
            for _ in 0..reads {
                next(true);
            }
            let at = next(true);
            if w > 0 {
                out.push(at);
            }
        }
        next(false);
        out.push(next(true));
        let end = line as u32 * LINE_CYCLES + 2 * (slot + 1);
        self.line = (end / LINE_CYCLES) as u16;
        self.into = end % LINE_CYCLES;
        if con0 & 0x100 == 0 {
            out.clear();
        }
        out
    }

    /// The position in [`LINE_CYCLES`] from the frame's start.
    pub fn nominal(&self) -> u32 {
        self.pos().nominal()
    }

    /// The horizontal position in DMA slots, as the beam counts it.
    pub fn hpos(&self) -> u16 {
        self.pos().paula_hpos()
    }

    /// The position as a [`BeamPos`].
    pub fn pos(&self) -> BeamPos {
        BeamPos::new(self.line, self.into)
    }

    /// Past the frame's end: the same moment counted from the next frame's start.
    pub fn next_frame(&self) -> Self {
        Self::at(self.line.saturating_sub(T::TIMING.frame_lines), self.into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A display with no fetch: every line the CPU's and the blitter's, but refresh.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    struct Blank;
    impl Timing for Blank {
        const TIMING: DisplayTiming = DisplayTiming {
            frame_lines: crate::SHORT_FRAME_LINES,
            fetch_lines: 0..0,
            fetch_slots: 0..0,
            blitter_free: 0,
            stolen_cycles: 0,
        };
    }

    /// Every slot of a 16-slot window fetched, on lines 10 to 19, and half each line's CPU.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    struct Busy;
    impl Timing for Busy {
        const TIMING: DisplayTiming = DisplayTiming {
            frame_lines: crate::FRAME_LINES,
            fetch_lines: 10..20,
            fetch_slots: 0x40..0x50,
            blitter_free: 0,
            stolen_cycles: LINE_CYCLES / 2,
        };
    }

    #[test]
    fn a_timing_decides_what_a_line_gives() {
        let mut c = Clock::<Blank>::START;
        // The line's 223 free slots run to `$e1`; the next is refresh's `$e2`.
        c.blit(LINE_SLOTS as u32 - 4);
        assert_eq!((c.line, c.into), (0, 2 * 0xe2));
        let mut c = Clock::<Busy>::at(10, 0);
        c.cpu(LINE_CYCLES / 2);
        assert_eq!((c.line, c.into), (11, 0));
        assert!(!Clock::<Busy>::blitter_slot(12, 0x48));
        assert!(Clock::<Busy>::blitter_slot(12, 0x50));
        assert!(Clock::<Busy>::blitter_slot(20, 0x48));
        // Lines wrap at the timing's frame: line 323 is the next frame's 10.
        assert!(!Clock::<Busy>::blitter_slot(crate::FRAME_LINES + 10, 0x48));
        assert_eq!(Clock::<Blank>::at(crate::FRAME_LINES, 7).next_frame(), Clock::at(1, 7));
    }
}
