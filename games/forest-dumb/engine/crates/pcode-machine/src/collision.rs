//! CLXDAT, Denise's collision detection, as the oracle's emulator makes it (vAmiga
//! `Denise::checkS2PCollisions`, `checkS2SCollisions`, `checkP2PCollisions`; all three on in
//! `amiga-oracle`'s shim), per line, from what the line drew (translator T30). The rules are those
//! fd-core's `render::contact` was measured to (forest-dumb level T59, T70), for a whole machine:
//!
//! - **Sprite against playfield** (bits 1 to 8), per sprite pixel inside the line's window: the
//!   even planes compared (PF2) set bit `5 + n/2`; in single-playfield mode a pixel whose even
//!   planes do not match is not compared further (vAmiga's "hardware oddity"), so the odd planes
//!   (PF1) set bit `1 + n/2` only where the even ones matched too. An odd sprite counts only with
//!   its ENSP bit; an attached pair's pixel counts as its odd sprite's and only where the pair is in
//!   front of the playfield; an unattached sprite's wherever it is not transparent.
//! - **Sprite against sprite** (bits 9 to 14): two groups (a pair, its odd sprite with ENSP) with a
//!   pixel at the same position: 9 is groups 0-1, 10 0-2, 11 0-3, 12 1-2, 13 1-3, 14 2-3.
//! - **Playfield against playfield** (bit 0): a window pixel matching both playfields' comparison.
//!
//! A read returns bit 15 set and clears the rest; the machine ORs each line's bits in as the beam
//! passes it.

use amiga_chips::sprite::{SPRITE_DELAY, SpriteLine};

/// CLXCON's register offset, and CLXDAT's.
pub(crate) const CLXCON: u16 = 0x098;
pub(crate) const CLXDAT: u16 = 0x00E;

/// CLXCON's comparison, split by playfield as Denise splits it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Compare {
    pub con: u16,
    pub dual: bool,
}

impl Compare {
    /// ENSP for an odd sprite `n`: whether it is OR'd into its pair's collisions.
    fn counts(&self, n: usize) -> bool {
        n % 2 == 0 || self.con >> (12 + n / 2) & 1 != 0
    }

    fn split(&self) -> (u8, u8, u8, u8) {
        let enabled = (self.con >> 6) as u8;
        let (en1, en2) = (enabled & 0b01_0101, enabled & 0b10_1010);
        (en1, en2, self.con as u8 & en1, self.con as u8 & en2)
    }

    /// Whether playfield bits `px` match on both playfields.
    fn both(&self, px: u8) -> bool {
        let (en1, en2, mv1, mv2) = self.split();
        px & en1 == mv1 && px & en2 == mv2
    }

