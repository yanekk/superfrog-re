//! The copper, run for one frame (docs/12 "Copper timing").
//!
//! The level's copper list sets the panel and play-field palettes, the bitplane pointers, the
//! scroll and the sprites, partly in the middle of a line. Which pixel a write lands on depends on
//! when the copper gets the bus, so the list is run cycle by cycle on the original's bus model:
//! the copper takes an even DMA cycle that no bitplane fetch uses, never cycle `$E0`, and spends
//! two cycles on each instruction word. WAIT wakes up as the reference emulator's comparator does
//! (vAmiga `Copper::findMatch`, `runComparator`). The result is every register write with the
//! beam position it happened at; Denise's side (what a write changes on screen) is in `mod.rs`.

/// DMA cycles per PAL line: `h` runs `0..=$E2`.
pub const LINE_CYCLES: u16 = 0xE3;
/// Lines per long PAL frame.
pub const FRAME_LINES: u16 = 313;

/// Custom registers the renderer follows, by their offset from `$DFF000`.
pub mod reg {
    pub const DIWSTRT: u16 = 0x08E;
    pub const DIWSTOP: u16 = 0x090;
    pub const DDFSTRT: u16 = 0x092;
    pub const DDFSTOP: u16 = 0x094;
    pub const BPL1PTH: u16 = 0x0E0;
    pub const BPLCON0: u16 = 0x100;
    pub const BPLCON1: u16 = 0x102;
    pub const BPLCON2: u16 = 0x104;
    pub const BPL1MOD: u16 = 0x108;
    pub const BPL2MOD: u16 = 0x10A;
    pub const SPR0PTH: u16 = 0x120;
    pub const SPR0POS: u16 = 0x140;
    pub const COLOR00: u16 = 0x180;
}

/// One register write by the copper, at beam position (`line`, `h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write {
    pub line: u16,
    pub h: u16,
    pub reg: u16,
    pub value: u16,
}

/// What decides which DMA cycles the bitplanes take: BPLCON0, the vertical window and the fetch
/// window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Fetch {
    pub bplcon0: u16,
    pub diwstrt: u16,
    pub diwstop: u16,
    pub ddfstrt: u16,
    pub ddfstop: u16,
}

impl Fetch {
    pub fn planes(&self) -> u16 {
        (self.bplcon0 >> 12 & 7).min(6)
    }

    pub fn hires(&self) -> bool {
        self.bplcon0 & 0x8000 != 0
    }

    /// The lines bitplane DMA runs on: OCS's vertical window (VSTOP's bit 8 is the inverse of
    /// its bit 7).
    pub fn lines(&self) -> std::ops::Range<u16> {
        let start = self.diwstrt >> 8;
        let stop = self.diwstop >> 8 | (!self.diwstop >> 7 & 0x100);
        start..stop
    }

    /// The fetch window's first DMA cycle (OCS ignores the low two bits).
    pub fn first_cycle(&self) -> u16 {
        self.ddfstrt & 0xFC
    }

    /// Words fetched per plane and line.
    pub fn words(&self) -> u16 {
        let span = (self.ddfstop & 0xFC).saturating_sub(self.first_cycle());
        if self.hires() {
            span / 4 + 2
        } else {
            span / 8 + 1
        }
    }

    /// Whether a bitplane fetch takes DMA cycle `h` on `line`. Lores fetch units are eight
    /// cycles, planes in the order `-, 4, 6, 2, -, 3, 5, 1`; hires units four, `4, 2, 3, 1`.
    pub fn takes(&self, line: u16, h: u16) -> bool {
        let planes = self.planes();
        if planes == 0 || !self.lines().contains(&line) {
            return false;
        }
        let start = self.first_cycle();
        if h < start || h >= start + self.words() * if self.hires() { 4 } else { 8 } {
            return false;
        }
        let plane = if self.hires() {
            [4, 2, 3, 1][((h - start) % 4) as usize]
        } else {
            [0, 4, 6, 2, 0, 3, 5, 1][((h - start) % 8) as usize]
        };
        plane != 0 && plane <= planes
    }

