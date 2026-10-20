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
use sf_core::bonus::death::{
    REWARD_FLOAT, REWARD_LIFE_COUNT, REWARD_POWER_COUNT, TOUCH_BONUS_ENEMY,
};
use sf_core::bonus::shots::{self, SHOT_CYCLE, SHOT_DELAY, SHOT_POWER, SHOTS};
use sf_core::frog::{FROG, f};
use sf_core::items::template;
use sf_core::layout::{LAYOUT, Ram, VarId, h0};
use sf_core::level::objects::{self, OBJ_SLOTS};
use sf_core::level::{Hw, LevelCtx, n_clear};
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
        let mut c = LevelCtx::new(
            Bus::new(&mut st, display.rom()),
            sf_core::level::standard_dispatch(),
            Hw::default(),
            sound(),
            &mut paula,
        );
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

// What a shot enemy turns into (`sf_core::bonus::death`, T78).

const SHOOTER_MODE: u32 = h0(0x9dc);
const MAP_ROWS: u32 = h0(0x17334);
const RNG_STATE: u32 = h0(0x160d4);
const FROG_LIVES: u32 = h0(0x82e);
const WAVE: u32 = h0(0x23de6);
const WAVE_END: u32 = h0(0x23e1e);
const ARC: u32 = h0(0x23e90);
const ARC_END: u32 = h0(0x23eaa);
const LOOKS: [u32; 3] = [h0(0x2cf58), h0(0x2cf64), h0(0x2cf70)];

/// A coin spawned at (`col`, `row`) in the first free slot, as pokes, its slot and its bob.
fn spawned(rig: &Rig, col: u32, row: u32) -> (Vec<(u32, Vec<u8>)>, u32, u32) {
    let (mut slot, mut bob) = (0, 0);
    let pokes = prepared(rig, &[], |c| {
        slot = objects::obj_spawn_cell_alt(c, template(0), cell(col, row), 0, 0xd).a0;
        bob = c.bus.l(slot + 0x50);
    });
    assert_ne!(slot, 0);
    (pokes, slot, bob)
}

/// A slot's position, the camera near it (so the despawn check goes either way) and the frog.
fn place(g: &mut Gen, s: &mut Setup, slot: u32, x: u16, y: u16) {
    s.poke_w(slot + 8, x);
    s.poke_w(slot + 0xa, y);
    s.poke_l(CAMERA_X, near(g, x.wrapping_sub(0xa0), 0x120) as u32 & 0xffff);
    s.poke_l(CAMERA_Y, near(g, y.wrapping_sub(0x80), 0xc0) as u32 & 0xffff);
}

/// A map cell's word: a tile and a marker, the solid ($3a..) and slope ($3e, $3f) ones often.
fn any_cell(g: &mut Gen) -> u16 {
    let marker = pick_or_any(g, &[0, 0, 0x18, 0x3a, 0x3b, 0x3d, 0x3e, 0x3f]) & 0x3f;
    (g.below(840) as u16) << 6 | marker
}

/// The map cells [`become_reward`](sf_core::bonus::death::become_reward) looks at for a slot at
/// (`x`, `y`) moved `dx`: the column's cell two above, one above, its own and the one below.
fn poke_ground(rig: &Rig, g: &mut Gen, s: &mut Setup, x: u16, y: u16, dx: u16) {
    let col = x.wrapping_add(dx).wrapping_sub(8) >> 4;
    let row = (y & 0xfff0).wrapping_add(0x10) >> 4;
    let _ = rig;
    let at = MAP_ROWS + 4 * row as u32;
    let m = bonus_level();
    let rows = u32::from_be_bytes(std::array::from_fn(|k| m.byte(at + k as u32)));
    let own = rows + 2 * col as u32 + 2;
    for k in [-2i32, -1, 0, 1] {
        let v = any_cell(g);
        s.poke_w(own.wrapping_add((k * 0x260) as u32), v);
    }
}

#[test]
fn touch_bonus_enemy_matches_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(781);
    let mut compared = 0;
    for case in 0..N {
        let (pokes, slot, _) = spawned(&rig, 20 + g.below(250) as u32, 4 + g.below(70) as u32);
        let mut s = Setup {
            pokes,
            ..Setup::default()
        };
        s.poke_w(slot + 0x1c, pick_or_any(&mut g, &[0, 1, 2, 3, 0xffff, 0x7fff, 0x8000]));
        s.poke_w(SHOT_POWER, pick_or_any(&mut g, &[0, 1, 2, 3, 4]));
        s.a[5] = slot;
        s.a[0] = FROG;
        if rig.check("touch_bonus_enemy", case, 0x238e2, &s, |c| {
            sf_core::bonus::death::touch_bonus_enemy(c, slot, 0)
        }) {
            compared += 1;
        }
    }
    enough("touch_bonus_enemy", compared, N);
}

