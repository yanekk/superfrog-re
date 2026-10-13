//! The bonus level's routines (`sf_core::bonus`, T75–T78) against the original's, on the routine
//! oracle.
//!
//! Each routine runs on Moira from the original's memory at the bonus level's first frame (WB-1:
//! shooter mode, its five shots live in the first five slots) and the port from the same memory
//! transplanted, over generated states. Every byte written, the sound driver's state and the Paula
//! writes must match (`sf_conform::routine`).
//!
//! Lives here, not in `sf-core/tests/`, because the purity test keeps the emulator out of the pure
//! crates' tests.

mod common;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_core::bonus::shots::{self, SHOT_CYCLE, SHOT_DELAY, SHOT_POWER, SHOTS};
use sf_core::frog::{FROG, f};
use sf_core::items::template;
use sf_core::layout::{LAYOUT, VarId, h0};
use sf_core::level::objects::{self, OBJ_SLOTS};
use sf_core::level::{Hw, LevelCtx};
use sf_core::mem::Bus;

const N: usize = 3000;

const ATTRACT_MODE: u32 = h0(0x18);
const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);
const SPRITES_HIDDEN: u32 = h0(0x9e2);
const SLOT: u32 = 0x7c;

/// The bytes `f` changes when the engine runs it from the rig's state with `pre` poked in (the
/// attract demo's flag set meanwhile, so no sound is started), as pokes.
fn prepared(rig: &Rig, pre: &[(u32, Vec<u8>)], f: impl FnOnce(&mut LevelCtx)) -> Vec<(u32, Vec<u8>)> {
    let base = rig.state().clone();
    let mut st = base.clone();
    for (a, b) in pre {
        st.poke(*a, b);
    }
    let attract = st.l(ATTRACT_MODE);
    st.set_l(ATTRACT_MODE, 0xffff_ffff);
    let display = sf_conform::display_data();
    let mut paula = Vec::new();
    {
        let mut c = LevelCtx {
            bus: Bus::new(&mut st, display.rom()),
            dispatch: sf_core::level::standard_dispatch(),
            hw: Hw::default(),
            sound: sound(),
            paula: &mut paula,
        };
        f(&mut c);
    }
    st.set_l(ATTRACT_MODE, attract);
    let mut out = Vec::new();
    for (d, id) in LAYOUT.iter().zip(VarId::ALL.iter()) {
        let (a, b) = (base.var(*id), st.var(*id));
        let mut k = 0;
        while k < a.len() {
            if a[k] == b[k] {
                k += 1;
                continue;
            }
            let start = k;
            while k < a.len() && a[k] != b[k] {
                k += 1;
            }
            out.push((d.addr + start as u32, b[start..k].to_vec()));
        }
    }
    out
}

/// The five shots' slots, as WB-1 starts.
fn shot_slots(rig: &Rig) -> [u32; 5] {
    std::array::from_fn(|k| rig.state().l(SHOTS + 4 * k as u32))
}

/// `v` moved by up to `span` either way, or now and then anywhere.
fn near(g: &mut Gen, v: u16, span: u16) -> u16 {
    if g.one_in(8) {
        g.next() as u16
    } else {
        v.wrapping_add(g.below(2 * span as u64 + 1) as u16).wrapping_sub(span)
    }
}

/// One of `choices`, or now and then any word.
fn pick_or_any(g: &mut Gen, choices: &[u16]) -> u16 {
    if g.one_in(choices.len() as u64 + 1) { g.next() as u16 } else { g.pick(choices) }
}

/// How many of `n` cases the original returned on: a crash or hang leaves a case uncompared
/// (DESIGN §2.11), and the generators must keep that rare.
fn enough(name: &str, compared: usize, n: usize) {
    assert!(compared * 100 >= n * 99, "{name}: only {compared} of {n} cases compared");
}

