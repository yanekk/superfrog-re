//! CLXDAT, the hardware's sprite against playfield collision, as the frame the engine draws makes it
//! (plans/forest-dumb-level-1 T70; docs/10-object-system.md, "Contact").
//!
//! Contact in the original is not a box test: `char_hit_check` reads CLXDAT, which Denise sets while
//! it draws a frame wherever a sprite pixel lies over a playfield pixel matching CLXCON, and which a
//! read clears. The level copper list sets CLXCON to `$37dd`: sprites 1 and 3 OR'd with 0 and 2,
//! planes 1 to 5 compared with 1, 0, 1, 1, 1, so only colour 29 matches the whole set.
//!
//! The model is the oracle's (vAmiga, `Denise::checkS2PCollisions`, collision on since T59), which
//! the engine is judged against: per sprite pixel inside the display window,
//!
//! - the even planes compared (PF2) set bit `5 + n/2`; in single-playfield mode a pixel whose even
//!   planes do not match is not compared further ("a hardware oddity", vAmiga's note), so
//! - the odd planes (PF1) set bit `1 + n/2` only where the even ones matched too;
//! - an odd sprite counts only with its ENSP bit; an attached pair's pixel counts as its odd
//!   sprite's, and only where the pair is drawn in front of the playfield (vAmiga marks the depth
//!   buffer only then); an unattached sprite's pixel counts wherever it is not transparent;
//! - bit 15 always reads set (the oracle read `$8000` before collision was on).
//!
//! Playfield against playfield (bit 0, `Denise::checkP2PCollisions`) is on in the oracle: any window
//! pixel matching both playfields' comparison, colour 29 here, sets it. Sprite against sprite (bits 9
//! to 14) is not modelled: the character's two pairs sit side by side and sprites 4 to 7 are empty,
//! so they never overlap.
//!
//! Which frame a pass's read sees is measured (T70, `fd-conform/tests/contact.rs`): the one the
//! display showed since the last pass, which is what [`crate::Game::picture`] draws after it.

use amiga_chips::copper::reg;
use amiga_chips::display::Planes;
use amiga_chips::sprite::{self, SPRITE_DELAY};

use super::{Channel, Chip, Display, DisplayData, LEVEL_COPPER, WINDOW, frame_writes};
use crate::program::{Program, h};

/// CLXCON's register offset.
const CLXCON: u16 = 0x098;
/// BPLCON0's dual-playfield bit.
const DBLPF: u16 = 0x0400;
/// What CLXDAT reads with no collision.
pub const CLXDAT_NONE: u16 = 0x8000;
/// The bits `char_hit_check` tests: odd planes against sprites 0/1 and 2/3.
pub const CONTACT: u16 = 0b110;
/// Bit 0: the odd planes against the even ones.
const PLAYFIELDS: u16 = 1;

/// CLXCON's comparison, split by playfield as Denise splits it.
#[derive(Debug, Clone, Copy)]
struct Compare {
    con: u16,
    dual: bool,
}

impl Compare {
    /// ENSP for an odd sprite `n`: whether it is OR'd into its pair's collisions.
    fn counts(&self, n: usize) -> bool {
        n % 2 == 0 || self.con >> (12 + n / 2) & 1 != 0
    }

    /// Whether playfield bits `px` match on both playfields (`Denise::checkP2PCollisions`).
    fn both(&self, px: u8) -> bool {
        let enabled = (self.con >> 6) as u8;
        let (en1, en2) = (enabled & 0b01_0101, enabled & 0b10_1010);
        px & en1 == self.con as u8 & en1 && px & en2 == self.con as u8 & en2
    }

    /// The bits a pixel of sprite `n` over playfield bits `px` sets.
    fn hit(&self, n: usize, px: u8) -> u16 {
        let enabled = (self.con >> 6) as u8;
        let (en1, en2) = (enabled & 0b01_0101, enabled & 0b10_1010);
        let (mv1, mv2) = (self.con as u8 & en1, self.con as u8 & en2);
        let mut bits = 0;
        if px & en2 == mv2 {
            bits |= 1 << (5 + n / 2);
        } else if !self.dual {
            return 0;
        }
        if px & en1 == mv1 {
            bits |= 1 << (1 + n / 2);
        }
        bits
    }
}

