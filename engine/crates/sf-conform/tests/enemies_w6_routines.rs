//! World 6's enemies (`sf_core::enemies::w6`, T40) and the touch handlers they share
//! (`sf_core::enemies::common`) against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at W6-1's first frame and the port from
//! the same memory transplanted, over generated states. Every byte written, the sound driver's
//! state and the Paula writes must match (`sf_conform::routine`). Slots are made by the engine's
//! own spawners, so a generated slot is as consistent as the game makes one.

mod common;

use std::sync::OnceLock;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_conform::{SyncPoint, sync};
use sf_core::enemies::w6::{cannon, flier, mace, thrower};
use sf_core::items::template;
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarId, h0};
use sf_core::level::objects::{self, OBJ_SLOTS};
use sf_core::level::{Hw, LevelCtx};
use sf_core::mem::Bus;

const N: usize = 3000;

const ATTRACT_MODE: u32 = h0(0x18);
const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);
const FROG: u32 = h0(0x7d6);
const SCORE: u32 = h0(0x834);
const SLOT: u32 = 0x7c;
/// W6's row of the dispatch.
const W6: u8 = 5;

/// The original's memory at W6-1's first frame.
fn w6_level() -> &'static AmigaMemory {
    static M: OnceLock<AmigaMemory> = OnceLock::new();
    M.get_or_init(|| {
        sync::reach(
            &SyncPoint::AtLevel("W6-1".parse().unwrap()),
            &sf_conform::InputScript::idle(),
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"))
        .memory
    })
}

fn w6_rig() -> Rig<'static> {
    static D: OnceLock<std::sync::Arc<sf_core::DisplayData>> = OnceLock::new();
    let d = D.get_or_init(sf_conform::display_data);
    Rig::new(w6_level(), sound(), d)
}