    pub fn write(&mut self, reg: u16, value: u16) {
        match reg {
            reg::BPLCON0 => self.bplcon0 = value,
            reg::DIWSTRT => self.diwstrt = value,
            reg::DIWSTOP => self.diwstop = value,
            reg::DDFSTRT => self.ddfstrt = value,
            reg::DDFSTOP => self.ddfstop = value,
            _ => {}
        }
    }
}

/// A beam position: line and DMA cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Beam {
    pub line: u16,
    pub h: u16,
}

impl Beam {
    fn plus(self, cycles: u16) -> Beam {
        let h = self.h + cycles;
        Beam {
            line: self.line + h / LINE_CYCLES,
            h: h % LINE_CYCLES,
        }
    }

    fn word(self) -> u32 {
        (self.line as u32) << 8 | self.h as u32
    }
}

/// Run the copper list at `start` for one frame from the vertical blank, `word` reading chip
/// memory. `fetch` is the bitplane setup the frame starts with (the previous frame's, as the
/// list left it). Returns every register write in beam order.
pub fn run(word: &dyn Fn(u32) -> u16, start: u32, fetch: Fetch) -> Vec<Write> {
    run_timed(&|a, _| word(a), start, fetch)
}

/// [`run`], `word(addr, line)` reading the list as the copper sees it on that line: a list the CPU
/// rewrites during the frame is read old above the line it was rewritten on, new below.
pub fn run_timed(word: &dyn Fn(u32, u16) -> u16, start: u32, fetch: Fetch) -> Vec<Write> {
    run_beam(&|a, line, _| word(a, line), start, fetch)
}

/// [`run_timed`], `word(addr, line, h)` given the DMA cycle of the read as well (a list a routine
/// rewrites word by word as the copper reads it).
pub fn run_beam(word: &dyn Fn(u32, u16, u16) -> u16, start: u32, fetch: Fetch) -> Vec<Write> {
    run_bus(word, start, fetch, &mut Vec::new())
}

/// [`run_beam`], also giving every bus cycle the copper takes, in order: what the CPU and the
/// blitter find taken. As vAmiga's `Copper::serviceEvent` has it, the vertical blank's restart
/// allocates cycle 0 and each instruction word's fetch its cycle; WAIT1, WAIT2 (SKIP1, SKIP2) and
/// the wake-up only wait for a free cycle without taking it. Cycle `$E0` is the copper's whenever
/// it asks for the bus there (`Agnus::busIsFree` marks it BLOCKED): nobody else gets it.
pub fn run_bus(
    word: &dyn Fn(u32, u16, u16) -> u16,
    start: u32,
    mut fetch: Fetch,
    bus: &mut Vec<Beam>,
) -> Vec<Write> {
    let mut writes = Vec::new();
    let free = |fetch: &Fetch, b: Beam| {
        if b.h == 0xE0 {
            return false;
        }
        if b.h % 2 == 1 && b.h != 0xE1 {
            // Refresh, disk, audio and sprite DMA own the odd cycles before the fetch window,
            // and the bitplanes most of those inside it; the copper reaches an odd cycle only
            // after being turned away at $E0.
            return b.h > 0x34 && !fetch.takes(b.line, b.h);
        }
        !fetch.takes(b.line, b.h)
    };
    // Whether the copper gets cycle `b` when it asks for it; asked at $E0, it blocks it.
    let ask = |fetch: &Fetch, b: Beam, bus: &mut Vec<Beam>| {
        if b.h == 0xE0 && !fetch.takes(b.line, b.h) {
            bus.push(b);
        }
        free(fetch, b)
    };
    let next_free = |fetch: &Fetch, mut b: Beam, bus: &mut Vec<Beam>| {
        while b.line < FRAME_LINES && !ask(fetch, b, bus) {
            b = b.plus(1);
        }
        b
    };
    let mut pc = start;
    // The vertical blank restarts the copper at COP1LC, taking cycle 0; its first fetch is two
    // cycles later.
    bus.push(Beam { line: 0, h: 0 });
    let mut b = Beam { line: 0, h: 2 };
    let mut skip = false;
    for _ in 0..8192 {
        b = next_free(&fetch, b, bus);
        if b.line >= FRAME_LINES {
            break;
        }
        let w1 = word(pc, b.line, b.h);
        bus.push(b);
        pc += 2;
        b = next_free(&fetch, b.plus(2), bus);
        if b.line >= FRAME_LINES {
            break;
        }
        let w2 = word(pc, b.line, b.h);
        bus.push(b);
        pc += 2;
        if w1 & 1 == 0 {
            let reg = w1 & 0x1FE;
            if reg < 0x40 {
                break; // an illegal register stops the copper
            }
            if !std::mem::take(&mut skip) {
                fetch.write(reg, w2);
                writes.push(Write {
                    line: b.line,
                    h: b.h,
                    reg,
                    value: w2,
                });
            }
            b = b.plus(2);
            continue;
        }
        let (comp, mask) = (w1 & 0xFFFE, (w2 & 0x7FFE) | 0x8001);
        // WAIT1 and WAIT2 (or SKIP1 and SKIP2) each wait for a free cycle.
        b = next_free(&fetch, b.plus(2), bus);
        b = next_free(&fetch, b.plus(2), bus);
        if w2 & 1 == 1 {
            skip = comparator(b.plus(2), comp, mask);
            b = b.plus(2);
            continue;
        }
        let Some(trigger) = find_match(b, comp, mask) else {
            break; // the end of the list: a wait that never comes
        };
        if trigger == b {
            b = b.plus(2);
            continue;
        }
        b = trigger;
        while b.line < FRAME_LINES
            && !(ask(&fetch, b, bus) && b.h.is_multiple_of(2) && comparator(b, comp, mask))
        {
            b = b.plus(1);
        }
        b = b.plus(2);
    }
    writes
}

