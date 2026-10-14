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
        // A quarter near the level's left or top edge, where a step past 0 is clamped.
        for o in [f::X, f::Y] {
            let p = if g.one_in(4) { g.below(24) } else { g.below(0x1400) };
            s.poke_w(FROG + o, p as u16);
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
    for (from, len) in [(0x17330, 0x16c), (0x174d6, 0x4c), (0x176c4, 0x60), (0x19ddc, 0x900)] {
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

/// The frog's animations the routines start (h0+$10492..$10f52), each a list of 8-byte steps.
const ANIMS: [u32; 31] = [
    0x10d8e, 0x10566, 0x10542, 0x1077a, 0x10d32, 0x10d3e, 0x10d0e, 0x10572, 0x10492, 0x10c9a,
    0x104fe, 0x10cd6, 0x10df2, 0x1061a, 0x10626, 0x10632, 0x10dfe, 0x10e0a, 0x1063e, 0x10e16,
    0x10646, 0x10e1e, 0x10f52, 0x10b7e, 0x105b6, 0x103f6, 0x10bce, 0x1041a, 0x10bf2, 0x103b2,
    0x10b8a,
];

/// An animation and a step of it, as the original's memory has them.
fn any_anim(g: &mut Gen) -> (u32, u32) {
    use sf_core::layout::{Ram, h0};
    let m = level();
    let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
    let anim = h0(g.pick(&ANIMS));
    let steps = (0..).take_while(|k| (long(anim + 8 * k) as i32) >= 0).count() as u64;
    (anim, anim + 8 * g.below(steps.max(1)) as u32)
}

/// A frog in any state the frog routines read.
fn any_frog(g: &mut Gen, s: &mut Setup) {
    use sf_core::layout::h0;
    let x = 48 + g.below(4600) as u16;
    let y = 48 + g.below(1200) as u16;
    s.poke_w(FROG + f::X, x);
    s.poke_w(FROG + f::Y, y);
    let near = |g: &mut Gen, v: u16| v.wrapping_add(g.below(7) as u16).wrapping_sub(3);
    s.poke_w(FROG + f::PREV_X, near(g, x));
    s.poke_w(FROG + f::PREV_Y, near(g, y));
    s.poke_w(FROG + f::VX, (g.below(25) as i16 - 12) as u16);
    s.poke_w(FROG + f::VY, (g.below(30) as i16 - 18) as u16);
    let (anim, step) = any_anim(g);
    s.poke_l(FROG + f::ANIM, anim);
    s.poke_l(FROG + f::ANIM_STEP, step);
    s.poke_l(FROG + f::SLOPE_ANIM, any_anim(g).0);
    s.poke_w(FROG + f::ANIM_TIME, g.pick(&[0xffff, 0, 1, 2, 5, 0x11, 0x4e1f, 0x4e20]));
    for (o, v) in [
        (f::AIR, g.pick(&[0, 0, 1, 2, 4, 0xc, 0x10, 0x1c, 0x20])),
        (f::ON_GROUND, g.below(2) as u16),
        (f::ON_PLATFORM, g.one_in(8) as u16),
        (f::FACING, g.pick(&[0, 0xffff])),
        (f::CLIMB, g.pick(&[0, 0xffff])),
        (f::HURT, g.one_in(5) as u16),
        (f::LANDED, g.one_in(3) as u16),
        (f::DYING, g.one_in(6) as u16),
        (f::BLINK, if g.one_in(5) { g.below(40) as u16 } else { 0 }),
        (f::SHIELD, if g.one_in(8) { g.below(3) as u16 } else { 0 }),
        (f::LAUNCH_HOLD, g.pick(&[0, 0, 0, 1, 5, 0xffff])),
        (f::LAUNCHED, g.pick(&[0, 0, 1, 3])),
        (f::SPECIAL_ANIM, if g.one_in(8) { 1 + g.below(3) as u16 } else { 0 }),
        (f::SIT_TIME, g.below(5) as u16),
        (f::IDLE_SOUND, g.below(3) as u16),
        (f::CROUCH, g.below(2) as u16),
        (f::CROUCH_TIME, g.below(6) as u16),
        (f::SLIDING, g.below(2) as u16),
        (f::SLIDE_TICK, g.below(3) as u16),
        (f::PUSHED, g.below(2) as u16),
        (f::SLOPE, (g.below(11) as i16 - 5) as u16),
        (f::SPEED, g.below(8) as u16),
        (f::MAX_SPEED, 4 + g.below(10) as u16),
        (f::BASE_SPEED, 4 + g.below(4) as u16),
        (f::SPEED_UP, g.pick(&[0xffff, 0, 1, 30])),
        (f::SPEED_TICK, g.below(5) as u16),
        (f::AIR_STEP, g.below(4) as u16),
        (f::AIR_STEP_DELAY, g.below(4) as u16),
        (f::AIR_SPEED, g.below(3) as u16),
        (f::FRICTION_TOGGLE, g.pick(&[0, 0xffff])),
        (f::RIGHT_TOGGLE, g.pick(&[0, 0xffff])),
        (f::LEFT_TOGGLE, g.pick(&[0, 0xffff])),
        (f::HURT_FRICTION, g.below(7) as u16),
        (f::SPUD_ON, g.below(2) as u16),
        (f::FLIGHT, g.one_in(5) as u16),
        (f::FIRE_HELD, g.below(2) as u16),
        (f::JUMP_PRESSES, g.below(10) as u16),
        (f::ANIM_LOCK, g.pick(&[0, 0, 0, 1, 2])),
        (f::HEAD_BUMP, g.below(2) as u16),
        (f::FROZEN, g.one_in(10) as u16),
        (f::BOUNCE_VY, 0xfff0),
        (f::GRAVITY_TICK, g.below(4) as u16),
        (f::SPRING, g.one_in(6) as u16),
    ] {
        s.poke_w(FROG + o, v);
    }
    let wall = g.below(5);
    s.poke_w(FROG + f::WALL_RIGHT, (wall == 0) as u16);
    s.poke_w(FROG + f::WALL_LEFT, (wall == 1) as u16);
    // The ground cell, and markers about the frog.
    let (col, row) = ((x + 8) as u32 / 16, y as u32 / 16);
    for r in row.saturating_sub(1)..(row + 6).min(84) {
        for c in col.saturating_sub(2)..(col + 3).min(300) {
            let m = g.pick(&[0, 0, 0x39, 0x3a, 0x3d, 0x3f, 6, 46]);
            s.poke_w(cell(c, r), (g.below(840) as u16) << 6 | m);
        }
    }
    s.poke_l(FROG + f::GROUND_CELL, cell(col, (row + 2).min(83)));
    let hist = if g.one_in(3) { y } else { y.wrapping_add(1) };
    for k in 0..8 {
        s.poke_w(h0(0x10182) + 2 * k, if g.one_in(10) { hist ^ 1 } else { hist });
    }
    s.poke_w(h0(0x11350), g.below(0x41) as u16);
    s.poke_w(h0(0x9f8), g.one_in(8) as u16);
    s.poke_w(h0(0x9dc), g.one_in(10) as u16);
    s.poke_w(h0(0x9e2), g.one_in(10) as u16);
    s.poke_w(h0(0x9e6), g.below(2) as u16);
    s.poke_w(h0(0x1194e), g.pick(&[0, 0xffff]));
    s.poke_w(h0(0x11a28), if g.one_in(10) { 0xf000 } else { 0 });
    s.poke_w(h0(0x11a2a), g.pick(&[0, 3, 0x300, 0x100, 0x103]));
    s.poke_l(h0(0x18), if g.one_in(10) { 0xffff_ffff } else { 0 });
    s.poke_l(h0(0x40da), g.pick(&[1, 5]));
    let (lo, hi) = (g.next() as u32, g.next() as u32);
    s.poke_l(h0(0x160d4), lo);
    s.poke_l(h0(0x160d8), hi);
}


fn frog_check(name: &str, entry: u32, seed: u64, port: fn(&mut sf_core::level::LevelCtx)) {
    let mut rig = rig();
    let mut g = Gen::new(seed);
    let mut skipped = 0;
    for case in 0..CASES {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        // The bubble a pushing surface makes is not modelled (T17), as the case files freeze it.
        s.pokes.extend(sf_conform::freeze::unported_spawn_patches());
        any_frog(&mut g, &mut s);
        if !rig.check(name, case, entry, &s, port) {
            skipped += 1;
        }
    }
    assert!(skipped < CASES / 100, "{name}: the original hung or crashed on {skipped} states");
}

#[test]
fn frog_anim_tick_matches_the_original() {
    frog_check("frog_anim_tick", 0x10222, 21, frog::anim::frog_anim_tick);
}

#[test]
fn frog_input_anim_matches_the_original() {
    frog_check("frog_input_anim", 0xfe34, 22, frog::anim::frog_input_anim);
}

#[test]
fn frog_walk_input_matches_the_original() {
    frog_check("frog_walk_input", 0x113be, 23, frog::physics::frog_walk_input);
}

#[test]
fn frog_air_anim_matches_the_original() {
    frog_check("frog_air_anim", 0x11056, 24, frog::physics::frog_air_anim);
}

#[test]
fn frog_jump_float_matches_the_original() {
    frog_check("frog_jump_float", 0x11352, 25, frog::anim::frog_jump_float);
}

#[test]
fn place_frog_sprites_matches_the_original() {
    frog_check("place_frog_sprites", 0x19bbe, 26, frog::anim::place_frog_sprites);
}

/// The rare ends of `frog_air_anim`: sitting on a pushing surface (the croak, with its random
/// chance) and resting under a marker-`$39` cell (the sitting lock).
#[test]
fn frog_air_anim_on_pushing_surfaces_matches_the_original() {
    use sf_core::layout::h0;
    let mut rig = rig();
    let mut g = Gen::new(27);
    let (mut croaks, mut locks) = (0, 0);
    for case in 0..CASES {
        let mut s = Setup::default();
        s.pokes.extend(sf_conform::freeze::unported_spawn_patches());
        any_frog(&mut g, &mut s);
        if g.one_in(4) {
            s.hw = any_input(&mut g);
        }
        let x = 48 + g.below(4600) as u16;
        let y = 48 + g.below(1200) as u16;
        s.poke_w(FROG + f::X, x);
        s.poke_w(FROG + f::Y, y);
        for (o, v) in [
            (f::HURT, 0),
            (f::DYING, 0),
            (f::SHIELD, 0),
            (f::LAUNCH_HOLD, 0),
            (f::SPECIAL_ANIM, 0),
            (f::ON_GROUND, 1),
            (f::PUSHED, g.pick(&[0, 1, 1])),
            (f::SLIDING, g.below(2) as u16),
            (f::CLIMB, g.pick(&[0, 0xffff, 0xffff])),
            (f::SIT_TIME, g.pick(&[0, 0, 1])),
            (f::IDLE_SOUND, g.below(3) as u16),
            (f::SLOPE, g.pick(&[0xfffe, 0xffff, 0, 1, 2])),
            (f::VX, g.pick(&[0, 0xfff8, 0xfff9, 7, 8, 3])),
            (f::VY, g.pick(&[9, 9, 0, 3])),
        ] {
            s.poke_w(FROG + o, v);
        }
        let (col, row) = ((x + 8) as u32 / 16, (y as u32 / 16 + 2).min(83));
        s.poke_l(FROG + f::GROUND_CELL, cell(col, row));
        let m = if g.one_in(2) { 0x39 } else { 0 };
        s.poke_w(cell(col, row - 1), (g.below(840) as u16) << 6 | m);
        s.poke_l(h0(0x18), 0);
        s.poke_l(h0(0x40da), g.pick(&[1, 1, 5]));
        let croak = s.hw.joy1dat & 0x303 == 0;
        if rig.check("frog_air_anim (pushing)", case, 0x11056, &s, frog::physics::frog_air_anim) {
            croaks += croak as u32;
            locks += (m == 0x39) as u32;
        }
    }
    assert!(croaks > 1000 && locks > 1000, "{croaks} croak and {locks} lock setups");
}

#[test]
fn frog_level_entry_matches_the_original() {
    use sf_core::layout::h0;
    use sf_core::level::camera::{FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT};
    let mut rig = rig();
    let mut g = Gen::new(28);
    for case in 0..CASES {
        let mut s = Setup::default();
        any_frog(&mut g, &mut s);
        s.poke_w(h0(0x9e2), (!g.one_in(8)) as u16);
        s.poke_w(h0(0x6c92), g.below(10) as u16);
        s.poke_w(FROG + f::ANIM_TIME, g.pick(&[0x1f3f, 0x1f40, 0x1f41, 3, 0xffff]));
        s.poke_w(h0(0x1e), g.one_in(5) as u16);
        for sp in [FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT] {
            s.poke_w(sp, 0x60 + g.below(0x40) as u16);
            s.poke_w(sp + 2, 0x40 + g.below(0x80) as u16);
        }
        rig.check("frog_level_entry", case, 0x6bd4, &s, frog::anim::frog_level_entry);
    }
}

/// Any bytes in every field of the frog's `$200`-byte record, and any options.
fn any_record(g: &mut Gen, s: &mut Setup) {
    use sf_core::layout::h0;
    any_frog(g, s);
    for o in (0..0x200).step_by(2) {
        if g.one_in(3) {
            s.poke_w(FROG + o, edgy(g));
        }
    }
    s.poke_w(h0(0x9e4), g.pick(&[0, 1, 2, 3, 0xffff]));
    s.poke_w(h0(0x9e6), g.pick(&[0, 1, 2, 0xffff]));
    s.poke_l(h0(0x445e), g.next() as u32);
    s.a[0] = FROG;
}

#[test]
fn frog_new_game_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(29);
    for case in 0..CASES {
        let mut s = Setup::default();
        any_record(&mut g, &mut s);
        rig.check("frog_new_game", case, 0x8f90, &s, |c| frog::frog_new_game(&mut c.bus));
    }
}

#[test]
fn frog_reset_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(30);
    for case in 0..CASES {
        let mut s = Setup::default();
        any_record(&mut g, &mut s);
        rig.check("frog_reset", case, 0x902e, &s, |c| frog::frog_reset(&mut c.bus));
    }
}

