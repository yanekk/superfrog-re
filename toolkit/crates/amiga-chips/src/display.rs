//! Denise's side of a lores frame: the bitplane registers as the copper leaves them, which fetched
//! bit each window pixel shows, and the colour registers through the frame ([`Picture`]'s palettes
//! and [`Split`]s). A game's renderer builds on these pieces; [`draw`] is the plain frame they make
//! from chip memory and the copper's writes alone, with no sprites.

use std::borrow::Borrow;

use crate::copper::{Fetch, Write, reg};
use crate::memory::Ram;
use crate::picture::{Picture, Split};

/// Lores pixels from a line's first fetched bit to its display (Denise's pipeline): the fetch
/// index of window pixel `x` is `left + x - (2 * DDFSTRT + 17) - delay`.
/// learned on: Superfrog's level and panel against the vAmiga oracle (its docs/12, "Fetched lines").
pub const PIPELINE: i32 = 17;
/// The hires counterpart of [`PIPELINE`], in hires pixels (see [`Planes::index_hires`]).
/// learned on: the Team 17 logo screen (hires) against the vAmiga oracle.
pub const HIRES_PIPELINE: i32 = 2;

/// BPLCON0's HAM bit.
const HOMOD: u16 = 0x0800;
/// BPLCON0's dual-playfield bit.
const DBLPF: u16 = 0x0400;

/// Bitplane registers as the copper leaves them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Planes {
    pub fetch: Fetch,
    pub bplcon1: u16,
    pub bplcon2: u16,
    pub modulo: [i16; 2],
    pub pt: [u32; 6],
}

impl Planes {
    pub fn write(&mut self, reg: u16, value: u16) {
        self.fetch.write(reg, value);
        match reg {
            reg::BPLCON1 => self.bplcon1 = value,
            reg::BPLCON2 => self.bplcon2 = value,
            reg::BPL1MOD => self.modulo[0] = value as i16,
            reg::BPL2MOD => self.modulo[1] = value as i16,
            r if (reg::BPL1PTH..reg::BPL1PTH + 24).contains(&r) => {
                let p = ((r - reg::BPL1PTH) / 4) as usize;
                self.pt[p] = if r & 2 == 0 {
                    (value as u32 & 0x1F) << 16 | self.pt[p] & 0xFFFF
                } else {
                    self.pt[p] & 0xFFFF_0000 | value as u32 & 0xFFFE
                };
            }
            _ => {}
        }
    }

    /// Plane `p`'s scroll delay in lores pixels: odd planes (index 0, 2, 4) use PF1's.
    pub fn delay(&self, p: usize) -> i32 {
        (if p.is_multiple_of(2) {
            self.bplcon1
        } else {
            self.bplcon1 >> 4
        } & 15) as i32
    }

    /// The fetch index (lores pixel of the line's fetched data) plane `p` shows at window
    /// position `left + x`.
    pub fn index(&self, p: usize, left: i32) -> i32 {
        left - (2 * self.fetch.first_cycle() as i32 + PIPELINE) - self.delay(p)
    }

    /// In hires, the fetched hires pixel the first half of lores position `left` shows: a hires
    /// fetch reaches the screen 16 hires pixels (8 lores) sooner after its first cycle than a lores
    /// one (pixel 0 at `$81` with DDFSTRT `$3c`, against `$38` in lores); the delay counts in lores
    /// pixels.
    pub fn index_hires(&self, p: usize, left: i32) -> i32 {
        2 * left - 4 * self.fetch.first_cycle() as i32 - HIRES_PIPELINE - 2 * self.delay(p)
    }

    /// The play field's priority against the sprite pairs: in single-playfield mode Denise
    /// uses PF2P (BPLCON2 bits 3-5); pairs below it are in front.
    pub fn priority(&self) -> u16 {
        self.bplcon2 >> 3 & 7
    }

    /// Whether `line` fetches bitplanes.
    pub fn fetching(&self, line: u16) -> bool {
        self.fetch.lines().contains(&line) && self.fetch.planes() > 0
    }

    /// The pointers after a fetched line: each plane on by its words and its playfield's modulo
    /// (BPL1MOD for odd planes, index 0, 2, 4).
    pub fn advance(&mut self) {
        let words = self.fetch.words() as i32;
        for p in 0..self.fetch.planes() as usize {
            let m = self.modulo[p % 2] as i32;
            self.pt[p] = (self.pt[p] as i32 + 2 * words + m) as u32;
        }
    }