/// vAmiga's `Copper::runComparator`: the vertical compare is on the line's low eight bits (bit 7
/// cannot be masked); the horizontal counter compares two cycles ahead and wraps at `$E0`.
fn comparator(b: Beam, comp: u16, mask: u16) -> bool {
    let (v, vc) = ((b.line & 0xFF) & mask >> 8, (comp & mask) >> 8);
    if v != vc {
        return v > vc;
    }
    let h = if b.h < 0xE0 { b.h + 2 } else { b.h - 0xE0 };
    (h & mask & 0xFF) >= (comp & mask & 0xFF)
}

/// vAmiga's `Copper::findMatch`: the first beam position at or after `from` where the WAIT's
/// comparator fires, or `None` for one that never does this frame.
fn find_match(from: Beam, comp: u16, mask: u16) -> Option<Beam> {
    let mut b = from;
    let vmask = (mask & 0xFF00) as u32;
    let vcomp = (comp as u32) & vmask;
    while b.line < FRAME_LINES {
        let v = b.word() & 0xFFFF & vmask;
        if v == vcomp {
            if let Some(h) = horizontal_match(b, comp, mask) {
                return Some(Beam { line: b.line, h });
            }
        } else if v > vcomp {
            return Some(b);
        }
        b = Beam {
            line: b.line + 1,
            h: 0,
        };
    }
    None
}