/// h0+$19c9c (T86): with VPOS's low byte on `$1d`..`$21` (compared signed) the CPU waits for line
/// `$22` before it writes a frog sprite's copper words; past the frame's end the beam is on the
/// next frame's lines. `frog_anim_tick` and `place_frog_sprites` both wait.
#[test]
fn the_frog_sprites_wait_out_lines_1d_to_21() {
    use sf_core::level::beam::{Clock, FRAME_LINES};
    use sf_core::level::{LevelCtx, standard_dispatch};
    use sf_core::mem::Bus;
    let rig = rig();
    let display = sf_conform::display_data();
    let mut st = rig.state().clone();
    let mut paula = Vec::new();
    let mut c = LevelCtx::new(
        Bus::new(&mut st, display.rom()),
        standard_dispatch(),
        sf_core::level::Hw::default(),
        sound(),
        &mut paula,
    );
    let n = FRAME_LINES;
    let on = |line| Clock { line, into: 0 };
    for (from, to) in [
        (0x1c, 0x1c),
        (0x1d, 0x22),
        (0x21, 0x22),
        (0x22, 0x22),
        (0x9d, 0x9d),
        (0x11d, 0x122),
        (n + 0x1c, n + 0x1c),
        (n + 0x1e, n + 0x22),
        (n + 0x22, n + 0x22),
    ] {
        c.clock = on(from);
        c.sprite_wait();
        assert_eq!(c.clock.line, to, "sprite_wait from line {from:#x}");
    }
    c.clock = on(0x1e);
    frog::anim::place_frog_sprites(&mut c);
    let line = c.clock.line;
    assert!(line >= 0x22, "place_frog_sprites from line $1e: {line:#x}");
    c.bus.set_w(FROG + f::ANIM_TIME, 0xffff);
    c.clock = on(0x1e);
    frog::anim::frog_anim_tick(&mut c);
    let line = c.clock.line;
    assert!(line >= 0x22, "frog_anim_tick from line $1e: {line:#x}");
}