/// The bytes `f` changes when the engine runs it from the rig's state with `pre` poked in (the
/// attract demo's flag set meanwhile, so no sound is started), as pokes.
fn prepared(
    rig: &Rig,
    pre: &[(u32, Vec<u8>)],
    f: impl FnOnce(&mut LevelCtx),
) -> Vec<(u32, Vec<u8>)> {
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

/// The rig's state with `s`'s pokes in.
fn poked(rig: &Rig, s: &Setup) -> sf_core::State {
    let mut st = rig.state().clone();
    for (a, b) in &s.pokes {
        st.poke(*a, b);
    }
    st
}

fn enough(name: &str, compared: usize, n: usize) {
    assert!(
        compared * 100 >= n * 99,
        "{name}: only {compared} of {n} cases compared"
    );
}

fn any_cell(g: &mut Gen) -> (u32, u32) {
    (2 + g.below(290) as u32, 2 + g.below(70) as u32)
}

fn near(g: &mut Gen, v: u16, span: u16) -> u16 {
    if g.one_in(8) {
        g.next() as u16
    } else {
        v.wrapping_add(g.below(2 * span as u64 + 1) as u16)
            .wrapping_sub(span)
    }
}

fn pick_or_any(g: &mut Gen, choices: &[u16]) -> u16 {
    if g.one_in(choices.len() as u64 + 1) {
        g.next() as u16
    } else {
        g.pick(choices)
    }
}

/// Each type's template, init and update.
const TYPES: [(u32, u32, u32); 13] = [
    (57, 0x25e9a, 0x25fa8),
    (58, 0x25eb6, 0x25fa8),
    (67, 0x260c4, 0x2619c),
    (68, 0x2627e, 0x262be),
    (101, 0x28324, 0x28340),
    (102, 0x28458, 0x28488),
    (103, 0x281d8, 0x2826e),
    (104, 0x27fa0, 0x27fd4),
    (105, 0x2804c, 0x28086),
    (106, 0x27efa, 0x27f2e),
    (107, 0x27e00, 0x27e34),
    (108, 0x27e00, 0x27e8e),
    (109, 0x27e1a, 0x27e34),
];

/// Marker stubs: (marker, stub).
const STUBS: [(u8, u32); 9] = [
    (12, 0x1892a),
    (13, 0x1893a),
    (14, 0x1894a),
    (15, 0x1897e),
    (16, 0x1898e),
    (17, 0x189ae),
    (18, 0x1895a),
    (19, 0x1896c),
    (20, 0x1899e),
];

/// A generated table: some slots freed, now and then the rest filled with shots (107) so a spawn
/// finds no room (never under marker 17's: see `spawn_67`); then `t` spawned at a cell by the
/// engine (marker 17's stub for 67 and 68, the link picked for 68). The slot and its bob (0, 0
/// with no room).
fn with_slot(rig: &Rig, g: &mut Gen, s: &mut Setup, t: u32) -> (u32, u32) {
    let (col, row) = any_cell(g);
    let free: Vec<u32> = (0..20).filter(|_| g.one_in(3)).collect();
    let fill = if t != 67 && t != 68 && g.one_in(4) {
        g.below(21)
    } else {
        0
    };
    let fill_at: Vec<(u16, u16)> = (0..fill)
        .map(|_| (g.below(0x1300) as u16, g.below(0x500) as u16))
        .collect();
    let link = 1 + g.below(5) as u32;
    let pre = s.pokes.clone();
    let (mut slot, mut bob) = (0, 0);
    let pokes = prepared(rig, &pre, |c| {
        for &k in &free {
            objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
        }
        if t == 67 || t == 68 {
            for k in 0..20 {
                objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
            }
            let at = cell(col, row.min(60));
            mace::spawn_67(c, at);
            let want = if t == 67 { at } else { at + link * 0x260 };
            slot = (0..20)
                .map(|k| OBJ_SLOTS + k * SLOT)
                .find(|&s| c.bus.l(s + 0x40) == want)
                .unwrap_or(0);
        } else {
            slot = objects::obj_spawn_cell_alt(c, template(t), cell(col, row), 0, 0xa).a0;
        }
        if slot != 0 {
            bob = c.bus.l(slot + 0x50);
        }
        for &(x, y) in &fill_at {
            objects::obj_spawn_at_xy_nodedup(c, template(107), x, y);
        }
    });
    s.pokes = pokes;
    (slot, bob)
}

/// The camera somewhere near `slot`, and the frog too.
fn camera_and_frog_near(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32) {
    let st = poked(rig, s);
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    s.poke_l(CAMERA_X, near(g, x.saturating_sub(0xa0), 0x200) as u32);
    s.poke_l(CAMERA_Y, near(g, y.saturating_sub(0x80), 0x100) as u32);
    s.poke_w(FROG, near(g, x, 0xa0));
    s.poke_w(FROG + 2, near(g, y.wrapping_sub(0x30), 0x40));
    s.poke_w(FROG + 0x24, pick_or_any(g, &[0, 1]));
    s.poke_w(FROG + 0x3a, pick_or_any(g, &[0, 0, 1]));
}

/// An animation step for `bob` to show: its `+$16` pointed at `slot + $6a`, so the step's `+$06`
/// word is the slot's `+$70`, and its frame (the long it starts with) the slot's `+$6a`.
fn step_word(g: &mut Gen, s: &mut Setup, slot: u32, bob: u32) {
    s.poke_l(bob + 0x16, slot + 0x6a);
    s.poke_w(slot + 0x70, pick_or_any(g, &[0, 1, 2]));
    s.poke_l(
        slot + 0x6a,
        if g.one_in(2) {
            h0(0x2af0a)
        } else {
            h0(0x2aefe)
        },
    );
}

#[test]
fn the_kinds_and_markers_are_registered() {
    let d = sf_core::level::standard_dispatch();
    let m = w6_level();
    let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
    for (t, init, update) in TYPES {
        assert!(d.kind(t as u16).is_some(), "template {t}");
        let tl = template(t);
        assert_eq!(long(tl), h0(init), "template {t}'s init");
        assert_eq!(long(tl + 4), h0(update), "template {t}'s update");
        assert!(d.routine(long(tl + 0x10)).is_some(), "template {t}'s touch");
        assert!(d.routine(h0(init)).is_some() && d.routine(h0(update)).is_some());
    }
    for a in [0x25f42, 0x28532, 0x2813e, 0x23b34, 0x23bee, 0x23662] {
        assert!(d.routine(h0(a)).is_some(), "h0+${a:x}");
    }
    // Every W6 marker the jump table sends anywhere but the empty stub, but the witch's (1, T18).
    let jt = h0(0x178ec) + 0x500;
    let word = |a: u32| (m.byte(a) as u16) << 8 | m.byte(a + 1) as u16;
    for k in 0..64u32 {
        let to = jt + 4 * k + 2 + word(jt + 4 * k + 2) as i16 as i32 as u32;
        if to != h0(0x17fec) && k != 1 {
            assert!(d.marker(W6, k as u8).is_some(), "marker {k}");
        }
    }
}

#[test]
fn spawners_match_the_original() {
    let mut rig = w6_rig();
    let mut g = Gen::new(4001);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (m, entry) = g.pick(&STUBS);
        let (other, _) = with_slot(&rig, &mut g, &mut s, if m == 17 { 104 } else { 107 });
        let (col, row) = any_cell(&mut g);
        let at = if other != 0 && g.one_in(5) {
            rig.state().l(other + 0x40)
        } else {
            cell(col, row.min(60))
        };
        // Marker 17 needs room for its ball and links (see `spawn_67`).
        if m == 17 {
            let st = poked(&rig, &s);
            let free = (0..20)
                .filter(|&k| st.l(OBJ_SLOTS + k * SLOT + 0x40) == 0)
                .count();
            if free < 6 {
                continue;
            }
            s.poke_l(mace::CHAIN_LINK_COUNT, g.next() as u32);
        }
        cases += 1;
        s.poke_l(CAMERA_X, g.below(0x1300) as u32);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        s.a[3] = at;
        let spawn = sf_core::level::standard_dispatch()
            .marker(W6, m)
            .expect("registered");
        compared += rig.check(&format!("marker {m}"), case, entry, &s, |c| spawn(c, at)) as usize;
    }
    enough("spawners", compared, cases);
}

