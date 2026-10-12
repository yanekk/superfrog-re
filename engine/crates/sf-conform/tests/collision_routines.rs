//! The frog against the map (`sf_core::level::collision`, T14) against the original's routines,
//! on the routine oracle: the ground probe and `frog_tiles`, over generated frogs standing,
//! walking, jumping and falling among generated cells (`sf_conform::routine`).

mod common;

use common::*;
use sf_conform::routine::{Gen, Setup};
use sf_core::frog::{FROG, f};
use sf_core::level::collision;

/// A marker as the ground probes see them: mostly ground, surfaces and air.
fn any_marker(g: &mut Gen) -> u16 {
    match g.below(10) {
        0..=2 => 0,
        3..=7 => g.pick(&[0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x3d, 0x3f]),
        _ => g.below(64) as u16,
    }
}

/// A frog somewhere in the level, moving any way, with the cells around it made up.
fn frog_among_cells(g: &mut Gen, s: &mut Setup) {
    let x = 48 + g.below(4600) as u16;
    let y = 48 + g.below(1200) as u16;
    s.poke_w(FROG + f::X, x);
    s.poke_w(FROG + f::Y, y);
    // Speeds the game reaches: a falling frog is capped at 8 a frame, a jump starts near -16.
    s.poke_w(FROG + f::VX, (g.below(25) as i16 - 12) as u16);
    s.poke_w(FROG + f::VY, (g.below(30) as i16 - 18) as u16);
    s.poke_w(FROG + f::AIR, g.pick(&[0, 0, 1, 2, 3, 4, 5, 0x10, 0x20]));
    s.poke_w(FROG + f::ON_GROUND, g.below(2) as u16);
    s.poke_w(FROG + f::ON_PLATFORM, if g.one_in(8) { 1 } else { 0 });
    s.poke_w(FROG + f::SLOPE_SIDE, g.below(4) as u16);
    s.poke_w(FROG + f::FACING, edgy(g));
    let (col, row) = ((x + 8) as u32 / 16, (y.wrapping_sub(16)) as u32 / 16);
    for r in row.saturating_sub(2)..(row + 7).min(84) {
        for c in col.saturating_sub(4)..(col + 6).min(300) {
            let tile = g.below(840) as u16;
            s.poke_w(cell(c, r), tile << 6 | any_marker(g));
        }
    }
}

#[test]
fn ground_probe_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(11);
    for case in 0..CASES {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        frog_among_cells(&mut g, &mut s);
        rig.check("ground_probe", case, 0xdfb0, &s, collision::ground_probe);
    }
}
