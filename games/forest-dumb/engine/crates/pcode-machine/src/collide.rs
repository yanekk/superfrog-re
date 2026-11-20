//! CLXDAT, Denise's collision register, as the oracle (vAmiga with its three collision checks on,
//! `amiga-oracle`'s `-clx` build) sets it while it draws each line: sprite against playfield
//! (`checkS2PCollisions`), sprite against sprite (`checkS2SCollisions`) and playfield against
//! playfield (`checkP2PCollisions`), at lores pixel granularity: sprite against playfield and
//! playfield against playfield over the pixels inside the line's display window, sprite against
//! sprite from the line's first bitplane data on, window or not ([`sprites`]), the pixels and marks being what [`crate::video`] draws.
//!
//! The rules (vAmiga's, which the original is judged against):
//!
//! - CLXCON: bits 0-5 MVBP (the value each plane must hold), 6-11 ENBP (the planes compared),
//!   12-15 ENSP (odd sprite 1, 3, 5, 7 OR'd into its pair's collisions). Odd planes are playfield 1,
//!   even planes playfield 2.
//! - Sprite `n` over playfield 2's match sets bit `5 + n/2`; in single-playfield mode a pixel whose
//!   playfield 2 does not match is not compared further (vAmiga's "hardware oddity"), so playfield 1's
//!   match sets bit `1 + n/2` only where playfield 2 matched too; in dual playfield, wherever it
//!   matches.
//! - A sprite marks the pixels it shows: an unattached one wherever it is not transparent; an attached
//!   pair as its odd sprite, and only where the pair is drawn in front of the playfield (vAmiga marks
//!   its depth buffer only then).
//! - Sprite against sprite: bits 9 to 14 for the groups 0/1-2/3, 0/1-4/5, 0/1-6/7, 2/3-4/5, 2/3-6/7,
//!   4/5-6/7, a group being its even sprite and, with ENSP, its odd one.
//! - Playfield against playfield, bit 0: any window pixel matching both playfields.
//! - A read returns the bits with bit 15 set and clears them.

use amiga_chips::sprite::{SPRITE_DELAY, SpriteLine};

/// What CLXDAT reads with no collision: bit 15 is always set.
pub const CLXDAT_NONE: u16 = 0x8000;
/// CLXCON's register offset.
pub const CLXCON: u16 = 0x098;
/// CLXDAT's register offset.
pub const CLXDAT: u16 = 0x00E;

/// CLXCON's comparison, with the playfield mode it is made in.
#[derive(Debug, Clone, Copy)]
pub struct Compare {
    pub con: u16,
    pub dual: bool,
}

impl Compare {
    /// Whether odd sprite `n` (or any even one) takes part in collisions.
    fn counts(&self, n: usize) -> bool {
        n.is_multiple_of(2) || self.con >> (12 + n / 2) & 1 != 0
    }

    fn masks(&self) -> (u8, u8, u8, u8) {
        let enabled = (self.con >> 6) as u8;
        let (en1, en2) = (enabled & 0b01_0101, enabled & 0b10_1010);
        (en1, en2, self.con as u8 & en1, self.con as u8 & en2)
    }

    /// Whether playfield bits `px` match on both playfields.
    pub fn both(&self, px: u8) -> bool {
        let (en1, en2, mv1, mv2) = self.masks();
        px & en1 == mv1 && px & en2 == mv2
    }