    /// The colour indices of one fetched line into `row`, whose pixel 0 is window position
    /// `left`. `bytes(p, addr, len)` reads plane `p`'s words of the line (a game whose memory
    /// changes while the beam draws reads them as the beam sees them).
    pub fn fetch_line<B: Borrow<[u8]>>(
        &self,
        left: i32,
        row: &mut [u8],
        mut bytes: impl FnMut(usize, u32, u32) -> B,
    ) {
        row.fill(0);
        let words = self.fetch.words() as i32;
        let hires = self.fetch.hires();
        for p in 0..self.fetch.planes() as usize {
            let data = bytes(p, self.pt[p], 2 * words as u32);
            let data = data.borrow();
            let start = self.index(p, left);
            for (x, px) in row.iter_mut().enumerate() {
                // Hires: a lores pixel shows the first of its two hires pixels, as the capture keeps.
                let i = if hires {
                    self.index_hires(p, left) + 2 * x as i32
                } else {
                    start + x as i32
                };
                if (0..16 * words).contains(&i) {
                    let (byte, bit) = ((i / 8) as usize, 7 - i % 8);
                    *px |= (data[byte] >> bit & 1) << p;
                }
            }
        }
    }
}

/// The colour registers through a frame, and the palette of the picture line being drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Colours {
    /// COLOR00–31 as the writes so far leave them.
    pub regs: [u16; 32],
    /// The line's palette, taken just before its first split; `None` while it has none.
    pub line: Option<[u16; 32]>,
}

impl Colours {
    /// Apply `w` if it is a colour write: from now on the register holds it, and a write inside
    /// picture line `y`'s pixels (lores position `2h`) becomes a [`Split`] of `pic`. Returns
    /// whether it was one.
    pub fn apply(&mut self, w: &Write, y: Option<usize>, pic: &mut Picture) -> bool {
        if !(reg::COLOR00..reg::COLOR00 + 64).contains(&w.reg) {
            return false;
        }
        let index = ((w.reg - reg::COLOR00) / 2) as u8;
        let x = 2 * w.h as i32 - pic.left;
        let on_line = y.filter(|&y| w.line == pic.top + y as u16);
        if let Some(y) = on_line.filter(|_| x > 0) {
            self.line.get_or_insert(self.regs);
            if x < pic.width as i32 {
                pic.splits.push(Split {
                    y: y as u16,
                    x: x as u16,
                    index,
                    colour: w.value & 0xFFF,
                });
            }
        }
        self.regs[index as usize] = w.value & 0xFFF;
        true
    }

    /// The finished line's palette, and a fresh start for the next.
    pub fn end_line(&mut self) -> [u16; 32] {
        self.line.take().unwrap_or(self.regs)
    }
}

/// The display window a frame's register state gives: DIWSTRT/DIWSTOP's horizontal and
/// vertical extent, every pixel colour 0.
pub fn window(fetch: &Fetch) -> Picture {
    let (hstart, hstop) = (fetch.diwstrt & 0xFF, fetch.diwstop & 0xFF | 0x100);
    let lines = fetch.lines();
    Picture::blank(
        hstart as i32,
        lines.start,
        hstop.saturating_sub(hstart),
        lines.end.saturating_sub(lines.start),
    )
}

