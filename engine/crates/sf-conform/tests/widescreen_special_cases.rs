//! real-widescreen T05 (DESIGN §2.6, D10): the objects that test the screen's edge themselves
//! follow the wide screen's edges, and with the mode off keep the original's. Spud, the bonus
//! level's shots and the watchers' eyes are run on the engine's own level state, under both
//! screens. That 4:3 is the original is the routine oracle's (`items_routines`, `bonus_routines`,
//! `enemies_w1_routines`, `enemies_w5_routines`).
//!
//! The chain (`chain_update`), the witch's loop and `set_cell_tile` need no change: the chain's
//! path is relative to its spawn x (which the spawn scan moves, T02), the witch's to `camera_x`
//! (which T04 clamps), and `set_cell_tile` only feeds the display buffers, while the strips draw
//! from the map.

use sf_conform::display_data;
use sf_core::bonus::shots::{self, SHOT_CYCLE, SHOT_DELAY, SHOTS};
use sf_core::enemies::common::direction;
use sf_core::enemies::w1::watcher;
use sf_core::frog::{FROG, f};
use sf_core::items::{powerups, template};
use sf_core::layout::h0;
use sf_core::level::objects::{self, SPUD_BOB};
use sf_core::level::screen::{Screen, WIDE_EXTRA};
use sf_core::level::camera::{CAMERA_X, CAMERA_Y, SHOOTER_MODE, SPRITES_HIDDEN};
use sf_core::level::{Hw, LevelCtx, standard_dispatch};
use sf_core::mem::Bus;
use sf_core::{Game, LevelId, Start, State};

/// The spud's fields in the frog's record (`items/powerups.rs`).
const IN_FLIGHT: u32 = 0x72;
const SPUD_X: u32 = 0x74;
const SPUD_Y: u32 = 0x76;
const SPUD_VX: u32 = 0x78;
const SPUD_VY: u32 = 0x7a;
const SPUD_WAIT: u32 = 0xae;
/// `park`'s x: the spud out of play.
const PARKED_X: u16 = 0x639c;
/// `shot_cycle_table` (h0+$80cc).
const SHOT_CYCLE_TABLE: u32 = h0(0x80cc);
const E: u16 = WIDE_EXTRA;

/// The level's state a few frames in, its objects live.
fn level_state(id: &str) -> State {
    let mut g = Game::with_display(display_data(), Start::Level(id.parse::<LevelId>().unwrap()));
    for _ in 0..30 {
        g.step(Default::default());
    }
    g.state().clone()
}

/// `f` run on `st` as a level step against `screen`, with `hw`.
fn run<R>(st: &mut State, screen: Screen, hw: Hw, f: impl FnOnce(&mut LevelCtx) -> R) -> R {
    let display = display_data();
    let mut paula = Vec::new();
    let mut c = LevelCtx::new(Bus::new(st, display.rom()), standard_dispatch(), hw, display.sound(), &mut paula);
    c.screen = screen;
    f(&mut c)
}

fn camera(st: &State) -> (u16, u16) {
    (st.w(CAMERA_X + 2), st.w(CAMERA_Y + 2))
}

/// One pass of a spud in flight at screen x `sx` (from `camera_x`, signed), not moving, the frog
/// well away: parked or not.
fn spud_parked_at(screen: Screen, sx: i16) -> bool {
    let mut st = level_state("W1-1");
    assert_eq!(st.w(SHOOTER_MODE), 0);
    let (cx, cy) = camera(&st);
    st.set_w(SPRITES_HIDDEN, 0);
    st.set_w(FROG + f::SPUD_ON, 1);
    st.set_w(FROG + IN_FLIGHT, 1);
    // No wall test for many passes, no splat.
    st.set_w(FROG + SPUD_WAIT, 0x7000);
    st.set_l(SPUD_BOB + 0x42, h0(0x2cfd4));
    st.set_w(FROG + SPUD_X, cx.wrapping_add(sx as u16));
    st.set_w(FROG + SPUD_Y, cy.wrapping_add(0x80));
    st.set_w(FROG + SPUD_VX, 0);
    st.set_w(FROG + SPUD_VY, 0);
    // Not on its splat's last frame (the parked spud's bob may be).
    st.set_l(SPUD_BOB + 0x16, h0(0x2cfd4));
    // The frog far from the spud, so it does not catch it.
    st.set_w(FROG + f::X, cx.wrapping_add(0x60));
    st.set_w(FROG + f::Y, cy.wrapping_add(0x10));
    run(&mut st, screen, Hw::default(), powerups::spud_update);
    let x = st.w(FROG + SPUD_X);
    assert!(x == PARKED_X || st.w(FROG + IN_FLIGHT) == 1, "the spud neither parked nor flying");
    x == PARKED_X
}