#[test]
fn the_rig_starts_with_the_five_shots_spawn_shots_made() {
    let rig = bonus_rig();
    let st = rig.state();
    assert_eq!(st.w(h0(0x9dc)), 1, "shooter mode");
    let slots = shot_slots(&rig);
    for (k, &s) in slots.iter().enumerate() {
        assert_eq!(s, OBJ_SLOTS + k as u32 * SLOT);
        assert_eq!(st.l(s + 4), h0(0x8070));
    }
    // The original's slip: y is x plus $130.
    assert_eq!(st.l(h0(0x7f0e)), st.l(h0(0x7f0a)) + 0x130);
}

#[test]
fn spawn_shots_matches_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(751);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup::default();
        // Free some of the slots (the shots among them), and now and then fill the table with
        // coins first, evictable or not, so the search, the evictions and a full table are met.
        let free: Vec<u32> = (0..20).filter(|_| g.one_in(2)).collect();
        let coins: Vec<(u32, u32, bool)> = (0..if g.one_in(3) { g.below(22) } else { 0 })
            .map(|_| (2 + g.below(290) as u32, 2 + g.below(78) as u32, g.one_in(2)))
            .collect();
        s.pokes = prepared(&rig, &[], |c| {
            for &k in &free {
                objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
            }
            for &(col, row, evict) in &coins {
                let r = objects::obj_spawn_cell(c, template(0), cell(col, row), 0, 0xd);
                if r.a0 != 0 && evict {
                    c.bus.set_w(r.a0 + 0x2a, 1);
                }
            }
        });
        // The camera where WB-1 spawned them (a live shot there is found again), or anywhere.
        let cx = if g.one_in(3) { 0x10 } else { g.below(0x1400) as u32 };
        s.poke_l(CAMERA_X, cx);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        if rig.check("spawn_shots", case, 0x7e3e, &s, shots::spawn_shots) {
            compared += 1;
        }
    }
    enough("spawn_shots", compared, N);
}

#[test]
fn spud_update_in_shooter_mode_matches_the_original() {
    let mut rig = bonus_rig();
    let slots = shot_slots(&rig);
    let mut g = Gen::new(752);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        if g.one_in(2) {
            s.hw.fire = true;
        }
        let cx = g.below(0x1200) as u16;
        let (fx, fy) = (cx + 0x20 + g.below(0x100) as u16, g.below(0x500) as u16);
        s.poke_l(CAMERA_X, cx as u32);
        s.poke_l(CAMERA_Y, fy.saturating_sub(0x80) as u32);
        s.poke_w(FROG + f::X, fx);
        s.poke_w(FROG + f::Y, fy);
        s.poke_w(FROG + 0x72, g.below(2) as u16);
        s.poke_w(FROG + 0x74, g.next() as u16);
        s.poke_w(FROG + f::DYING, g.one_in(10) as u16);
        s.poke_w(SPRITES_HIDDEN, g.one_in(10) as u16);
        s.poke_w(SHOT_POWER, g.pick(&[0, 0, 1, 2, 7]));
        s.poke_w(SHOT_DELAY, pick_or_any(&mut g, &[0, 0, 1, 2, 3, 0xffff, 0x8000]));
        s.poke_l(SHOT_CYCLE, h0(0x80cc) + 4 * g.below(4) as u32);
        // Each shot just inside or just past the screen's edge (camera x + $150), or anywhere.
        for &slot in &slots {
            s.poke_w(slot + 8, near(&mut g, cx.wrapping_add(0x150), 4));
            s.poke_w(slot + 0xa, g.next() as u16);
        }
        if rig.check("spud_update", case, 0x80f0, &s, sf_core::items::powerups::spud_update) {
            compared += 1;
        }
    }
    enough("spud_update", compared, N);
}

#[test]
fn shot_update_matches_the_original() {
    let mut rig = bonus_rig();
    let slots = shot_slots(&rig);
    let mut g = Gen::new(753);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let slot = g.pick(&slots);
        let (cx, cy) = (g.below(0x1200) as u16, g.below(0x500) as u16);
        s.poke_l(CAMERA_X, cx as u32);
        s.poke_l(CAMERA_Y, cy as u32);
        // Around the right limit (camera x + $200) and the top and bottom ones ($ffe0, $120).
        s.poke_w(slot + 8, near(&mut g, cx.wrapping_add(0x200), 0x30));
        let y = cy.wrapping_add(g.pick(&[0xffe0, 0x120, 0x80]));
        s.poke_w(slot + 0xa, near(&mut g, y, 0x10));
        s.poke_w(slot + 0xc, pick_or_any(&mut g, &[0x18, 0x1e, 0, 0xfff0]));
        s.poke_w(slot + 0xe, pick_or_any(&mut g, &[0, 0xfff4, 0xc]));
        s.a[0] = slot;
        if rig.check("shot_update", case, 0x8070, &s, |c| shots::shot_update(c, slot, 0)) {
            compared += 1;
        }
    }
    enough("shot_update", compared, N);
}