#[test]
fn inits_match_the_original() {
    let mut rig = w6_rig();
    let mut g = Gen::new(4002);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, init, _) = g.pick(&TYPES);
        let (slot, _) = with_slot(&rig, &mut g, &mut s, t);
        if slot == 0 {
            continue;
        }
        cases += 1;
        s.poke_w(slot + 8, near(&mut g, 0x800, 0x800));
        s.poke_w(slot + 0xa, near(&mut g, 0x200, 0x200));
        s.poke_l(mace::CHAIN_LINK_COUNT, g.below(7) as u32);
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(init))
            .expect("ported");
        compared += rig.check(&format!("init h0+${init:x}"), case, init, &s, |c| {
            port(c, slot, 0)
        }) as usize;
    }
    enough("inits", compared, cases);
}

/// The fields an update reads, varied.
fn vary(g: &mut Gen, s: &mut Setup, slot: u32, bob: u32, t: u32) {
    s.poke_w(
        slot + 0xc,
        pick_or_any(g, &[0, 3, 0xfffd, 5, 0xfffb, 8, 0xfff8, 0xfffa]),
    );
    s.poke_w(
        slot + 0xe,
        pick_or_any(g, &[0, 1, 0xffff, 5, 7, 8, 9, 0xfffb]),
    );
    s.poke_w(slot + 0x5c, pick_or_any(g, &[0, 1, 2, 0x40, 0xffff]));
    s.poke_w(slot + 0x60, pick_or_any(g, &[0, 1, 2, 7, 8, 9]));
    if t == 57 || t == 58 {
        s.poke_w(slot + 0x64, pick_or_any(g, &[0, 1, 2, 0x20]));
    }
    s.poke_w(slot + 0x68, pick_or_any(g, &[0, 1, 2, 0x10]));
    s.poke_w(
        bob + 0x1a,
        pick_or_any(g, &[0, 1, 2, 3, 0xf9f, 0xfa0, 0x7530, 0x752f]),
    );
    if g.one_in(2) {
        step_word(g, s, slot, bob);
    }
    match t {
        67 => {
            let n = 0x194 / 4;
            s.poke_l(slot + 0x54, mace::SWING + 4 * g.below(n) as u32);
            if g.one_in(4) {
                // Some links gone.
                for k in 0..20 {
                    if g.one_in(4) {
                        s.poke_l(OBJ_SLOTS + k * SLOT + 0x40, 0);
                    }
                }
            }
        }
        68 => {
            if g.one_in(8) {
                s.poke_l(slot + 0x68, g.next() as u32);
            }
        }
        103 => {
            let n = 0x64 / 4;
            s.poke_l(slot + 0x54, flier::FLIER_PATH + 4 * g.below(n) as u32);
        }
        _ => {}
    }
}

