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

    /// Whether the CPU can use DMA cycle `h` of `line`: refresh's cycles ([`super::cpu::refresh`])
    /// and the fetch's. Audio's are left out: a channel fetches a word every few lines, and the
    /// oracle's slot owners under scene 4's masks' clearing show none (T50).
    pub fn free_to_cpu(&self, line: u32, h: u16) -> bool {
        let l = (line % FRAME_LINES as u32) as u16;
        !super::cpu::refresh(h as u32) && !self.fetch.takes(l, h)
    }

    /// Whether the display fetches on `line`.
    pub fn fetched(&self, line: u32) -> bool {
        self.fetch.planes() > 0
            && self
                .fetch
                .lines()
                .contains(&((line % FRAME_LINES as u32) as u16))
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

/// The CPU's time between two blits outside a draw (wait for BLTBUSY, write five registers), in
/// DMA cycles. A draw's own CPU time is its program on [`super::sim`] (T53).
pub const CPU_PER_BLIT: u32 = 40;

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
    /// Where the handler started at each blank the run passed (it waits for the blit in progress).
    pub handlers: Vec<Beam>,
    /// The blanks (their first line) a blit held the CPU through to the next one: the interrupt's
    /// request is a latch, so the next blank's handler serves both.
    pub merged: Vec<u32>,
    /// Where the last blit started (after its set-up).
    pub last_start: Beam,
}

impl BlitClock {
    pub fn new(bus: Bus, from: Beam, handler: u32) -> BlitClock {
        BlitClock {
            bus,
            now: from,
            blanks: 0,
            handler,
            blits: 0,
            handlers: Vec::new(),
            merged: Vec::new(),
            last_start: from,
        }
    }

    /// The CPU works `cycles` DMA cycles (it gets the bus only between blits). A blank that comes
    /// meanwhile has its handler at once: the CPU's work goes on after it.
    pub fn cpu(&mut self, cycles: u32) {
        self.blank();
        let line = LINE_CYCLES as u32;
        let mut left = cycles;
        loop {
            let top = FRAME_LINES as u32 * (self.blanks + 1);
            let to_top = (top - self.now.line) * line - self.now.h as u32;
            if left < to_top {
                self.now = self.now.plus(left);
                return;
            }
            left -= to_top;
            self.blanks += 1;
            self.handlers.push(Beam::at_line(top));
            self.now = Beam::at_line(top).plus(self.handler);
        }
    }

    /// The CPU runs a loop of `turns` turns whose body is `ops` (bus accesses and idle cycles,
    /// [`super::cpu::Cpu::run`]'s letters), each access waiting for a cycle the bus leaves it.
    pub fn cpu_loop(&mut self, ops: &str, turns: u32) {
        for _ in 0..turns {
            for c in ops.bytes() {
                match c {
                    b'n' => {
                        self.now = self.now.plus(1);
                        continue;
                    }
                    b'p' | b'r' | b'w' => {}
                    _ => continue,
                }
                self.now = self.now.plus(1);
                while !self.bus.free_to_cpu(self.now.line, self.now.h) {
                    self.now = self.now.plus(1);
                }
                self.now = self.now.plus(1);
            }
            self.blank();
        }
    }

    /// One blit, after its set-up.
    pub fn blit(&mut self, con0: u16, size: u16) {
        self.blits += 1;
        self.cpu(CPU_PER_BLIT);
        self.last_start = self.now;
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
            if self.now.line >= frame * (self.blanks + 1) {
                self.merged.push(frame * self.blanks);
                continue;
            }
            let at = Beam::at_line(frame * self.blanks).max(self.now);
            self.handlers.push(at);
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

    #[test]
    fn the_masks_clearing_is_slowed_by_the_fetch() {
        // Fifteen DMA cycles a turn plus refresh's waits above the display (the oracle's 15.133),
        // about eighteen a turn under five lores planes (oracle: 75 accesses, 12.5 turns, a line).
        let mut c = BlitClock::new(scene_bus(), Beam::at_line(0x10), 0);
        c.cpu_loop("rpw pnn npp", 150);
        assert_eq!(c.now.line, 0x10 + 9);
        let mut c = BlitClock::new(scene_bus(), Beam::at_line(0x50), 0);
        c.cpu_loop("rpw pnn npp", 125);
        assert_eq!(c.now.line, 0x50 + 10);
    }

    #[test]
    fn a_blit_held_through_a_whole_frame_merges_its_blank_into_the_next() {
        // Scene 4's first two mask blits (1024 rows of 20 words, A B and D), the first from line 147
        // as in the original: it holds the next blank's handler, and the second holds the CPU
        // through the whole frame after.
        let mut c = BlitClock::new(scene_bus(), Beam::at_line(147), 12 * 227);
        c.blit(0x0dfc, 0x14);
        c.blit(0x0dfc, 0x14);
        assert_eq!(c.merged, vec![2 * FRAME_LINES as u32]);
        assert_eq!(c.handlers.len(), 2);
        assert!(c.handlers[1].line > 3 * FRAME_LINES as u32);
    }
}