/// The RNG anywhere, shooter mode on or off, and the reward counters around their turns.
fn reward_setup(g: &mut Gen, s: &mut Setup) {
    s.poke_l(RNG_STATE, g.next() as u32);
    s.poke_l(RNG_STATE + 4, g.next() as u32);
    s.poke_w(SHOOTER_MODE, g.below(2) as u16);
    s.poke_w(REWARD_LIFE_COUNT, pick_or_any(g, &[0, 0x26, 0x27, 0x28, 3]));
    s.poke_w(REWARD_POWER_COUNT, pick_or_any(g, &[0, 8, 9, 10, 9]));
}

#[test]
fn death_fall_matches_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(782);
    let mut compared = 0;
    let mut became = 0;
    for case in 0..N {
        let (col, row) = (20 + g.below(250) as u32, 4 + g.below(70) as u32);
        let (pokes, slot, bob) = spawned(&rig, col, row);
        let mut s = Setup {
            pokes,
            ..Setup::default()
        };
        let (x, y) = (near(&mut g, (col * 16) as u16, 0x10), near(&mut g, (row * 16) as u16, 0x10));
        let (x, y) = (x.clamp(0x140, 0x1180), y.clamp(0x40, 0x4a0));
        place(&mut g, &mut s, slot, x, y);
        poke_ground(&rig, &mut g, &mut s, x, y, 8);
        reward_setup(&mut g, &mut s);
        let fall = pick_or_any(&mut g, &[0x7530, 0x752f, 0x7531, 0x8000, 0x7fff, 0]);
        s.poke_w(bob + 0x1a, fall);
        s.a[0] = slot;
        if rig.check("death_fall", case, 0x23956, &s, |c| {
            sf_core::bonus::death::death_fall(c, slot, 0)
        }) {
            compared += 1;
            became += n_clear(fall.wrapping_sub(0x7530)) as usize;
        }
    }
    enough("death_fall", compared, N);
    assert!(became * 4 >= N, "only {became} of {N} cases became a reward");
}

#[test]
fn become_reward_matches_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(783);
    let mut compared = 0;
    let mut kinds = [[0usize; 3]; 2];
    for case in 0..N {
        let (col, row) = (20 + g.below(250) as u32, 4 + g.below(70) as u32);
        let (pokes, slot, bob) = spawned(&rig, col, row);
        let mut s = Setup {
            pokes,
            ..Setup::default()
        };
        let x = (near(&mut g, (col * 16) as u16, 0x10)).clamp(0x140, 0x1180);
        let y = (near(&mut g, (row * 16) as u16, 0x10)).clamp(0x40, 0x4a0);
        place(&mut g, &mut s, slot, x, y);
        let dx = pick_or_any(&mut g, &[8, 0, 0xfff8, 0x10]) & 0x1f;
        poke_ground(&rig, &mut g, &mut s, x, y, dx);
        reward_setup(&mut g, &mut s);
        s.poke_l(slot + 0xc, g.next() as u32);
        s.d[0] = dx as u32;
        s.a[0] = slot;
        let shooter = s.pokes.iter().rev().find(|p| p.0 == SHOOTER_MODE).unwrap().1[1] as usize;
        let mut look = 0;
        if rig.check("become_reward", case, 0x23c14, &s, |c| {
            sf_core::bonus::death::become_reward(c, slot, dx);
            look = c.bus.l(bob + 0x28);
        }) {
            compared += 1;
            if let Some(k) = LOOKS.iter().position(|&l| l == look) {
                kinds[shooter][k] += 1;
            }
        }
    }
    enough("become_reward", compared, N);
    for (mode, k) in kinds.iter().enumerate() {
        assert!(k.iter().all(|&n| n > 0), "shooter mode {mode}: kinds met {k:?}");
    }
}

#[test]
fn reward_float_matches_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(784);
    let mut compared = 0;
    for case in 0..N {
        let (col, row) = (20 + g.below(250) as u32, 4 + g.below(70) as u32);
        let (pokes, slot, _) = spawned(&rig, col, row);
        let mut s = Setup {
            pokes,
            ..Setup::default()
        };
        s.poke_w(SHOOTER_MODE, 1);
        s.poke_w(slot + 8, near(&mut g, (col * 16) as u16, 0x10));
        s.poke_w(slot + 0x60, near(&mut g, (row * 16) as u16, 0x10));
        // Along the wave, at its end now and then.
        let at = if g.one_in(4) { WAVE_END } else { WAVE + 2 * g.below(0x1c) as u32 };
        s.poke_l(slot + 0x6c, at);
        s.poke_l(slot + 0x70, WAVE);
        s.poke_w(slot + 0x5c, pick_or_any(&mut g, &[1, 2, 0x20, 0]));
        s.a[0] = slot;
        if rig.check("reward_float", case, 0x23e22, &s, |c| {
            sf_core::bonus::death::reward_float(c, slot, 0)
        }) {
            compared += 1;
        }
    }
    enough("reward_float", compared, N);
}

