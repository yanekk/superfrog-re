//! The picture a frame shows (translator T24), from the frame's custom-register writes, the CPU's
//! and the copper's alike, in beam order, and chip RAM as it was when the beam fetched each line
//! (the frame's journal). Bitplanes through `amiga_chips::display`'s `Planes` and `Colours`, the
//! sprites' DMA as Agnus runs it and `amiga_chips::sprite` over the play field, colour 0 outside
//! each line's display window.
//!
//! The picture's window spans every line the frame shows: each line inside the vertical window in
//! force at its start, horizontally from the leftmost start to the rightmost stop among them. A
//! frame that reloads DIWSTRT/DIWSTOP part way down (a play area over a status bar) keeps both.
//! Hires is drawn as the oracle's lores capture keeps it (the first hires pixel of each lores one);
//! HAM and dual playfield are not drawn: the frame is refused, and the run stops.

use amiga_chips::Picture;
use amiga_chips::copper::{Fetch, Write, reg};
use amiga_chips::display::{Colours, Planes};
use amiga_chips::sprite::{self, SpriteLine};
use amiga_clock::LINE_CYCLES;

use crate::journal::History;

/// The line every sprite fetches its control words afresh on (PAL: the end of the vertical blank).
const SPRITE_RESET_LINE: u16 = 25;
/// The first line the display can show: PAL's vertical blank is lines 0 to 25, whatever the
/// window registers say (they hold anything at all as a frame starts).
const FIRST_SHOWN: u16 = 26;
const DMACON: u16 = 0x096;
const HOMOD: u16 = 0x0800;
const DBLPF: u16 = 0x0400;
/// DMACON's DMAEN with BPLEN, and with SPREN.
const BPL_DMA: u16 = 0x0300;
const SPR_DMA: u16 = 0x0220;

/// What a frame starts with and what it wrote.
pub(crate) struct FrameIn<'a> {
    pub chip: &'a [u8],
    pub history: &'a History,
    pub frame_start: i64,
    pub frame_lines: u16,
    /// The custom registers as the frame found them, by `reg / 2`.
    pub start: &'a [u16; 256],
    /// DMACON in force as the frame began.
    pub dmacon: u16,
    /// Every custom write of the frame: (cycle, register, value), in order.
    pub writes: &'a [(u64, u16, u16)],
}

impl FrameIn<'_> {
    fn word(&self, a: u32, at: i64) -> u16 {
        let off = a & (self.chip.len() as u32 - 1) & !1;
        self.history.word(self.chip, off, at)
    }
    fn byte(&self, a: u32, at: i64) -> u8 {
        let off = a & (self.chip.len() as u32 - 1);
        self.history.byte(self.chip, off, at)
    }
    fn at(&self, line: u16, h: u16) -> i64 {
        self.frame_start + crate::beam::offset(line, h) as i64
    }
}

/// One sprite channel as Agnus runs it: its pointer, control words and the line's data.
#[derive(Debug, Clone, Copy, Default)]
struct Channel {
    pt: u32,
    pos: u16,
    ctl: u16,
    shown: Option<[u16; 2]>,
}

impl Channel {
    /// The line's sprite DMA: the control words on the reset line and at the stop line, a line of
    /// data inside the window.
    fn dma(&mut self, line: u16, f: &FrameIn, at: i64) {
        let (start, stop) = sprite::lines(self.pos, self.ctl);
        self.shown = None;
        if line == SPRITE_RESET_LINE || (line > SPRITE_RESET_LINE && line == stop) {
            self.pos = f.word(self.pt, at);
            self.ctl = f.word(self.pt + 2, at);
            self.pt += 4;
        } else if line > SPRITE_RESET_LINE && line >= start && line < stop {
            self.shown = Some([f.word(self.pt, at), f.word(self.pt + 2, at)]);
            self.pt += 4;
        }
    }

