//! The frog's routines (`sf_core::frog`, T14) against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at the first frame of W1-1, and the port
//! from the same memory transplanted, over 10 000 generated states: the frog's fields, the
//! joystick and the fire button varied. Every byte written, the sound driver's state and the Paula
//! writes must match (`sf_conform::routine`).
//!
//! Lives here, not in `sf-core/tests/`, because the purity test keeps the emulator out of the pure
//! crates' tests.

mod common;

use common::*;
use sf_conform::routine::{Gen, Setup};
use sf_core::frog::{self, FROG, f};

#[test]
fn frog_move_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(1);
    for case in 0..CASES {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        for o in [f::X, f::Y] {
            s.poke_w(FROG + o, g.below(0x1400) as u16);
        }
        for o in [f::VX, f::VY] {
            s.poke_w(FROG + o, small_word(&mut g));
        }
        s.poke_w(FROG + f::FROZEN, if g.one_in(8) { 1 } else { 0 });
        rig.check("frog_move", case, 0xdb58, &s, frog::physics::frog_move);
    }
}

#[test]
fn camera_follow_matches_the_original() {
    use sf_core::level::camera::*;
    let mut rig = rig();
    let mut g = Gen::new(2);
    for case in 0..CASES {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        let (x, y) = (g.below(0x1400) as u16, g.below(0x600) as u16);
        s.poke_w(FROG + f::X, x);
        s.poke_w(FROG + f::Y, y);
        s.poke_w(FROG + f::VX, small_word(&mut g));
        s.poke_w(FROG + f::VY, small_word(&mut g));
        let near = |g: &mut Gen, v: u16, off: u16| {
            if g.one_in(5) {
                g.next() as u16
            } else {
                v.wrapping_sub(off).wrapping_add(g.below(41) as u16).wrapping_sub(20)
            }
        };
        let cx = near(&mut g, x, 0x90);
        let cy = near(&mut g, y, 0x70);
        s.poke_l(CAMERA_X, if g.one_in(10) { g.next() as u32 } else { cx as u32 });
        s.poke_l(CAMERA_Y, if g.one_in(10) { g.next() as u32 } else { cy as u32 });
        s.poke_w(LEVEL_WIDTH, if g.one_in(4) { g.next() as u16 } else { 0x12c0 });
        s.poke_w(LEVEL_HEIGHT, if g.one_in(4) { g.next() as u16 } else { 0x540 });
        s.poke_b(SCROLL_DIRS, g.next() as u8);
        s.poke_w(FROG + f::FACING, edgy(&mut g));
        s.poke_w(FROG + f::CLIMB, edgy(&mut g));
        s.poke_w(FROG + f::CAMERA_HELD, if g.one_in(6) { 1 } else { 0 });
        s.poke_w(SHOOTER_MODE, if g.one_in(6) { 1 } else { 0 });
        s.poke_w(FINAL_LEVEL_FLAG, if g.one_in(6) { 1 } else { 0 });
        s.poke_w(SPRITES_HIDDEN, if g.one_in(6) { 1 } else { 0 });
        s.poke_w(SHOOTER_HALF_STEP, g.pick(&[0, 0xffff]));
        rig.check("camera_follow", case, 0xdb92, &s, camera_follow);
    }
}

#[test]
fn the_boot_s_tables_are_the_original_s() {
    use sf_core::layout::Ram;
    let mem = level();
    let rom = sf_conform::display_data();
    let rom = rom.rom();
    for (from, len) in [(0x17330, 0x16c), (0x174d6, 0x4c), (0x176c4, 0x60)] {
        for a in (0..len).map(|k| sf_core::layout::h0(from + k)) {
            assert_eq!(rom.byte(1, a), Some(mem.byte(a)), "h0 ${:x}", a - sf_core::layout::H0);
        }
    }
    for k in 0..0xe00 {
        let a = sf_core::mem::TILE_PTRS + k;
        assert_eq!(rom.byte(1, a), Some(mem.byte(a)), "${a:x}");
    }
    // W1's tiles where the game loaded them.
    for a in (sf_core::layout::H1..sf_core::layout::H1 + 840 * 160).step_by(97) {
        assert_eq!(rom.byte(1, a), Some(mem.byte(a)), "${a:x}");
    }
}