#[test]
fn reward_arc_matches_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(785);
    let mut compared = 0;
    for case in 0..N {
        let (pokes, slot, _) = spawned(&rig, 20 + g.below(250) as u32, 4 + g.below(70) as u32);
        let mut s = Setup {
            pokes,
            ..Setup::default()
        };
        s.poke_w(SHOOTER_MODE, 0);
        s.poke_l(slot + 0x64, g.next() as u32);
        let at = if g.one_in(4) { ARC_END } else { ARC + 2 * g.below(0xe) as u32 };
        s.poke_l(slot + 0x54, at);
        s.a[0] = slot;
        if rig.check("reward_arc", case, 0x23e6c, &s, |c| {
            sf_core::bonus::death::reward_arc(c, slot, 0)
        }) {
            compared += 1;
        }
    }
    enough("reward_arc", compared, N);
}

#[test]
fn touch_reward_matches_the_original() {
    let mut rig = bonus_rig();
    let slots = shot_slots(&rig);
    let mut g = Gen::new(786);
    let mut compared = 0;
    for case in 0..N {
        let (pokes, slot, bob) = spawned(&rig, 20 + g.below(250) as u32, 4 + g.below(70) as u32);
        let mut s = Setup {
            pokes,
            ..Setup::default()
        };
        s.poke_w(SHOOTER_MODE, g.below(2) as u16);
        let at = if g.one_in(4) { ARC + 2 * g.below(0xe) as u32 } else { ARC_END };
        s.poke_l(slot + 0x54, if g.one_in(8) { h0(0x23e68) } else { at });
        let look = if g.one_in(8) { h0(0x2ce04) } else { g.pick(&LOOKS) };
        s.poke_l(bob + 8, look);
        s.poke_w(SHOT_POWER, pick_or_any(&mut g, &[0, 0, 1, 2]));
        s.poke_w(FROG_LIVES, small_word(&mut g));
        s.poke_l(FROG + 0x5e, if g.one_in(4) { g.next() as u32 } else { g.below(100_000) as u32 });
        s.poke_w(FROG + 0x7e, small_word(&mut g));
        for &k in &slots {
            s.poke_w(k + 0xc, pick_or_any(&mut g, &[0x18, 0x1d, 0x1e]));
        }
        s.a[5] = slot;
        s.a[0] = FROG;
        if rig.check("touch_reward", case, 0x23eac, &s, |c| {
            sf_core::bonus::death::touch_reward(c, slot, 0)
        }) {
            compared += 1;
        }
    }
    enough("touch_reward", compared, N);
}

