//! The level's tally (T38, `sf_core::level::tally`) against the original's routines on the
//! routine oracle: the pages' icons cleared past their count (h0+$47f6), the level's number
//! (h0+$4992), the score's print (h0+$479a) and the extra-life check the points are followed by
//! (h0+$d9e). `level_tally` itself waits on the vertical blank's fades, which the rig does not
//! run: its frames are proven by `tally-w1-1`.

mod common;

use common::*;
use sf_conform::routine::{Gen, Setup};
use sf_core::layout::{h0, h3};
use sf_core::level::{panel, tally};

/// Display buffer 0, where the tally's picture is.
const BUFFER_0: u32 = h3(0x10600);
/// `world`, `level`, `score`, the frog's lives (`+$58`) and next extra life (`+$aa`),
/// `extra_life_shown`.
const WORLD: u32 = h0(0x40da);
const LEVEL: u32 = h0(0x40de);
const SCORE: u32 = h0(0x834);
const LIVES: u32 = h0(0x7d6 + 0x58);
const NEXT_LIFE: u32 = h0(0x7d6 + 0xaa);
const EXTRA_LIFE_SHOWN: u32 = h0(0xdd4);

#[test]
fn the_pages_icons_are_cleared_as_the_original_clears_them() {
    let mut rig = rig();
    let mut g = Gen::new(380);
    let mut ran = 0;
    for case in 0..60 {
        let mut s = Setup::default();
        let n = g.pick(&[0u32, 1, 2, 3, 4, 5, 6, 7, 0xffff_ffff]);
        let anywhere = 2 * g.below(0x1000) as u32;
        let to = BUFFER_0 + g.pick(&[0xdac, 0x107c, 0x134c, anywhere]);
        s.d[0] = n;
        s.a[0] = to;
        if rig.check("h0+$47f6", case, 0x47f6, &s, |c| {
            tally::hide_icons(c, n, to);
        }) {
            ran += 1;
        }
    }
    assert_eq!(ran, 60);
}

#[test]
fn the_level_number_is_printed_as_the_original_prints_it() {
    let mut rig = rig();
    let mut g = Gen::new(381);
    for case in 0..40 {
        let mut s = Setup::default();
        s.poke_l(WORLD, 1 + g.below(7) as u32);
        s.poke_l(LEVEL, 1 + g.below(4) as u32);
        assert!(rig.check("h0+$4992", case, 0x4992, &s, |c| {
            tally::level_text(c);
        }));
    }
}

#[test]
fn the_score_is_printed_as_the_original_prints_it() {
    let mut rig = rig();
    let mut g = Gen::new(382);
    for case in 0..80 {
        let mut s = Setup::default();
        let score = match g.below(4) {
            0 => g.pick(&[
                0,
                5_000,
                10_000,
                25_000,
                999_999_999,
                1_000_000_000,
                u32::MAX,
            ]),
            1 => g.below(100_000) as u32,
            _ => g.below(100_000_000) as u32,
        };
        s.poke_l(SCORE, score);
        assert!(rig.check("h0+$479a", case, 0x479a, &s, |c| {
            tally::score_text(c);
        }));
    }
}

/// The points taken, the extra-life check (h0+$d9e) draws the lives on the panel, a life more
/// for each 200 000 the score has passed.
#[test]
fn the_extra_life_check_after_the_points_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(383);
    for case in 0..60 {
        let mut s = Setup::default();
        let next = 200_000 * (1 + g.below(3) as u32);
        s.poke_l(NEXT_LIFE, next);
        s.poke_l(
            SCORE,
            next.wrapping_sub(25_000)
                .wrapping_add(5_000 * g.below(12) as u32),
        );
        s.poke_w(LIVES, g.pick(&[0, 1, 2, 4, 5, 9]));
        s.poke_w(EXTRA_LIFE_SHOWN, g.pick(&[0, 0, 1]));
        assert!(rig.check("h0+$d9e", case, 0xd9e, &s, panel::extra_life));
    }
}