    /// The bits a pixel of sprite `n` over playfield bits `px` sets.
    fn hit(&self, n: usize, px: u8) -> u16 {
        let (en1, en2, mv1, mv2) = self.split();
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

/// Sprite `n`'s two bits at window position `at`, if it shows there.
fn bits(lines: &[Option<SpriteLine>; 8], n: usize, at: i32) -> Option<u8> {
    let s = lines[n]?;
    let i = at - s.hstart as i32 - SPRITE_DELAY;
    (0..16).contains(&i).then(|| {
        let b = 15 - i;
        (s.data[0] >> b & 1 | (s.data[1] >> b & 1) << 1) as u8
    })
}

/// One line's collisions. `row` holds the playfield's pixel bits from window position `left` (no
/// sprite drawn over them), `window` the line's display window `(hstart, hstop)` in window
/// positions, `fetched` whether the line fetched planes, `priority` the playfield's against the
/// pairs (BPLCON2's PF2P).
pub(crate) fn line(
    cmp: &Compare,
    lines: &[Option<SpriteLine>; 8],
    row: &[u8],
    left: i32,
    window: (i32, i32),
    fetched: bool,
    priority: usize,
) -> u16 {
    let px_at = |at: i32| -> u8 {
        let x = at - left;
        if fetched && (0..row.len() as i32).contains(&x) {
            row[x as usize]
        } else {
            0
        }
    };
    let mut clx = 0u16;
    if fetched {
        let (from, to) = (window.0.max(left), window.1.min(left + row.len() as i32));
        if (from..to).any(|at| cmp.both(px_at(at))) {
            clx |= 1;
        }
    }
    if lines.iter().all(Option::is_none) {
        return clx;
    }
    // Only where a sprite is: 16 pixels from one after its HSTART.
    let mut spans: Vec<(i32, i32)> = lines
        .iter()
        .flatten()
        .map(|l| {
            let a = l.hstart as i32 + SPRITE_DELAY;
            (a.max(window.0), (a + 16).min(window.1))
        })
        .filter(|(a, b)| a < b)
        .collect();
    spans.sort_unstable();
    let mut done = i32::MIN;
    for (a, b) in spans {
        for at in a.max(done)..b {
            let px = px_at(at);
            let mut groups = [false; 4];
            for p in 0..4 {
                let (even, odd) = (bits(lines, 2 * p, at), bits(lines, 2 * p + 1, at));
                let attached = lines[2 * p + 1].is_some_and(|s| s.attached);
                let (e, o) = (even.unwrap_or(0), odd.unwrap_or(0));
                // vAmiga marks an attached pair only where it is drawn in front of the playfield.
                let marked = if attached {
                    [false, (e | o << 2) != 0 && (p < priority || px == 0)]
                } else {
                    [e != 0, o != 0]
                };
                for (k, m) in marked.into_iter().enumerate() {
                    let n = 2 * p + k;
                    if m && cmp.counts(n) {
                        clx |= cmp.hit(n, px);
                        groups[p] = true;
                    }
                }
            }
            let mut bit = 9;
            for g in 0..4 {
                for h in g + 1..4 {
                    if groups[g] && groups[h] {
                        clx |= 1 << bit;
                    }
                    bit += 1;
                }
            }
        }
        done = done.max(b);
    }
    clx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The level copper list's CLXCON (forest-dumb `h1+$118`): sprites 1 and 3 with 0 and 2, all
    /// six planes compared with 1, 0, 1, 1, 1, 0: colour 29 alone matches.
    const LEVEL: Compare = Compare {
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
    fn a_sprite_over_colour_29_sets_its_pairs_odd_and_even_bits() {
        let mut lines = [None; 8];
        lines[0] = sprite(100 - SPRITE_DELAY as u16, [0x8000, 0], false);
        let mut row = vec![0u8; 64];
        row[100 - 80] = 29;
        assert_eq!(line(&LEVEL, &lines, &row, 80, (80, 144), true, 4), 0b10_0010 | 1);
        // Colour 21 matches the odd planes only: not compared further in single playfield.
        row[100 - 80] = 21;
        assert_eq!(line(&LEVEL, &lines, &row, 80, (80, 144), true, 4), 0);
    }

    #[test]
    fn outside_the_window_or_off_the_sprite_nothing_collides() {
        let mut lines = [None; 8];
        lines[0] = sprite(100 - SPRITE_DELAY as u16, [0x8000, 0], false);
        let mut row = vec![0u8; 64];
        row[100 - 80] = 29;
        assert_eq!(line(&LEVEL, &lines, &row, 80, (80, 100), true, 4), 0);
        lines[0] = sprite(101 - SPRITE_DELAY as u16, [0x8000, 0], false);
        assert_eq!(line(&LEVEL, &lines, &row, 80, (80, 144), true, 4), 1);
    }

    #[test]
    fn an_attached_pair_counts_only_in_front_and_as_its_odd_sprite() {
        let mut lines = [None; 8];
        lines[0] = sprite(100 - SPRITE_DELAY as u16, [0x8000, 0], false);
        lines[1] = sprite(100 - SPRITE_DELAY as u16, [0, 0], true);
        let mut row = vec![0u8; 64];
        row[100 - 80] = 29;
        // Pair 0 in front of the playfield (priority 4): sprite 1's bits, with ENSP1 set.
        assert_eq!(line(&LEVEL, &lines, &row, 80, (80, 144), true, 4), 0b10_0010 | 1);
        // Behind it, over a non-zero pixel: no mark.
        assert_eq!(line(&LEVEL, &lines, &row, 80, (80, 144), true, 0), 1);
    }

    #[test]
    fn two_groups_overlapping_set_their_sprite_bit() {
        let mut lines = [None; 8];
        lines[0] = sprite(100, [0xffff, 0], false);
        lines[4] = sprite(108, [0xffff, 0], false);
        let row = vec![0u8; 64];
        let none = Compare {
            con: 0x0000,
            dual: false,
        };
        // CLXCON 0 compares no plane, so every pixel matches: P2P and S2P bits too.
        let clx = line(&none, &lines, &row, 80, (80, 144), true, 4);
        assert_eq!(clx & 0x7e00, 1 << 10, "groups 0 and 2: bit 10");
    }
}