    fn line(&self) -> Option<SpriteLine> {
        self.shown.map(|data| SpriteLine {
            hstart: sprite::hstart(self.pos, self.ctl),
            data,
            attached: self.ctl & 0x80 != 0,
            cut: None,
        })
    }
}

fn fetch_of(r: &[u16; 256]) -> Fetch {
    let g = |reg: u16| r[reg as usize >> 1];
    Fetch {
        bplcon0: g(reg::BPLCON0),
        diwstrt: g(reg::DIWSTRT),
        diwstop: g(reg::DIWSTOP),
        ddfstrt: g(reg::DDFSTRT),
        ddfstop: g(reg::DDFSTOP),
    }
}

/// The frame's writes as beam positions, those inside the frame.
fn beam_writes(f: &FrameIn) -> Vec<Write> {
    let len = f.frame_lines as i64 * LINE_CYCLES as i64;
    f.writes
        .iter()
        .filter_map(|&(at, reg, value)| {
            let d = at as i64 - f.frame_start;
            (0..len).contains(&d).then(|| Write {
                line: (d / LINE_CYCLES as i64) as u16,
                h: (d % LINE_CYCLES as i64 / 2) as u16,
                reg,
                value,
            })
        })
        .collect()
}

/// The window every line shown falls in, as (left, top, right, bottom), right and bottom
/// exclusive; `None` if no line is inside its vertical window.
fn spanning_window(start: Fetch, writes: &[Write], lines: u16) -> Option<(i32, u16, i32, u16)> {
    let mut f = start;
    let mut next = 0;
    let mut span: Option<(i32, u16, i32, u16)> = None;
    for line in 0..lines {
        while let Some(w) = writes.get(next).filter(|w| w.line < line) {
            f.write(w.reg, w.value);
            next += 1;
        }
        if line < FIRST_SHOWN || !f.lines().contains(&line) {
            continue;
        }
        let (l, r) = ((f.diwstrt & 0xFF) as i32, (f.diwstop & 0xFF | 0x100) as i32);
        span = Some(match span {
            None => (l, line, r, line + 1),
            Some((sl, st, sr, _)) => (sl.min(l), st, sr.max(r), line + 1),
        });
    }
    span
}

