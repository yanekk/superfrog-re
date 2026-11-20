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

#[test]
fn head_probe_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(12);
    for case in 0..CASES {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        frog_among_cells(&mut g, &mut s);
        rig.check("head_probe", case, 0xe4fc, &s, collision::head_probe);
    }
}

#[test]
fn slope_anim_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(13);
    for case in 0..CASES {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        frog_among_cells(&mut g, &mut s);
        let (c, r) = (g.below(300) as u32, g.below(84) as u32);
        s.poke_w(cell(c, r), (g.below(840) as u16) << 6 | any_marker(&mut g));
        s.poke_l(FROG + f::GROUND_CELL, cell(c, r));
        rig.check("slope_anim", case, 0xde2a, &s, collision::slope_anim);
    }
}


/// Markers `frog_tiles` reacts to, and ground.
fn handled_marker(g: &mut Gen) -> u16 {
    match g.below(3) {
        0 => 0,
        1 => g.pick(&[0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f]),
        _ => g.pick(&[
            4, 5, 6, 7, 38, 39, 40, 41, 42, 43, 44, 46, 47, 48, 49, 50, 51, 54, 55, 56, 57, 61, 63,
        ]),
    }
}

#[test]
fn frog_tiles_matches_the_original() {
    use sf_core::layout::h0;
    let mut rig = rig();
    let mut g = Gen::new(14);
    let mut hung = 0;
    for case in 0..CASES {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        frog_among_cells(&mut g, &mut s);
        let x = 48 + g.below(4600) as u16;
        let y = 48 + g.below(1200) as u16;
        s.poke_w(FROG + f::X, x);
        s.poke_w(FROG + f::Y, y);
        let (col, row) = ((x + 8) as u32 / 16, (y.wrapping_sub(16)) as u32 / 16);
        for r in row.saturating_sub(2)..(row + 7).min(84) {
            for c in col.saturating_sub(4)..(col + 6).min(300) {
                s.poke_w(cell(c, r), (g.below(840) as u16) << 6 | handled_marker(&mut g));
            }
        }
        for (o, v) in [
            (f::HURT, g.one_in(6) as u16),
            (f::DYING, g.one_in(8) as u16),
            (f::BLINK, if g.one_in(8) { g.below(40) as u16 } else { 0 }),
            (f::SHIELD, g.one_in(10) as u16),
            (f::SPRING, g.one_in(6) as u16),
            (f::IN_PASSAGE, g.one_in(6) as u16),
            (f::COINS_LEFT, g.pick(&[0, 0, 3])),
            (f::LIVES, g.below(6) as u16),
            (f::GRAVITY_TICK, g.below(6) as u16),
            (f::GRAVITY_DELAY, g.pick(&[0, 1, 2, 3, 5])),
            (f::SLOPE, (g.below(9) as i16 - 4) as u16),
            (f::SLIDING, g.below(2) as u16),
            (f::SLIDE_TICK, g.below(5) as u16),
            (f::LAUNCHED, g.below(3) as u16),
            (f::FLOAT_TOGGLE, g.pick(&[0, 0xffff])),
            (f::BOUNCE_VY, 0xfff0),
        ] {
            s.poke_w(FROG + o, v);
        }
        // At most one wall stopped the frog last frame: one probe list runs a frame.
        let wall = g.below(5);
        s.poke_w(FROG + f::WALL_RIGHT, (wall == 0) as u16);
        s.poke_w(FROG + f::WALL_LEFT, (wall == 1) as u16);
        s.poke_l(FROG + f::ANIM, g.pick(&[h0(0x10d8e), h0(0x10566), h0(0x10542), h0(0x1077a)]));
        s.poke_w(h0(0xeb28), g.below(6) as u16);
        s.poke_w(h0(0xebf0), g.below(10) as u16);
        s.poke_w(h0(0x14), g.one_in(10) as u16);
        s.poke_l(h0(0x18), if g.one_in(20) { 0xffff_ffff } else { 0 });
        if !rig.check("frog_tiles", case, 0xe2e2, &s, collision::frog_tiles) {
            hung += 1;
        }
    }
    // The original spins for ever when the frog faces left between two `$3d` edges with both
    // wall flags set (h0+$e3fe, DESIGN §2.11); the generator makes that often (~1.7 %).
    assert!(hung < CASES / 30, "the original hung or crashed on {hung} states");
}