#[test]
fn spud_flies_to_the_screens_edge_wide_or_narrow() {
    let n = Screen::NARROW;
    let w = Screen::WIDE;
    let e = E as i16;
    // Right: 4:3 parks past cx + $150, widescreen past cx + $150 + 65.
    assert!(!spud_parked_at(n, 0x150));
    assert!(spud_parked_at(n, 0x151));
    assert!(!spud_parked_at(w, 0x151));
    assert!(!spud_parked_at(w, 0x150 + e));
    assert!(spud_parked_at(w, 0x151 + e));
    // Left: 4:3 parks at cx and left of it, widescreen at cx − 65 and left of it.
    assert!(!spud_parked_at(n, 1));
    assert!(spud_parked_at(n, 0));
    assert!(!spud_parked_at(w, 0));
    assert!(!spud_parked_at(w, 1 - e));
    assert!(spud_parked_at(w, -e));
}

/// The bonus level's state, its frog live (shooter mode on, sprites shown, not dying).
fn bonus_state() -> State {
    let mut st = level_state("WB-1");
    assert_ne!(st.w(SHOOTER_MODE), 0, "the bonus level is in shooter mode");
    st.set_w(SPRITES_HIDDEN, 0);
    st.set_w(FROG + f::DYING, 0);
    st
}

fn shot(st: &State, k: u32) -> u32 {
    st.l(SHOTS + 4 * k)
}

#[test]
fn the_shots_spawn_past_the_screens_right_edge() {
    for screen in [Screen::NARROW, Screen::WIDE] {
        let mut st = bonus_state();
        // Room for five more.
        let old: Vec<u32> = (0..5).map(|k| shot(&st, k)).collect();
        run(&mut st, screen, Hw::default(), |c| old.iter().for_each(|&s| objects::obj_free(c, s, 0)));
        let (cx, _) = camera(&st);
        run(&mut st, screen, Hw::default(), shots::spawn_shots);
        for k in 0..5 {
            let want = cx.wrapping_add(0x160 + screen.extra() + 0x10 * k as u16);
            assert_eq!(st.w(shot(&st, k) + 8), want, "{screen:?} shot {k}");
        }
        // y keeps the original's slip from the narrow x (no vertical margin moves).
        assert_eq!(st.w(shot(&st, 0) + 0xa), cx.wrapping_add(0x160 + 0x130), "{screen:?}");
    }
}

/// Fire with the cycle's first shot at screen x `sx`: whether it was sent off again (put at the
/// frog).
fn refired(screen: Screen, sx: u16) -> bool {
    let mut st = bonus_state();
    let (cx, _) = camera(&st);
    let s = shot(&st, 0);
    st.set_w(s + 8, cx.wrapping_add(sx));
    st.set_w(SHOT_DELAY, 0);
    st.set_l(SHOT_CYCLE, SHOT_CYCLE_TABLE);
    let fx = st.w(FROG + f::X);
    assert_ne!(fx, cx.wrapping_add(sx));
    let hw = Hw { fire: true, ..Hw::default() };
    run(&mut st, screen, hw, shots::shooter_spud);
    st.w(s + 8) == fx
}

#[test]
fn a_shot_fires_again_once_past_the_screens_right_edge() {
    assert!(!refired(Screen::NARROW, 0x150));
    assert!(refired(Screen::NARROW, 0x151));
    assert!(!refired(Screen::WIDE, 0x151));
    assert!(!refired(Screen::WIDE, 0x150 + E));
    assert!(refired(Screen::WIDE, 0x151 + E));
}

/// One `shot_update` of the cycle's first shot at screen x `sx`, moving right by 8 while at or
/// left of the far mark, on the screen
/// (or off its top when `off`): its screen x after.
fn shot_after(screen: Screen, sx: u16, off: bool) -> u16 {
    let mut st = bonus_state();
    let (cx, cy) = camera(&st);
    let s = shot(&st, 0);
    st.set_w(s + 8, cx.wrapping_add(sx));
    st.set_w(s + 0xa, if off { cy.wrapping_sub(0x40) } else { cy.wrapping_add(0x80) });
    st.set_w(s + 0xc, 8);
    st.set_w(s + 0xe, 0);
    run(&mut st, screen, Hw::default(), |c| shots::shot_update(c, s, 0));
    st.w(s + 8).wrapping_sub(cx)
}

#[test]
fn a_shot_moves_until_and_parks_at_the_far_mark() {
    assert_eq!(shot_after(Screen::NARROW, 0x200, false), 0x208);
    assert_eq!(shot_after(Screen::NARROW, 0x201, false), 0x201);
    assert_eq!(shot_after(Screen::WIDE, 0x201, false), 0x209);
    assert_eq!(shot_after(Screen::WIDE, 0x200 + E, false), 0x208 + E);
    assert_eq!(shot_after(Screen::WIDE, 0x201 + E, false), 0x201 + E);
    assert_eq!(shot_after(Screen::NARROW, 0x80, true), 0x200);
    assert_eq!(shot_after(Screen::WIDE, 0x80, true), 0x200 + E);
}