/// vAmiga's `Copper::findHorizontalMatch`: the counter the comparator sees runs two cycles ahead
/// of the beam and wraps for the line's last three cycles.
fn horizontal_match(b: Beam, comp: u16, mask: u16) -> Option<u16> {
    let v = (b.line as u32 & 0xFF) << 8;
    let (comp, mask) = (comp as u32 & mask as u32, mask as u32);
    let mut h = b.h;
    let mut i = b.h as u32 + 2;
    while i <= 0xE1 {
        if (v | i) & mask >= comp {
            return Some(h);
        }
        i += 1;
        h += 1;
    }
    for i in 0..=2u32 {
        if (v | i) & mask >= comp {
            return Some(h);
        }
        h += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(words: &[u16]) -> impl Fn(u32) -> u16 + '_ {
        move |a| words.get(a as usize / 2).copied().unwrap_or(0xFFFF)
    }

    #[test]
    fn moves_take_two_cycles_a_word_and_a_wait_holds_until_its_line() {
        let w = [
            0x0180, 0x0123, 0x2001, 0xFF00, 0x0182, 0x0456, 0xFFFF, 0xFFFE,
        ];
        let out = run(&list(&w), 0, Fetch::default());
        assert_eq!(
            out,
            [
                Write {
                    line: 0,
                    h: 4,
                    reg: 0x180,
                    value: 0x123
                },
                Write {
                    line: 0x20,
                    h: 4,
                    reg: 0x182,
                    value: 0x456
                },
            ]
        );
    }

    #[test]
    fn the_copper_takes_the_vblank_cycle_and_its_fetches_but_not_a_waits() {
        // The intro's scene-1 list (T89): three moves and the end. Nine cycles: 0 and the eight
        // fetches; WAIT1 and WAIT2 ($12, $14) only wait for the bus.
        let w = [
            0x0180, 0x0000, 0x0100, 0x0200, 0x0096, 0x0020, 0xFFFF, 0xFFFE,
        ];
        let mut bus = Vec::new();
        run_bus(&|a, _, _| list(&w)(a), 0, Fetch::default(), &mut bus);
        let h: Vec<u16> = bus.iter().map(|b| b.h).collect();
        assert!(bus.iter().all(|b| b.line == 0));
        assert_eq!(h, [0, 2, 4, 6, 8, 10, 12, 14, 16]);
        // WAIT $10,$DC wakes at $DA (the comparator runs two cycles ahead), taking nothing; the
        // move fetches at $DC and $DE, the next instruction asks at $E0 (blocking it) and
        // fetches at $E1.
        let w = [0x10DD, 0xFFFE, 0x0180, 0x0123, 0xFFFF, 0xFFFE];
        let mut bus = Vec::new();
        run_bus(&|a, _, _| list(&w)(a), 0, Fetch::default(), &mut bus);
        let line: Vec<u16> = bus.iter().filter(|b| b.line == 0x10).map(|b| b.h).collect();
        assert_eq!(&line[..4], &[0xDC, 0xDE, 0xE0, 0xE1]);
    }

    #[test]
    fn a_fifth_plane_slows_the_copper_inside_the_fetch_window() {
        let fetch = Fetch {
            bplcon0: 0x5200,
            diwstrt: 0x2779,
            diwstop: 0x2cc7,
            ddfstrt: 0x28,
            ddfstop: 0xd8,
        };
        // Lores, five planes: the fifth takes cycle 6 of each unit from $28, i.e. $2e, $36, ...
        assert!(fetch.takes(0x30, 0x2e) && !fetch.takes(0x30, 0x2c) && !fetch.takes(0x20, 0x2e));
        let mut w = vec![0x3001, 0xFF00];
        for k in 0..32 {
            w.extend([0x0180 + 2 * k, k]);
        }
        w.extend([0xFFFF, 0xFFFE]);
        let out = run(&list(&w), 0, fetch);
        let h: Vec<u16> = out.iter().map(|w| w.h).collect();
        // Four cycles a move before the window; from $28 one even cycle in four is taken.
        assert_eq!(&h[..10], &[4, 8, 12, 16, 20, 24, 28, 32, 36, 40]);
        assert_eq!(&h[10..13], &[44, 50, 56]);
    }

    #[test]
    fn a_wait_reached_past_cycle_e0_of_its_line_waits_for_the_next_frame() {
        // WAIT $ff,$de reached on line $ff after $E0 compares against the wrapped counter, and on
        // the lines after $ff the low eight bits are below $ff: it never fires (docs/12).
        let comp = 0xFFDE;
        let mask = 0xFFFF;
        assert!(comparator(
            Beam {
                line: 0xFF,
                h: 0xDC
            },
            comp,
            mask
        ));
        assert!(!comparator(
            Beam {
                line: 0xFF,
                h: 0xE1
            },
            comp,
            mask
        ));
        assert_eq!(
            find_match(
                Beam {
                    line: 0xFF,
                    h: 0xE1
                },
                comp,
                mask
            ),
            None
        );
        assert_eq!(
            find_match(Beam { line: 0x40, h: 0 }, comp, mask),
            Some(Beam {
                line: 0xFF,
                h: 0xDC
            })
        );
    }

    #[test]
    fn the_vertical_window_and_ocs_vstop_bit_8() {
        let f = Fetch {
            bplcon0: 0x1200,
            diwstrt: 0x2779,
            diwstop: 0x2cc7,
            ..Fetch::default()
        };
        assert_eq!(f.lines(), 0x27..0x12c);
        assert_eq!(
            Fetch {
                diwstop: 0xf0c1,
                ..f
            }
            .lines(),
            0x27..0xf0
        );
    }
}