/// Each type's other states' updates, by template.
fn other_states(t: u32) -> &'static [u32] {
    match t {
        57 | 58 => &[0x25f42],
        102 => &[0x28532],
        105 => &[0x2813e],
        103 | 104 | 106 | 107 | 108 | 109 => &[0x23b34, 0x23bee],
        _ => &[],
    }
}

#[test]
fn updates_match_the_original() {
    let mut rig = w6_rig();
    let mut g = Gen::new(4003);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, _, mut update) = g.pick(&TYPES);
        let others = other_states(t);
        if !others.is_empty() && g.one_in(2) {
            update = g.pick(others);
        }
        let (slot, bob) = with_slot(&rig, &mut g, &mut s, t);
        if slot == 0 {
            continue;
        }
        cases += 1;
        vary(&mut g, &mut s, slot, bob, t);
        if update == 0x23b34 {
            // A falling enemy's speed stays within -1..=8; any other sends its map look-up off
            // the map, into memory the engine does not keep.
            s.poke_w(slot + 0xe, (g.below(10) as u16).wrapping_sub(1));
        }
        camera_and_frog_near(&rig, &mut g, &mut s, slot);
        s.poke_l(SCORE, g.below(100_000) as u32);
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(update))
            .expect("ported");
        compared += rig.check(
            &format!("update h0+${update:x} (template {t})"),
            case,
            update,
            &s,
            |c| port(c, slot, 0),
        ) as usize;
    }
    enough("updates", compared, cases);
}

#[test]
fn touches_match_the_original() {
    let mut rig = w6_rig();
    let mut g = Gen::new(4004);
    let mut compared = 0;
    let mut cases = 0;
    let touches = [0x23582, 0x2360e, 0x23662, 0x236ac];
    for case in 0..N {
        let mut s = Setup::default();
        let (t, _, _) = g.pick(&TYPES);
        let touch = g.pick(&touches);
        let (slot, bob) = with_slot(&rig, &mut g, &mut s, t);
        if slot == 0 {
            continue;
        }
        cases += 1;
        vary(&mut g, &mut s, slot, bob, t);
        camera_and_frog_near(&rig, &mut g, &mut s, slot);
        s.poke_w(
            slot + 0x1c,
            pick_or_any(&mut g, &[0x1f, 0x20, 0x21, 0x3f, 0x7d00, 0]),
        );
        let d7 = g.below(2) as u16;
        s.d[7] = d7 as u32;
        s.a[5] = slot;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(touch))
            .expect("ported");
        compared += rig.check(
            &format!("touch h0+${touch:x} (template {t})"),
            case,
            touch,
            &s,
            |c| port(c, slot, d7),
        ) as usize;
    }
    enough("touches", compared, cases);
}

#[test]
fn the_aims_and_counters_are_layout_variables() {
    for a in [
        mace::CHAIN_LINK_COUNT,
        thrower::THROWER_VX,
        flier::FLIER_FACING_RIGHT,
        cannon::CANNON_AIM,
    ] {
        assert!(sf_core::layout::index_of(a).is_some(), "${a:x}");
    }
}
