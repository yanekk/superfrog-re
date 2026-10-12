//! How long the intro's blits take (docs/13 "Blit timing"): the intro draws its bobs with the
//! blitter holding the bus (DMACON BLTPRI), so a draw takes as long as its blits, and a draw can
//! run past the next vertical blank. When it ends decides the frame its buffer is shown and the
//! frame its DMACON write lands in, so the engine counts the blitter's cycles on the bus.
//!
//! The blitter spends a cycle per step of its program (vAmiga's `SlowBlitter`, from the HRM's
//! table 6.2): A-to-D copies two a word, cookie-cuts (A, B, C, D) four, A-or-B-to-D three, plus
//! the pipeline's start and end. Every step waits for a bus cycle no other DMA has: refresh
//! (`$01`..`$07`), audio (`$0f`..`$15`) and the bitplane fetch. The CPU's own work between blits
//! (waiting on BLTBUSY, writing the next blit's registers) and the vertical blank's handler,
//! which takes the bus between two blits when the blank comes during a draw, are counted in CPU
//! lines measured on the oracle.

use crate::render::copper::{FRAME_LINES, Fetch, LINE_CYCLES};

/// A beam position: line in the frame counted from the draw's frame (it can pass
/// `FRAME_LINES`), DMA cycle in the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Beam {
    pub line: u32,
    pub h: u16,
}

impl Beam {
    pub fn at_line(line: u32) -> Beam {
        Beam { line, h: 0 }
    }

    /// `cycles` DMA cycles later, whatever the bus does.
    pub fn plus(self, cycles: u32) -> Beam {
        let h = self.h as u32 + cycles;
        Beam {
            line: self.line + h / LINE_CYCLES as u32,
            h: (h % LINE_CYCLES as u32) as u16,
        }
    }
}

/// Who else takes the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bus {
    /// The display's fetch (the scene's: five lores planes on lines `$44`..`$10c`).
    pub fetch: Fetch,
    /// Audio DMA on: its four slots are taken on every line.
    pub audio: bool,
}

impl Bus {
    /// Whether the blitter can use DMA cycle `h` of `line` (counted from a frame's start; past
    /// the frame's end it is the next frame's).
    pub fn free(&self, line: u32, h: u16) -> bool {
        let l = (line % FRAME_LINES as u32) as u16;
        if h % 2 == 1 && h <= 0x07 {
            return false; // refresh
        }
        if self.audio && h % 2 == 1 && (0x0f..=0x15).contains(&h) {
            return false;
        }
        !self.fetch.takes(l, h)
    }

    /// The position after the blitter has had `cycles` free bus cycles from `from`.
    pub fn run(&self, from: Beam, cycles: u32) -> Beam {
        let mut b = from;
        let mut left = cycles;
        while left > 0 {
            if self.free(b.line, b.h) {
                left -= 1;
            }
            b = b.plus(1);
        }
        b
    }
}

/// The blitter's cycles for one area blit: `words` words a row, `rows` rows, using channels
/// `abcd` (BLTCON0 bits 11..8).
pub fn blit_cycles(abcd: u16, words: u32, rows: u32) -> u32 {
    let n = words * rows;
    match abcd & 0xf {
        0xf | 0xe => 4 * n + 1,
        0xd | 0xb | 0x7 => 3 * n + 2,
        0x9 | 0x5 | 0x3 | 0x1 => 2 * n + 2,
        0xc | 0xa | 0x6 => 3 * n,
        _ => 2 * n,
    }
}

/// The CPU's time between two blits of a draw (wait for BLTBUSY, write five registers), in DMA
/// cycles; and per object around its blits (clipping, the next blit's set-up).
pub const CPU_PER_BLIT: u32 = 40;
pub const CPU_PER_OBJECT: u32 = 300;

/// A sequence of blits run back to back by the CPU, timed on the bus.
#[derive(Debug, Clone)]
pub struct BlitClock {
    pub bus: Bus,
    pub now: Beam,
    /// The vertical blanks the run has passed (their handler took the bus between blits).
    pub blanks: u32,
    /// The handler's length when it comes during a draw, in DMA cycles.
    pub handler: u32,
    /// Blits run so far.
    pub blits: u32,
}

impl BlitClock {
    pub fn new(bus: Bus, from: Beam, handler: u32) -> BlitClock {
        BlitClock {
            bus,
            now: from,
            blanks: 0,
            handler,
            blits: 0,
        }
    }

    /// The CPU works `cycles` DMA cycles (it gets the bus only between blits).
    pub fn cpu(&mut self, cycles: u32) {
        self.now = self.now.plus(cycles);
        self.blank();
    }

    /// One blit, after its set-up.
    pub fn blit(&mut self, con0: u16, size: u16) {
        self.blits += 1;
        self.cpu(CPU_PER_BLIT);
        let rows = match size >> 6 {
            0 => 1024,
            n => n as u32,
        };
        let words = match size & 63 {
            0 => 64,
            n => n as u32,
        };
        self.now = self.bus.run(self.now, blit_cycles(con0 >> 8, words, rows));
        self.blank();
    }

    /// A vertical blank passed since the last look: its handler runs now, between blits.
    fn blank(&mut self) {
        let frame = FRAME_LINES as u32;
        while self.now.line >= frame * (self.blanks + 1) {
            self.blanks += 1;
            let at = Beam::at_line(frame * self.blanks).max(self.now);
            self.now = at.plus(self.handler);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_bus() -> Bus {
        Bus {
            fetch: Fetch {
                bplcon0: 0x5200,
                diwstrt: 0x4481,
                diwstop: 0x0cc1,
                ddfstrt: 0x38,
                ddfstop: 0xd0,
            },
            audio: true,
        }
    }

    #[test]
    fn a_copy_takes_two_cycles_a_word_and_the_display_slows_it() {
        assert_eq!(blit_cycles(0x9, 22, 1000), 44_002);
        assert_eq!(blit_cycles(0xf, 3, 1), 13);
        let bus = scene_bus();
        // Above the display only refresh and audio take cycles: 219 of 227 a line are free.
        let end = bus.run(Beam::at_line(0), 219);
        assert_eq!(end, Beam { line: 1, h: 0 });
        // Inside it the five planes take 100 more.
        let end = bus.run(Beam::at_line(0x50), 119);
        assert_eq!(end, Beam { line: 0x51, h: 0 });
    }
}