/// The sprites of pair `p` that mark a collision at window position `at` over playfield bits `px`,
/// with the playfield's priority against the pairs: as vAmiga marks its depth buffer.
fn marked(
    lines: &[Option<sprite::SpriteLine>; 8],
    p: usize,
    at: i32,
    px: u8,
    priority: usize,
) -> [bool; 2] {
    let bits = |n: usize| {
        let s = lines[n]?;
        let i = at - s.hstart as i32 - SPRITE_DELAY;
        (0..16).contains(&i).then(|| {
            let b = 15 - i;
            (s.data[0] >> b & 1 | (s.data[1] >> b & 1) << 1) as u8
        })
    };
    let (even, odd) = (bits(2 * p).unwrap_or(0), bits(2 * p + 1).unwrap_or(0));
    if lines[2 * p + 1].is_some_and(|s| s.attached) {
        let shown = (even | odd << 2) != 0 && (p < priority || px == 0);
        [false, shown]
    } else {
        [even != 0, odd != 0]
    }
}

/// One fetched line inside the window, as the beam leaves the registers there: what the playfield
/// against playfield check needs once the sprites are done.
#[derive(Debug, Clone, Copy)]
struct Fetched {
    planes: Planes,
    cmp: Compare,
}

/// Bytes a plane fetches on a line, at most: DDFSTRT to DDFSTOP's widest, overscan included.
const LINE_BYTES: usize = 64;

/// One line's fetched planes, read once, and the playfield bits at a window position from them, as
/// `Planes::fetch_line` reads them (a hires lores pixel shows its first hires pixel).
struct Line {
    planes: Planes,
    data: [[u8; LINE_BYTES]; 6],
    /// Planes fetched, and the fetched bits a plane holds.
    n: usize,
    bits: i32,
    /// Plane `p`'s fetch index at window position `at` is `scale * at + base[p]`
    /// (`Planes::index`, `Planes::index_hires`), worked out once a line.
    scale: i32,
    base: [i32; 6],
}

impl Line {
    fn new(planes: &Planes, chip: &Chip) -> Line {
        let mut data = [[0; LINE_BYTES]; 6];
        let n = (2 * planes.fetch.words() as usize).min(LINE_BYTES);
        for (p, d) in data
            .iter_mut()
            .enumerate()
            .take(planes.fetch.planes() as usize)
        {
            chip.read(planes.pt[p], &mut d[..n]);
        }
        Line::of(planes, data)
    }

    fn of(planes: &Planes, data: [[u8; LINE_BYTES]; 6]) -> Line {
        let hires = planes.fetch.hires();
        Line {
            planes: *planes,
            data,
            n: planes.fetch.planes() as usize,
            bits: (16 * planes.fetch.words() as i32).min(8 * LINE_BYTES as i32),
            scale: if hires { 2 } else { 1 },
            base: std::array::from_fn(|p| {
                if hires {
                    planes.index_hires(p, 0)
                } else {
                    planes.index(p, 0)
                }
            }),
        }
    }

    /// Whether a pixel at window positions `from..to` matches both playfields. Eight pixels a byte
    /// when every plane shows the same fetched pixel at a position (lores, one delay), else each.
    fn any_both(&self, cmp: &Compare, from: i32, to: i32) -> bool {
        let pl = &self.planes;
        let n = pl.fetch.planes() as usize;
        if pl.fetch.hires() || (0..n).any(|p| pl.delay(p) != pl.delay(0)) {
            return (from..to).any(|at| cmp.both(self.at(at)));
        }
        let bits = (16 * pl.fetch.words() as i32).min(8 * LINE_BYTES as i32);
        let (lo, hi) = (pl.index(0, from).max(0), pl.index(0, to).min(bits));
        let enabled = (cmp.con >> 6) as u8 & 0x3f;
        // A plane not fetched reads 0.
        let wanted = |p: usize, byte: u8| if p < n { byte } else { 0 };
        let mut i = lo;
        while i < hi {
            let k = (i / 8) as usize;
            let mut m = 0xffu8;
            for p in (0..6).filter(|p| enabled >> p & 1 != 0) {
                let b = wanted(p, self.data.get(p).map_or(0, |d| d[k]));
                m &= if cmp.con >> p & 1 != 0 { b } else { !b };
            }
            let first = (i % 8) as u32;
            let last = (hi - 8 * k as i32).min(8) as u32;
            let span = (0xffu16 >> first) as u8 & !((0xffu16 >> last) as u8);
            if m & span != 0 {
                return true;
            }
            i = 8 * (k as i32 + 1);
        }
        false
    }

    fn at(&self, at: i32) -> u8 {
        let mut px = 0;
        for p in 0..self.n {
            let i = self.scale * at + self.base[p];
            if (0..self.bits).contains(&i) {
                px |= (self.data[p][(i / 8) as usize] >> (7 - i % 8) & 1) << p;
            }
        }
        px
    }
}