#[test]
fn collide_all_with_a_shot_hitting_a_bonus_enemy_matches_the_original() {
    let mut rig = bonus_rig();
    let slots = shot_slots(&rig);
    let mut g = Gen::new(787);
    let mut compared = 0;
    let (mut hits, mut kills) = (0, 0);
    for case in 0..N {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        // Bonus enemies (coins given the touch and the hit points of one) around a spot, now
        // and then a reward among them, the shots over them.
        let (px, py) = (0x140 + g.below(4000) as u32, 64 + g.below(1100) as u32);
        let n = 1 + g.below(5);
        let mut pre = Vec::new();
        for _ in 0..n {
            let col = (px / 16 + g.below(5) as u32).saturating_sub(2);
            let row = (py / 16 + g.below(5) as u32).saturating_sub(2);
            let hp = pick_or_any(&mut g, &[0, 1, 2, 5]);
            pre.push((col, row, hp, g.one_in(6)));
        }
        let mut enemies = Vec::new();
        s.pokes = prepared(&rig, &[], |c| {
            for &(col, row, hp, reward) in &pre {
                let r = objects::obj_spawn_cell_alt(c, template(0), cell(col, row), 0, 0xd);
                if r.a0 == 0 {
                    continue;
                }
                c.bus.set_l(r.a0 + 0x10, TOUCH_BONUS_ENEMY);
                c.bus.set_w(r.a0 + 0x1c, hp);
                if reward {
                    c.bus.set_l(r.a0 + 4, REWARD_FLOAT);
                } else {
                    enemies.push(r.a0);
                }
            }
            objects::obj_build_hitboxes(c);
        });
        for &slot in &slots {
            s.poke_w(slot + 8, near(&mut g, px as u16, 0x30));
            s.poke_w(slot + 0xa, near(&mut g, py as u16, 0x30));
        }
        s.poke_w(SHOT_POWER, pick_or_any(&mut g, &[0, 1, 2, 3]));
        s.poke_w(FROG + f::X, near(&mut g, px as u16, 0x40));
        s.poke_w(FROG + f::Y, near(&mut g, py as u16, 0x40));
        s.poke_w(SPRITES_HIDDEN, g.one_in(10) as u16);
        s.poke_l(CAMERA_X, px.saturating_sub(0xa0));
        s.poke_l(CAMERA_Y, py.saturating_sub(0x80));
        let (mut hit, mut killed) = (false, false);
        if rig.check("collide_all", case, 0xd0cc, &s, |c| {
            let before: Vec<u16> = enemies.iter().map(|&k| c.bus.w(k + 0x1c)).collect();
            objects::collide_all(c);
            hit = enemies.iter().zip(&before).any(|(&k, &v)| c.bus.w(k + 0x1c) != v);
            killed = enemies.iter().any(|&k| c.bus.l(k + 4) == h0(0x23956));
        }) {
            compared += 1;
            hits += hit as usize;
            kills += killed as usize;
        }
    }
    enough("collide_all", compared, N);
    assert!(hits * 5 >= N, "only {hits} of {N} cases had a shot hit a bonus enemy");
    assert!(kills * 10 >= N, "only {kills} of {N} cases killed one");
}

/// `bonus_visit` (h0+$8cd6), `level_start_y` (h0+$a1e).
const BONUS_VISIT: u32 = h0(0x8cd6);
const START_Y: u32 = h0(0xa1e);

/// `bonus_frame` (h0+$6a64, T79), after `frog_reset` as `top_loop` calls it after a death:
/// the frog's step, the camera, the panel's digits, lives count and icon, and the frog back on
/// the screen, from any lives, coins, icon, camera and input.
#[test]
fn bonus_frame_matches_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(791);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        s.pokes = prepared(&rig, &[], |c| sf_core::frog::frog_reset(&mut c.bus));
        let cx = 0x10 + g.below(0x1000) as u16;
        let cy = g.below(0x300) as u16;
        s.poke_l(CAMERA_X, cx as u32);
        s.poke_l(CAMERA_Y, cy as u32);
        s.poke_w(FROG + f::X, near(&mut g, cx.wrapping_add(0x80), 0x100));
        s.poke_w(FROG + f::Y, near(&mut g, cy.wrapping_add(0x5c), 0x80));
        s.poke_w(FROG + 0x58, pick_or_any(&mut g, &[0, 1, 2, 4, 5, 6, 9, 60]));
        s.poke_w(FROG + 0x5a, g.below(6) as u16);
        s.poke_w(FROG + 0x5c, g.below(100) as u16);
        s.poke_w(h0(0x87c0), pick_or_any(&mut g, &[0, 4, 0xffff]));
        s.poke_w(SPRITES_HIDDEN, g.one_in(2) as u16);
        if rig.check("bonus_frame", case, 0x6a64, &s, sf_core::bonus::frame::bonus_frame) {
            compared += 1;
        }
    }
    enough("bonus_frame", compared, N);
}

/// `level_init_lb` (h0+$8c3a, T79): the bonus level set up again, its start row chosen by
/// `bonus_visit` (0 or 1 the top, 2 and 3 lower, cleared from 3 on), the five shots spawned.
#[test]
fn level_init_lb_matches_the_original() {
    let mut rig = bonus_rig();
    let id: sf_assets::LevelId = "WB-1".parse().unwrap();
    let mut rows = std::collections::BTreeSet::new();
    for (case, visit) in [0u16, 1, 2, 3, 4, 7, 0x7fff, 0x8000, 0xffff].into_iter().enumerate() {
        let mut s = Setup::default();
        s.poke_w(BONUS_VISIT, visit);
        let mut row = 0;
        assert!(
            rig.check("level_init_lb", case, 0x8c3a, &s, |c| {
                sf_core::level::load::level_init(c, id);
                row = c.bus.l(START_Y);
            }),
            "level_init_lb returns (bonus_visit {visit:x})"
        );
        rows.insert(row);
    }
    assert_eq!(rows.into_iter().collect::<Vec<_>>(), [0, 0x120, 0x220]);
}