#[test]
fn shots_faster_matches_the_original() {
    let mut rig = bonus_rig();
    let slots = shot_slots(&rig);
    let mut g = Gen::new(754);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup::default();
        for &slot in &slots {
            s.poke_w(slot + 0xc, pick_or_any(&mut g, &[0x18, 0x1d, 0x1e, 0x1f, 0xffff]));
        }
        if rig.check("shots_faster", case, 0x8008, &s, shots::shots_faster) {
            compared += 1;
        }
    }
    enough("shots_faster", compared, N);
}

#[test]
fn collide_all_in_shooter_mode_matches_the_original() {
    let mut rig = bonus_rig();
    let slots = shot_slots(&rig);
    let mut g = Gen::new(755);
    let mut compared = 0;
    let mut hits = 0;
    for case in 0..N {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        // Coins, springs and a lock (objects whose touch is ported) around a spot, the shots over
        // them and the frog near.
        let (px, py) = (64 + g.below(4400) as u32, 64 + g.below(1100) as u32);
        let n = 1 + g.below(6);
        let mut pre = Vec::new();
        for _ in 0..n {
            let col = (px / 16 + g.below(5) as u32).saturating_sub(2);
            let row = (py / 16 + g.below(5) as u32).saturating_sub(2);
            pre.push((g.pick(&[0, 0, 2, 4, 5]), col, row, g.one_in(4)));
        }
        let pokes = prepared(&rig, &[], |c| {
            for &(t, col, row, reward) in &pre {
                let r = objects::obj_spawn_cell_alt(c, template(t), cell(col, row), 0, 0xd);
                if r.a0 != 0 && reward {
                    // A reward's update: shots pass through it.
                    c.bus.set_l(r.a0 + 4, h0(0x23e22));
                }
            }
            objects::obj_build_hitboxes(c);
        });
        s.pokes = pokes;
        for &slot in &slots {
            s.poke_w(slot + 8, near(&mut g, px as u16, 0x30));
            s.poke_w(slot + 0xa, near(&mut g, py as u16, 0x30));
        }
        let (fx, fy) = (near(&mut g, px as u16, 0x40), near(&mut g, py as u16, 0x40));
        s.poke_w(FROG + f::X, fx);
        s.poke_w(FROG + f::Y, fy);
        s.poke_w(FROG + f::VY, (g.below(30) as i16 - 18) as u16);
        s.poke_w(FROG + f::ON_GROUND, g.below(2) as u16);
        s.poke_w(FROG + f::HURT, g.one_in(6) as u16);
        s.poke_w(SPRITES_HIDDEN, g.one_in(10) as u16);
        s.poke_l(CAMERA_X, px.saturating_sub(0xa0));
        s.poke_l(CAMERA_Y, py.saturating_sub(0x80));
        if g.one_in(3) {
            s.poke_w(FROG + 0x74, near(&mut g, px as u16, 0x20));
            s.poke_w(FROG + 0x76, near(&mut g, py as u16, 0x20));
        }
        let mut hit = false;
        if rig.check("collide_all", case, 0xd0cc, &s, |c| {
            let before: Vec<u16> = slots.iter().map(|&k| c.bus.w(k + 8)).collect();
            objects::collide_all(c);
            hit = slots.iter().zip(&before).any(|(&k, &x)| c.bus.w(k + 8) != x);
        }) {
            compared += 1;
            hits += hit as usize;
        }
    }
    enough("collide_all", compared, N);
    assert!(hits * 10 >= N, "only {hits} of {N} cases had a shot hit something");
}