/// The line's window, left and right, in window positions.
fn window(planes: &Planes) -> (i32, i32) {
    let f = &planes.fetch;
    ((f.diwstrt & 0xff) as i32, (f.diwstop & 0xff | 0x100) as i32)
}

/// CLXDAT after the frame the display shows with the camera at `camera`, read from its last clear
/// at the frame's start.
pub fn clxdat(data: &DisplayData, display: &Display, camera: (u16, u16), program: &Program) -> u16 {
    let chip = Chip::new(data, display, camera, program);
    let (writes, start) = frame_writes(&chip, h(1, LEVEL_COPPER));
    let (diwstrt, diwstop) = WINDOW;
    let top = diwstrt >> 8;
    let left = (diwstrt & 0xff) as i32;
    let stop = diwstop >> 8 | (!diwstop >> 7 & 0x100);
    let right = ((diwstop & 0xff) | 0x100) as i32;
    let mut planes = Planes {
        fetch: start,
        ..Planes::default()
    };
    let mut clxcon = 0u16;
    let mut bplcon0 = 0u16;
    let mut channels = [Channel::default(); 8];
    let mut next = 0;
    let mut clx = CLXDAT_NONE;
    let mut fetched = Vec::new();
    // The copper's writes before beam position (`line`, `h`), as `draw` applies them.
    let mut apply = |line: u16,
                     hpos: u16,
                     planes: &mut Planes,
                     channels: &mut [Channel; 8],
                     clxcon: &mut u16,
                     bplcon0: &mut u16| {
        while let Some(wr) = writes.get(next) {
            if (wr.line, wr.h) >= (line, hpos) {
                break;
            }
            next += 1;
            match wr.reg {
                CLXCON => *clxcon = wr.value,
                r if (reg::SPR0PTH..reg::SPR0PTH + 32).contains(&r) => {
                    let c = &mut channels[((r - reg::SPR0PTH) / 4) as usize];
                    c.pt = if r & 2 == 0 {
                        (wr.value as u32 & 0x1f) << 16 | c.pt & 0xffff
                    } else {
                        c.pt & 0xffff_0000 | wr.value as u32 & 0xfffe
                    };
                }
                r => {
                    if r == reg::BPLCON0 {
                        *bplcon0 = wr.value;
                    }
                    planes.write(r, wr.value);
                }
            }
        }
    };
    for line in 0..stop {
        let first = planes.fetch.first_cycle();
        apply(
            line,
            first,
            &mut planes,
            &mut channels,
            &mut clxcon,
            &mut bplcon0,
        );
        for c in &mut channels {
            c.dma(line, &chip);
        }
        let fetching = planes.fetching(line);
        let cmp = Compare {
            con: clxcon,
            dual: bplcon0 & DBLPF != 0,
        };
        if line >= top && fetching {
            fetched.push(Fetched { planes, cmp });
        }
        let lines: [Option<sprite::SpriteLine>; 8] = std::array::from_fn(|n| channels[n].line());
        // Outside the window, or where no sprite shows, no sprite collides. Only the pixels under a
        // sprite are read: a sprite is 16 pixels from one after its HSTART.
        if line >= top && lines.iter().any(Option::is_some) {
            let priority = planes.priority() as usize;
            let (hstart, hstop) = window(&planes);
            let mut spans = [(i32::MAX, i32::MAX); 8];
            for (span, l) in spans.iter_mut().zip(&lines) {
                if let Some(l) = l {
                    let a = l.hstart as i32 + SPRITE_DELAY;
                    *span = (a.max(hstart), (a + 16).min(hstop));
                }
            }
            spans.sort_unstable();
            let shown = fetching.then(|| Line::new(&planes, &chip));
            let mut done = i32::MIN;
            for (a, b) in spans.into_iter().filter(|s| s.0 != i32::MAX) {
                for at in a.max(done)..b {
                    let px = shown.as_ref().map_or(0, |l| l.at(at));
                    // A pixel no sprite could collide with needs no sprite test.
                    if cmp.hit(0, px) == 0 {
                        continue;
                    }
                    for p in 0..4 {
                        for (k, m) in marked(&lines, p, at, px, priority).into_iter().enumerate() {
                            let n = 2 * p + k;
                            if m && cmp.counts(n) {
                                clx |= cmp.hit(n, px);
                            }
                        }
                    }
                }
                done = done.max(b);
            }
        }
        if fetching {
            planes.advance();
        }
        apply(
            line + 1,
            0,
            &mut planes,
            &mut channels,
            &mut clxcon,
            &mut bplcon0,
        );
    }
    // Playfield against playfield: one matching pixel anywhere in the window sets bit 0, so the
    // check stops at the first. The status bar's lines come first: its picture holds one (measured,
    // T70), so a play frame costs its few lines, not the whole screen.
    for f in fetched.iter().rev() {
        let (hstart, hstop) = window(&f.planes);
        let (from, to) = (hstart.max(left), hstop.min(right));
        let line = Line::new(&f.planes, &chip);
        if line.any_both(&f.cmp, from, to) {
            clx |= PLAYFIELDS;
            break;
        }
    }
    clx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The level's CLXCON.
    const PLAY: Compare = Compare {
        con: 0x37dd,
        dual: false,
    };

    #[test]
    fn the_levels_clxcon_matches_colour_29_on_both_playfields() {
        // 29 = planes 1, 3, 4, 5: odd and even match.
        assert_eq!(PLAY.hit(1, 29), 0b10_0010);
        assert_eq!(PLAY.hit(3, 29), 0b100_0100);
        // 21, 23, 31: the odd planes match but plane 2 or 4 does not, so in single playfield nothing.
        for px in [21, 23, 31] {
            assert_eq!(PLAY.hit(1, px), 0, "colour {px}");
        }
        // Even planes alone (plane 4 set, plane 2 clear): the PF2 bit only.
        assert_eq!(PLAY.hit(1, 8), 0b10_0000);
        // Dual playfield compares the odd planes whatever the even ones.
        let dual = Compare { dual: true, ..PLAY };
        assert_eq!(dual.hit(1, 21), 0b10);
    }

    #[test]
    fn ensp_or_s_the_odd_sprites_in() {
        assert!(PLAY.counts(0) && PLAY.counts(1) && PLAY.counts(2) && PLAY.counts(3));
        assert!(!PLAY.counts(5) && !PLAY.counts(7));
    }

    fn line(hstart: u16, data: [u16; 2], attached: bool) -> Option<sprite::SpriteLine> {
        Some(sprite::SpriteLine {
            hstart,
            data,
            attached,
            cut: None,
        })
    }

    #[test]
    fn an_attached_pair_marks_its_odd_sprite_only_in_front() {
        let mut l = [None; 8];
        l[0] = line(0x100, [0x8000, 0], false);
        l[1] = line(0x100, [0, 0], true);
        let at = 0x100 + SPRITE_DELAY;
        assert_eq!(marked(&l, 0, at, 29, 4), [false, true]);
        // Behind the playfield (priority 0): no mark over a coloured pixel, a mark over colour 0.
        assert_eq!(marked(&l, 0, at, 29, 0), [false, false]);
        assert_eq!(marked(&l, 0, at, 0, 0), [false, true]);
        // One pixel right of the sprite's 16: nothing.
        assert_eq!(marked(&l, 0, at + 16, 29, 4), [false, false]);
        // Unattached, each sprite marks itself, in front or not.
        l[1] = line(0x100, [0x8000, 0], false);
        assert_eq!(marked(&l, 0, at, 29, 0), [true, true]);
    }

    #[test]
    fn eight_pixels_at_a_time_find_what_each_pixel_finds() {
        let mut planes = Planes::default();
        for (r, v) in [
            (reg::BPLCON0, 0x5200),
            (0x092, 0x38),
            (0x094, 0xd0),
            (0x08e, 0x2c81),
            (0x090, 0x2cc1),
        ] {
            planes.write(r, v);
        }
        let mut seed = 0x1234_5678u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for delay in [0u16, 0x33, 0x55, 0x3f] {
            planes.write(reg::BPLCON1, delay);
            for _ in 0..200 {
                // Mostly no match; now and then one colour 29 pixel.
                let mut data = [[0u8; LINE_BYTES]; 6];
                for d in data.iter_mut().take(5) {
                    for b in d.iter_mut() {
                        *b = next() as u8 & next() as u8;
                    }
                }
                if next() % 2 == 0 {
                    let (k, bit) = ((next() % 40) as usize, 1 << (next() % 8));
                    for (p, d) in data.iter_mut().enumerate().take(5) {
                        d[k] = if 29 >> p & 1 != 0 {
                            d[k] | bit
                        } else {
                            d[k] & !bit
                        };
                    }
                }
                let line = Line::of(&planes, data);
                let (from, to) = (0x81 + (next() % 40) as i32, 0x1c1 - (next() % 40) as i32);
                let each = (from..to).any(|at| PLAY.both(line.at(at)));
                assert_eq!(line.any_both(&PLAY, from, to), each, "delay {delay:#x}");
            }
        }
    }
}