/// A lores frame as Denise shows it, from `mem` and the copper's `writes` for the frame (in beam
/// order, as [`crate::copper::run`] gives them), the frame starting with the bitplane setup
/// `start`. Follows BPLCON0's plane count (six planes are extra-half-brite), BPL1PT–BPL6PT,
/// BPL1MOD/BPL2MOD, BPLCON1's delay and COLOR00–31; the window is the one the writes leave at the
/// frame's end. Sprites are not drawn and the bitplane pointers start at 0 until a write sets them.
///
/// # Panics
///
/// If the frame turns on hires, HAM or dual playfield, which this does not draw.
pub fn draw(mem: &impl Ram, writes: &[Write], start: Fetch) -> Picture {
    let lores = |v: u16| {
        assert!(
            v & 0x8000 == 0,
            "display::draw draws lores only: BPLCON0 {v:#06x} sets HIRES"
        );
        assert!(
            v & HOMOD == 0,
            "display::draw does not draw HAM: BPLCON0 {v:#06x} sets HOMOD"
        );
        assert!(
            v & DBLPF == 0,
            "display::draw does not draw dual playfield: BPLCON0 {v:#06x} sets DBLPF"
        );
    };
    lores(start.bplcon0);
    let mut end = start;
    for w in writes {
        if w.reg == reg::BPLCON0 {
            lores(w.value);
        }
        end.write(w.reg, w.value);
    }
    let mut pic = window(&end);
    let (left, width) = (pic.left, pic.width as usize);
    let mut planes = Planes {
        fetch: start,
        ..Planes::default()
    };
    let mut colours = Colours::default();
    let mut next = 0;
    // Apply the writes before beam position (`line`, `h`).
    let mut apply = |line: u16,
                     h: u16,
                     y: Option<usize>,
                     planes: &mut Planes,
                     colours: &mut Colours,
                     pic: &mut Picture| {
        while let Some(w) = writes.get(next) {
            if (w.line, w.h) >= (line, h) {
                break;
            }
            next += 1;
            if !colours.apply(w, y, pic) {
                planes.write(w.reg, w.value);
            }
        }
    };
    for line in 0..pic.top + pic.height {
        let y = line.checked_sub(pic.top).map(|y| y as usize);
        let first = planes.fetch.first_cycle();
        apply(line, first, y, &mut planes, &mut colours, &mut pic);
        if planes.fetching(line) {
            if let Some(y) = y {
                let row = &mut pic.pixels[y * width..(y + 1) * width];
                planes.fetch_line(left, row, |_, addr, len| {
                    (addr..addr + len).map(|a| mem.byte(a)).collect::<Vec<u8>>()
                });
            }
            planes.advance();
        }
        apply(line + 1, 0, y, &mut planes, &mut colours, &mut pic);
        if let Some(y) = y {
            pic.palettes[y] = colours.end_line();
        }
    }
    pic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::AmigaMemory;

    /// A 320-pixel window at `$81`, `$2c` to line `$130` (VSTOP `$30`'s bit 8 is set), DDFSTRT `$38`
    /// (20 words a line): the usual lores
    /// screen, `planes` planes.
    fn screen(planes: u16) -> Fetch {
        Fetch {
            bplcon0: planes << 12 | 0x200,
            diwstrt: 0x2c81,
            diwstop: 0x30c1,
            ddfstrt: 0x38,
            ddfstop: 0xd0,
        }
    }

    fn mov(line: u16, h: u16, reg: u16, value: u16) -> Write {
        Write {
            line,
            h,
            reg,
            value,
        }
    }

    /// Writes in the vertical blank pointing plane `p` (0-based) at `addr`.
    fn pt(p: usize, addr: u32) -> [Write; 2] {
        let r = reg::BPL1PTH + 4 * p as u16;
        [
            mov(0, 4, r, (addr >> 16) as u16),
            mov(0, 6, r + 2, addr as u16),
        ]
    }

    fn chip(len: usize) -> AmigaMemory {
        AmigaMemory {
            chip: vec![0; len],
            slow: Vec::new(),
        }
    }

    fn row(p: &Picture, y: usize) -> &[u8] {
        &p.pixels[y * p.width as usize..(y + 1) * p.width as usize]
    }

    #[test]
    fn the_window_is_the_one_the_registers_leave() {
        let p = draw(&chip(0), &[], screen(1));
        assert_eq!(
            (p.left, p.top, p.width, p.height),
            (0x81, 0x2c, 320, 0x130 - 0x2c)
        );
    }

    #[test]
    fn one_plane_shows_its_bits() {
        let mut m = chip(0x1000);
        let pattern = [0b1010_0000u8, 0xFF, 0x0F, 0x81];
        m.chip[0x100..0x104].copy_from_slice(&pattern);
        let p = draw(&m, &pt(0, 0x100), screen(1));
        let want: Vec<u8> = pattern
            .iter()
            .flat_map(|b| (0..8).rev().map(move |i| b >> i & 1))
            .collect();
        assert_eq!(&row(&p, 0)[..32], &want[..]);
        assert!(row(&p, 0)[32..].iter().all(|&i| i == 0));
    }

    #[test]
    fn five_planes_combine_in_order() {
        let mut m = chip(0x10000);
        let mut writes = Vec::new();
        for plane in 0..5 {
            let base = 0x1000 * (plane as u32 + 1);
            writes.extend(pt(plane, base));
            // Pixel x has plane `plane` set when bit `plane` of x is set, for x in 0..32.
            for x in 0..32u32 {
                if x >> plane & 1 == 1 {
                    m.chip[(base + x / 8) as usize] |= 0x80 >> (x % 8);
                }
            }
        }
        writes.sort_by_key(|w| (w.line, w.h));
        let p = draw(&m, &writes, screen(5));
        let want: Vec<u8> = (0..32).collect();
        assert_eq!(&row(&p, 0)[..32], &want[..]);
    }

    #[test]
    fn six_planes_are_extra_half_brite() {
        let mut m = chip(0x10000);
        let mut writes = Vec::new();
        for plane in 0..6 {
            writes.extend(pt(plane, 0x1000 * (plane as u32 + 1)));
        }
        // Plane 6 and plane 1 set on pixel 0: index 33; plane 6 alone on pixel 1: index 32.
        m.chip[0x1000] = 0x80;
        m.chip[0x6000] = 0xC0;
        writes.sort_by_key(|w| (w.line, w.h));
        let mut p = draw(&m, &writes, screen(6));
        assert_eq!(&row(&p, 0)[..3], &[33, 32, 0]);
        p.palettes[0][1] = 0xEEE;
        assert_eq!(p.rgb12(0, 0), 0x777);
    }

    #[test]
    fn the_modulo_moves_the_next_line_on() {
        let mut m = chip(0x1000);
        // 20 words a line, modulo 8: line 1 starts 48 bytes on.
        m.chip[0x100] = 0x80;
        m.chip[0x100 + 48] = 0x40;
        m.chip[0x100 + 40] = 0x20;
        let mut writes = pt(0, 0x100).to_vec();
        writes.push(mov(0, 8, reg::BPL1MOD, 8));
        let p = draw(&m, &writes, screen(1));
        assert_eq!(&row(&p, 0)[..3], &[1, 0, 0]);
        assert_eq!(&row(&p, 1)[..3], &[0, 1, 0]);
    }

    #[test]
    fn even_planes_take_bpl2mod() {
        let mut m = chip(0x1000);
        // Plane 1 (odd) with BPL1MOD 8 starts line 1 48 bytes on; plane 2 (even) with BPL2MOD -4,
        // 36 bytes on.
        m.chip[0x100 + 48] = 0x80;
        m.chip[0x400 + 36] = 0x40;
        let mut writes = pt(0, 0x100).to_vec();
        writes.extend(pt(1, 0x400));
        writes.push(mov(0, 8, reg::BPL1MOD, 8));
        writes.push(mov(0, 10, reg::BPL2MOD, -4i16 as u16));
        writes.sort_by_key(|w| (w.line, w.h));
        let p = draw(&m, &writes, screen(2));
        assert_eq!(&row(&p, 1)[..3], &[1, 2, 0]);
    }

    #[test]
    fn bplcon1_delays_the_odd_planes() {
        let mut m = chip(0x1000);
        m.chip[0x100] = 0x80;
        let mut writes = pt(0, 0x100).to_vec();
        writes.push(mov(0, 8, reg::BPLCON1, 0x03));
        let p = draw(&m, &writes, screen(1));
        assert_eq!(&row(&p, 0)[..5], &[0, 0, 0, 1, 0]);
    }

    #[test]
    fn a_colour_write_changes_its_line_on_and_mid_line_is_a_split() {
        let m = chip(0);
        let f = screen(1);
        let line = |y: u16| 0x2c + y;
        // In the border before line 10 (h $10, before the window's left edge): its palette.
        // Inside line 12 at h $90: a split at x = 2 * $90 - $81.
        let writes = [
            mov(0, 2, reg::COLOR00, 0x111),
            mov(line(10), 0x10, reg::COLOR00 + 2, 0xF00),
            mov(line(12), 0x90, reg::COLOR00 + 4, 0x0F0),
        ];
        let p = draw(&m, &writes, f);
        assert_eq!(p.palettes[9][1], 0);
        assert_eq!(p.palettes[10][1], 0xF00);
        assert_eq!(p.palettes[11][0], 0x111);
        assert_eq!(
            p.palettes[12][2], 0,
            "the split's line palette is before the write"
        );
        assert_eq!(p.palettes[13][2], 0x0F0);
        assert_eq!(
            p.splits,
            [Split {
                y: 12,
                x: 2 * 0x90 - 0x81,
                index: 2,
                colour: 0x0F0
            }]
        );
    }

    #[test]
    fn a_pointer_rewritten_mid_frame_moves_the_fetch_from_that_line_on() {
        let mut m = chip(0x1000);
        m.chip[0x100..0x100 + 4 * 40].fill(0x80);
        m.chip[0x800..0x800 + 4 * 40].fill(0x40);
        let mut writes = pt(0, 0x100).to_vec();
        // Between line 1 and line 2 (raster $2e, before the fetch window).
        writes.push(mov(0x2e, 0x10, reg::BPL1PTH, 0));
        writes.push(mov(0x2e, 0x12, reg::BPL1PTH + 2, 0x800));
        let p = draw(&m, &writes, screen(1));
        assert_eq!(&row(&p, 1)[..2], &[1, 0]);
        assert_eq!(&row(&p, 2)[..2], &[0, 1]);
        assert_eq!(
            &row(&p, 3)[..2],
            &[0, 1],
            "and goes on from the new pointer"
        );
    }

    #[test]
    #[should_panic(expected = "lores only")]
    fn hires_panics() {
        draw(
            &chip(0),
            &[],
            Fetch {
                bplcon0: 0x9200,
                ..screen(1)
            },
        );
    }

    #[test]
    #[should_panic(expected = "HAM")]
    fn ham_panics() {
        draw(&chip(0), &[mov(0x30, 0, reg::BPLCON0, 0x6a00)], screen(1));
    }

    #[test]
    #[should_panic(expected = "dual playfield")]
    fn dual_playfield_panics() {
        draw(&chip(0), &[mov(0, 2, reg::BPLCON0, 0x2600)], screen(1));
    }
}