/// The frame's picture, or the mode it cannot draw.
pub(crate) fn draw(f: &FrameIn) -> Result<Picture, &'static str> {
    let writes = beam_writes(f);
    let start = fetch_of(f.start);
    for con0 in std::iter::once(start.bplcon0).chain(
        writes
            .iter()
            .filter(|w| w.reg == reg::BPLCON0)
            .map(|w| w.value),
    ) {
        if con0 & HOMOD != 0 {
            return Err("display ham");
        }
        if con0 & DBLPF != 0 {
            return Err("display dual playfield");
        }
    }
    let Some((left, top, right, bottom)) = spanning_window(start, &writes, f.frame_lines) else {
        return Ok(Picture::blank(0, 0, 0, 0));
    };
    let mut pic = Picture::blank(left, top, (right - left) as u16, bottom - top);
    let w = pic.width as usize;

    let mut planes = Planes::default();
    for r in [
        reg::BPLCON0,
        reg::BPLCON1,
        reg::BPLCON2,
        reg::BPL1MOD,
        reg::BPL2MOD,
        reg::DIWSTRT,
        reg::DIWSTOP,
        reg::DDFSTRT,
        reg::DDFSTOP,
    ]
    .into_iter()
    .chain((reg::BPL1PTH..reg::BPL1PTH + 24).step_by(2))
    {
        planes.write(r, f.start[r as usize >> 1]);
    }
    let mut colours = Colours::default();
    for (i, c) in colours.regs.iter_mut().enumerate() {
        *c = f.start[(reg::COLOR00 as usize >> 1) + i] & 0xFFF;
    }
    let mut channels: [Channel; 8] = std::array::from_fn(|n| {
        let g = |r: u16| f.start[r as usize >> 1];
        Channel {
            pt: (g(reg::SPR0PTH + 4 * n as u16) as u32 & 0x1F) << 16
                | g(reg::SPR0PTH + 4 * n as u16 + 2) as u32 & 0xFFFE,
            pos: g(reg::SPR0POS + 8 * n as u16),
            ctl: g(reg::SPR0POS + 8 * n as u16 + 2),
            shown: None,
        }
    });
    let mut dmacon = f.dmacon;
    let mut next = 0;
    // Apply the writes before beam position (`line`, `h`).
    let mut apply = |line: u16,
                     h: u16,
                     y: Option<usize>,
                     planes: &mut Planes,
                     colours: &mut Colours,
                     channels: &mut [Channel; 8],
                     dmacon: &mut u16,
                     pic: &mut Picture| {
        while let Some(wr) = writes.get(next) {
            if (wr.line, wr.h) >= (line, h) {
                break;
            }
            next += 1;
            if colours.apply(wr, y, pic) {
                continue;
            }
            if wr.reg == DMACON {
                *dmacon = crate::irq::set_clear(*dmacon, wr.value);
            } else if (reg::SPR0PTH..reg::SPR0PTH + 32).contains(&wr.reg) {
                let c = &mut channels[((wr.reg - reg::SPR0PTH) / 4) as usize];
                c.pt = if wr.reg & 2 == 0 {
                    (wr.value as u32 & 0x1F) << 16 | c.pt & 0xFFFF
                } else {
                    c.pt & 0xFFFF_0000 | wr.value as u32 & 0xFFFE
                };
            } else {
                planes.write(wr.reg, wr.value);
            }
        }
    };
    for line in 0..bottom {
        let y = line.checked_sub(top).map(|y| y as usize);
        let first = planes.fetch.first_cycle();
        apply(
            line,
            first,
            y,
            &mut planes,
            &mut colours,
            &mut channels,
            &mut dmacon,
            &mut pic,
        );
        let at = f.at(line, first);
        if dmacon & SPR_DMA == SPR_DMA {
            for c in &mut channels {
                c.dma(line, f, at);
            }
        } else {
            for c in &mut channels {
                c.shown = None;
            }
        }
        let fetching = dmacon & BPL_DMA == BPL_DMA && planes.fetching(line);
        if let Some(y) = y {
            let row = &mut pic.pixels[y * w..(y + 1) * w];
            if fetching {
                planes.fetch_line(left, row, |_, addr, len| {
                    (addr..addr + len)
                        .map(|a| f.byte(a, at))
                        .collect::<Vec<u8>>()
                });
            }
            let lines: [Option<SpriteLine>; 8] = std::array::from_fn(|n| channels[n].line());
            if lines.iter().any(Option::is_some) {
                let priority = planes.priority() as usize;
                for (x, px) in row.iter_mut().enumerate() {
                    if let Some((c, pair)) = sprite::sprite_at(&lines, left + x as i32, x as i32)
                        && (pair < priority || *px == 0)
                    {
                        *px = c;
                    }
                }
            }
            // Denise shows colour 0 outside the window in force on the line, sprites included.
            let fw = &planes.fetch;
            let shown = line >= FIRST_SHOWN && fw.lines().contains(&line);
            let (hstart, hstop) = (
                (fw.diwstrt & 0xFF) as i32,
                (fw.diwstop & 0xFF | 0x100) as i32,
            );
            for (x, px) in row.iter_mut().enumerate() {
                let p = left + x as i32;
                if !shown || p < hstart || p >= hstop {
                    *px = 0;
                }
            }
        }
        if fetching {
            planes.advance();
        }
        apply(
            line + 1,
            0,
            y,
            &mut planes,
            &mut colours,
            &mut channels,
            &mut dmacon,
            &mut pic,
        );
        if let Some(y) = y {
            pic.palettes[y] = colours.end_line();
        }
    }
    Ok(pic)
}