/// A shot fired from the frog flies into the right strip and hits a bonus enemy there; until it
/// does, and until it is past the wide edge, the fire button does not send it again.
#[test]
fn a_shot_in_the_strip_hits_an_enemy_there() {
    let w = Screen::WIDE;
    let mut st = bonus_state();
    let (cx, cy) = camera(&st);
    let y = cy.wrapping_add(0x80);
    // The enemy in the right strip, past the narrow screen's despawn mark.
    let at = cx.wrapping_add(0x150 + 0x30);
    let enemy = run(&mut st, w, Hw::default(), |c| objects::obj_spawn_at_xy(c, template(113), at, y).a0);
    assert_ne!(enemy, 0);
    let s = shot(&st, 0);
    st.set_w(s + 8, cx.wrapping_add(0x100));
    // Level with the enemy where the spawn put it.
    st.set_w(s + 0xa, st.w(enemy + 0xa));
    st.set_w(s + 0xc, 8);
    st.set_w(s + 0xe, 0);
    let mut hit_at = None;
    for _ in 0..40 {
        let before = st.w(s + 8);
        let after = run(&mut st, w, Hw::default(), |c| {
            shots::shot_update(c, s, 0);
            let moved = c.bus.w(s + 8);
            objects::obj_build_hitboxes(c);
            shots::shot_touches(c);
            moved
        });
        if st.w(s + 8) != after {
            assert_eq!(st.w(s + 8), after.wrapping_add(0x280), "thrown out of play");
            hit_at = Some(after.wrapping_sub(cx));
            break;
        }
        assert_eq!(after, before.wrapping_add(8), "the shot moves through the strip");
    }
    let sx = hit_at.expect("the shot hit the enemy in the strip");
    assert!(sx > 0x150 && sx <= 0x150 + E, "hit at screen x {sx:#x}, in the right strip");
}

/// The frog's direction from a watcher: (`x0`, `y0`) to (`x1`, `y1`), screen coordinates.
#[test]
fn direction_clamps_to_the_wide_screen() {
    let (n, w) = (Screen::NARROW, Screen::WIDE);
    let neg = |v: u16| v.wrapping_neg();
    // On the narrow screen both agree.
    for (x0, y0, x1, y1) in [(0x40, 0x80, 0x100, 0x40), (0x100, 0x20, 0x10, 0xe0), (0x80, 0x80, 0x80, 0x10)] {
        assert_eq!(direction(n, x0, y0, x1, y1), direction(w, x0, y0, x1, y1));
    }
    // A frog in the right strip, a little up: the narrow clamp puts it level-ish above the eye
    // (steep), the wide one where it is (flatter).
    let (nd, wd) = (direction(n, 0x140, 0x80, 0x190, 0x40), direction(w, 0x140, 0x80, 0x190, 0x40));
    assert_eq!(wd, direction(n, 0x40, 0x80, 0x90, 0x40), "the unclamped direction");
    assert_ne!(nd, wd);
    // A frog in the left strip, level with an eye on the left edge: narrow clamps it onto the
    // eye's column; wide sees it to the left.
    assert_eq!(direction(w, 0, 0x80, neg(40), 0x78), direction(n, 40, 0x80, 0, 0x78));
    assert_ne!(direction(n, 0, 0x80, neg(40), 0x78), direction(w, 0, 0x80, neg(40), 0x78));
    // Past the wide edge it clamps again.
    assert_eq!(direction(w, 0x100, 0x80, 0x154 + E + 0x50, 0x40), direction(w, 0x100, 0x80, 0x153 + E, 0x40));
    assert_eq!(direction(w, 0x100, 0x80, neg(E + 0x50), 0x40), direction(w, 0x100, 0x80, neg(E), 0x40));
}

/// A W1 watcher's eye, through its own update, looks toward a frog in the right strip.
#[test]
fn a_watchers_eye_looks_at_a_frog_in_the_strip() {
    let looks = |screen: Screen| {
        let mut st = level_state("W1-1");
        let (cx, cy) = camera(&st);
        let (wx, wy) = (cx.wrapping_add(0x140), cy.wrapping_add(0x80));
        let slot = run(&mut st, screen, Hw::default(), |c| objects::obj_spawn_at_xy(c, template(76), wx, wy).a0);
        assert_ne!(slot, 0);
        // The update reads the slot's own x, y: put them back where the frog is measured from.
        st.set_w(slot + 8, wx);
        st.set_w(slot + 0xa, wy);
        st.set_w(FROG + f::X, cx.wrapping_add(0x190));
        st.set_w(FROG + f::Y, cy.wrapping_add(0x40));
        let update = watcher::WATCHER.routines[1].1;
        let d = direction(screen, 0x140, 0x80, 0x190, 0x40) as u32;
        run(&mut st, screen, Hw::default(), |c| {
            update(c, slot, 0);
            let bob = c.bus.l(slot + 0x50);
            // The looks are in the program's memory, read through the bus.
            (c.bus.l(bob + 0x42), c.bus.l(watcher::LOOKS + 4 * d))
        })
    };
    let (narrow, narrow_want) = looks(Screen::NARROW);
    let (wide, wide_want) = looks(Screen::WIDE);
    assert_eq!(narrow, narrow_want);
    assert_eq!(wide, wide_want);
    assert_ne!(narrow, wide, "the wide eye looks further right than the clamped narrow one");
}