    /// The bits a pixel of sprite `n` over playfield bits `px` sets.
    pub fn hit(&self, n: usize, px: u8) -> u16 {
        let (en1, en2, mv1, mv2) = self.masks();
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

/// The sprites of pair `p` that mark window position `at` over playfield bits `px`, with the
/// playfield's priority against the pairs (`BPLCON2`'s PF2P, as the picture draws them).
pub fn marked(lines: &[Option<SpriteLine>; 8], p: usize, at: i32, px: u8, priority: usize) -> [bool; 2] {
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

/// Sprite against sprite: CLXDAT's bit for groups `a < b`, bits 9 to 14 in the pairs' order
/// (0,1) (0,2) (0,3) (1,2) (1,3) (2,3). Computed, not listed: the pure crates carry no integer
/// table over 16 numbers (fd-conform's purity test).
fn s2s_bit(a: usize, b: usize) -> u16 {
    // Pairs before group a's: 3 + 2 + 1 for a = 0, 1, 2.
    let before = [0, 3, 5][a];
    9 + (before + b - a - 1) as u16
}

/// The sprite and playfield collision bits one line sets: `pf` the playfield bits of each pixel
/// of the picture's row (window position `left + x`), `inside` whether pixel `x` lies in the
/// line's display window. Sprite against sprite is [`sprites`].
pub fn line(
    cmp: &Compare,
    lines: &[Option<SpriteLine>; 8],
    priority: usize,
    left: i32,
    pf: &[u8],
    inside: impl Fn(usize) -> bool,
) -> u16 {
    let mut clx = 0;
    let any_sprite = lines.iter().any(Option::is_some);
    for (x, &px) in pf.iter().enumerate() {
        if !inside(x) {
            continue;
        }
        if clx & 1 == 0 && cmp.both(px) {
            clx |= 1;
        }
        if !any_sprite {
            continue;
        }
        let at = left + x as i32;
        for p in 0..4 {
            for (k, m) in marked(lines, p, at, px, priority).into_iter().enumerate() {
                let n = 2 * p + k;
                if m && cmp.counts(n) {
                    clx |= cmp.hit(n, px);
                }
            }
        }
    }
    clx
}

/// The window positions past the last a sprite can show at (its 9-bit start, 16 pixels, the
/// delay).
const SPRITE_END: i32 = 0x200 + 16 + SPRITE_DELAY;

/// Sprite against sprite on a line whose bitplanes are fetched: every position from `from` on,
/// window or not. vAmiga draws no sprite before the line's first BPL1DAT (`spriteClipBegin`) and
/// checks the pairs it drew without the window test S2P makes; measured on the oracle, the first
/// position counted is `2 * DDFSTRT + 15` and pixels right of DIWSTOP count (T31 review).
/// `pf` gives the playfield bits at a position, for an attached pair's priority.
pub fn sprites(
    cmp: &Compare,
    lines: &[Option<SpriteLine>; 8],
    priority: usize,
    from: i32,
    pf: impl Fn(i32) -> u8,
) -> u16 {
    let mut clx = 0;
    if lines.iter().filter(|l| l.is_some()).count() < 2 {
        return 0;
    }
    for at in from..SPRITE_END {
        let px = pf(at);
        let mut groups = 0u8;
        for p in 0..4 {
            for (k, m) in marked(lines, p, at, px, priority).into_iter().enumerate() {
                if m && cmp.counts(2 * p + k) {
                    groups |= 1 << p;
                }
            }
        }
        for a in 0..4 {
            for b in a + 1..4 {
                if groups >> a & 1 != 0 && groups >> b & 1 != 0 {
                    clx |= 1 << s2s_bit(a, b);
                }
            }
        }
    }
    clx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The formula gives the list it replaced, pair by pair (HRM, CLXDAT bits 9 to 14).
    #[test]
    fn the_sprite_pair_bits_are_the_hardwares() {
        assert_eq!(s2s_bit(0, 1), 9);
        assert_eq!(s2s_bit(0, 2), 10);
        assert_eq!(s2s_bit(0, 3), 11);
        assert_eq!(s2s_bit(1, 2), 12);
        assert_eq!(s2s_bit(1, 3), 13);
        assert_eq!(s2s_bit(2, 3), 14);
    }

    /// CLXCON `$37dd`: sprites 1 and 3 OR'd in, planes 1 to 5 compared with 1, 0, 1, 1, 1.
    const PLAY: Compare = Compare {
        con: 0x37dd,
        dual: false,
    };

    fn sprite(hstart: u16, data: [u16; 2], attached: bool) -> Option<SpriteLine> {
        Some(SpriteLine {
            hstart,
            data,
            attached,
            cut: None,
        })
    }

    #[test]
    fn single_playfield_compares_playfield_1_only_where_playfield_2_matched() {
        // 29 = planes 1, 3, 4, 5: both match.
        assert_eq!(PLAY.hit(1, 29), 0b10_0010);
        assert_eq!(PLAY.hit(3, 29), 0b100_0100);
        // 21, 23, 31: playfield 1 matches, playfield 2 not: nothing in single playfield.
        for px in [21, 23, 31] {
            assert_eq!(PLAY.hit(1, px), 0, "colour {px}");
        }
        assert_eq!(PLAY.hit(1, 8), 0b10_0000);
        let dual = Compare { dual: true, ..PLAY };
        assert_eq!(dual.hit(1, 21), 0b10);
    }

    #[test]
    fn ensp_or_s_the_odd_sprites_in() {
        assert!(PLAY.counts(0) && PLAY.counts(1) && PLAY.counts(2) && PLAY.counts(3));
        assert!(!PLAY.counts(5) && !PLAY.counts(7));
    }

    #[test]
    fn an_attached_pair_marks_its_odd_sprite_only_in_front() {
        let mut l = [None; 8];
        l[0] = sprite(0x100, [0x8000, 0], false);
        l[1] = sprite(0x100, [0, 0], true);
        let at = 0x100 + SPRITE_DELAY;
        assert_eq!(marked(&l, 0, at, 29, 4), [false, true]);
        assert_eq!(marked(&l, 0, at, 29, 0), [false, false]);
        assert_eq!(marked(&l, 0, at, 0, 0), [false, true]);
    }

    #[test]
    fn a_line_sets_sprite_playfield_and_playfield_bits_only_inside_the_window() {
        let mut l = [None; 8];
        l[0] = sprite(0x80, [0xffff, 0], false);
        // Sprite 0's first pixel is at window position 0x81; the row starts at 0x80.
        let mut pf = [0u8; 32];
        pf[1] = 29;
        let bits = line(&PLAY, &l, 4, 0x80, &pf, |_| true);
        assert_eq!(bits, 1 | 0b10_0010);
        // The same pixel outside the window sets nothing.
        assert_eq!(line(&PLAY, &l, 4, 0x80, &pf, |x| x != 1), 0);
    }

    #[test]
    fn two_groups_overlapping_set_their_sprite_bit() {
        let mut l = [None; 8];
        l[0] = sprite(0x80, [0x8000, 0], false);
        l[6] = sprite(0x80, [0x8000, 0], false);
        // Groups 0/1 and 6/7 meet at window position 0x81: bit 11, no playfield match (0 != MVBP).
        assert_eq!(sprites(&PLAY, &l, 4, 0x80, |_| 0), 1 << 11);
        // Not before `from`.
        assert_eq!(sprites(&PLAY, &l, 4, 0x82, |_| 0), 0);
        // Sprite 7 alone does not count without ENSP.
        let mut l2 = [None; 8];
        l2[0] = sprite(0x80, [0x8000, 0], false);
        l2[7] = sprite(0x80, [0x8000, 0], false);
        assert_eq!(sprites(&PLAY, &l2, 4, 0x80, |_| 0), 0);
    }
}
