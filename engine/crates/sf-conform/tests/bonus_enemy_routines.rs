//! The bonus level's enemies of markers 1–6 (`sf_core::bonus::enemies`, T76) and of markers 7–15
//! and 55 (T77) against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at the bonus level's first frame (WB-1)
//! and the port from the same memory transplanted, over generated states. Every byte written, the
//! sound driver's state and the Paula writes must match (`sf_conform::routine`). Slots are made by
//! the engine's own spawners, so a generated slot is as consistent as the game makes one.
//!
//! Lives here, not in `sf-core/tests/`, because the purity test keeps the emulator out of the pure
//! crates' tests.

mod common;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_core::bonus::enemies::{FLY_OFF, TOGGLE_113, TOGGLE_114};
use sf_core::items::template;
use sf_core::layout::{LAYOUT, Ram, VarId, h0};
use sf_core::level::objects::{self, OBJ_SLOTS};
use sf_core::level::{Hw, LevelCtx};
use sf_core::mem::Bus;

const N: usize = 3000;

const ATTRACT_MODE: u32 = h0(0x18);
const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);
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

/// How many of `n` cases the original returned on: a crash or hang leaves a case uncompared
/// (DESIGN §2.11), and the generators must keep that rare.
fn enough(name: &str, compared: usize, n: usize) {
    assert!(compared * 100 >= n * 99, "{name}: only {compared} of {n} cases compared");
}

/// A map cell somewhere on the level (0-indexed column and row, away from its edges).
fn any_cell(g: &mut Gen) -> (u32, u32) {
    (2 + g.below(290) as u32, 2 + g.below(78) as u32)
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

/// The steps of the path at `start`, `stride` bytes apart, up to and with its end mark `end`.
fn path_len(start: u32, stride: u32, end: u16) -> u32 {
    path_len_by(start, stride, |w| w == end)
}

/// The steps of the path at `start`, `stride` bytes apart, up to and with the first word that
/// `is_end`.
fn path_len_by(start: u32, stride: u32, is_end: impl Fn(u16) -> bool) -> u32 {
    let m = bonus_level();
    let word = |a: u32| (m.byte(a) as u16) << 8 | m.byte(a + 1) as u16;
    (0..).find(|&k| is_end(word(start + stride * k))).expect("an end mark") + 1
}

/// A generated table: some slots freed (the five shots among them), now and then the rest filled
/// with 123s so a spawn finds no room; then `t` spawned at a cell by the engine. The slot and its
/// bob (0, 0 with no room).
fn with_slot(rig: &Rig, g: &mut Gen, s: &mut Setup, t: u32) -> (u32, u32) {
    let (col, row) = any_cell(g);
    let free: Vec<u32> = (0..20).filter(|_| g.one_in(3)).collect();
    let fill = if g.one_in(4) { g.below(21) } else { 0 };
    let fill_at: Vec<(u16, u16)> = (0..fill)
        .map(|_| (g.below(0x1300) as u16, g.below(0x500) as u16))
        .collect();
    let toggles = (g.next() as u16, g.next() as u16);
    let pre = s.pokes.clone();
    let (mut slot, mut bob) = (0, 0);
    let pokes = prepared(rig, &pre, |c| {
        c.bus.set_w(TOGGLE_113, toggles.0);
        c.bus.set_w(TOGGLE_114, toggles.1);
        for &k in &free {
            objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
        }
        let r = objects::obj_spawn_cell_alt(c, t, cell(col, row), 0, 0xa);
        slot = r.a0;
        if slot != 0 {
            bob = c.bus.l(slot + 0x50);
        }
        for &(x, y) in &fill_at {
            objects::obj_spawn_at_xy_nodedup(c, template(123), x, y);
        }
    });
    s.pokes = pokes;
    (slot, bob)
}

/// The camera somewhere near `slot`'s position, or anywhere.
fn camera_near(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32) {
    let mut st = rig.state().clone();
    for (a, b) in &s.pokes {
        st.poke(*a, b);
    }
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    s.poke_l(CAMERA_X, near(g, x.saturating_sub(0xa0), 0x200) as u32);
    s.poke_l(CAMERA_Y, near(g, y.saturating_sub(0x80), 0x100) as u32);
}

/// Each type's template, init and update.
const TYPES: [(u32, u32, u32); 11] = [
    (113, 0x28af2, 0x28b4c),
    (114, 0x28a7a, 0x28ada),
    (115, 0x28d52, 0x28d80),
    (116, 0x28cd4, 0x28cf0),
    (117, 0x28e3c, 0x28e52),
    (118, 0x28e3c, 0x28ec6),
    (119, 0x28e3c, 0x28ef0),
    (120, 0x28f1a, 0x28f36),
    (121, 0x28f7a, 0x28fae),
    (122, 0x28fd8, 0x29044),
    (123, 0x2916a, 0x29180),
];

#[test]
fn the_kinds_and_markers_are_registered() {
    let d = sf_core::level::standard_dispatch();
    for (t, init, update) in TYPES {
        assert!(d.kind(t as u16).is_some(), "template {t}");
        let tl = template(t);
        let m = bonus_level();
        let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
        assert_eq!(long(tl), h0(init), "template {t}'s init");
        assert_eq!(long(tl + 4), h0(update), "template {t}'s update");
        assert!(d.routine(h0(init)).is_some() && d.routine(h0(update)).is_some());
    }
    assert!(d.routine(h0(0x28eaa)).is_some());
    for m in 1..=6 {
        assert!(d.marker(6, m).is_some(), "marker {m}");
    }
}

#[test]
fn spawners_match_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(761);
    let mut compared = 0;
    let stubs: [(u8, u32); 6] = [
        (1, 0x18ac8),
        (2, 0x18ad8),
        (3, 0x18ae8),
        (4, 0x18af8),
        (5, 0x18b08),
        (6, 0x18b18),
    ];
    for case in 0..N {
        let mut s = Setup::default();
        let (m, entry) = g.pick(&stubs);
        // A table as `with_slot` makes one, with a 123 standing in.
        let (other, _) = with_slot(&rig, &mut g, &mut s, template(123));
        let (col, row) = any_cell(&mut g);
        let at = if other != 0 && g.one_in(5) {
            rig.state().l(other + 0x40)
        } else {
            cell(col, row)
        };
        s.poke_l(CAMERA_X, g.below(0x1300) as u32);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        s.poke_w(TOGGLE_113, g.next() as u16);
        s.poke_w(TOGGLE_114, g.next() as u16);
        s.a[3] = at;
        let spawn = sf_core::level::standard_dispatch().marker(6, m).expect("registered");
        compared += rig.check(&format!("marker {m}"), case, entry, &s, |c| spawn(c, at)) as usize;
    }
    enough("spawners", compared, N);
}

#[test]
fn inits_match_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(762);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, init, _) = g.pick(&TYPES);
        let (slot, _) = with_slot(&rig, &mut g, &mut s, template(t));
        if slot == 0 {
            continue;
        }
        cases += 1;
        camera_near(&rig, &mut g, &mut s, slot);
        s.poke_w(TOGGLE_113, g.next() as u16);
        s.poke_w(TOGGLE_114, g.next() as u16);
        s.poke_w(slot + 0xa, near(&mut g, 0x200, 0x200));
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch().routine(h0(init)).expect("ported");
        compared += rig.check(&format!("init h0+${init:x}"), case, init, &s, |c| port(c, slot, 0))
            as usize;
    }
    enough("inits", compared, cases);
}

/// The fields a type's update reads, varied.
fn vary(g: &mut Gen, s: &mut Setup, slot: u32, bob: u32, t: u32) {
    s.poke_w(slot + 0xc, pick_or_any(g, &[0, 1, 0xffff, 0xfffe, 0xfffa, 8, 0xfff8, 3]));
    s.poke_w(slot + 0xe, pick_or_any(g, &[0, 1, 0xffff, 2, 0xfffe, 6, 0xfffb, 8, 0xfff8]));
    s.poke_w(slot + 0x5c, pick_or_any(g, &[0, 1, 2, 3, 0xffff, 0xc, 0x1e]));
    s.poke_w(slot + 0x60, pick_or_any(g, &[1, 2, 0x20, 0x28, 0, 0xffff]));
    match t {
        113 => {
            let n = path_len(h0(0x28b8a), 4, 0xff80);
            s.poke_l(slot + 0x54, h0(0x28b8a) + 4 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, h0(0x28b8a));
            s.poke_w(slot + 0x66, near(g, 0x100, 0x100));
        }
        122 => {
            let n = path_len(h0(0x2900a), 2, 0x80);
            s.poke_l(slot + 0x64, h0(0x2900a) + 2 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x68, h0(0x2900a));
            s.poke_w(slot + 0x5c, near(g, 0x100, 0x100));
        }
        116 => s.poke_w(bob + 0x1a, pick_or_any(g, &[0, 0x7530, 0x752f, 0x7531, 0x8000])),
        _ => {}
    }
}

#[test]
fn updates_match_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(763);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, _, mut update) = g.pick(&TYPES);
        // A 117, 118 or 119 that has stopped drifts back.
        if (117..=119).contains(&t) && g.one_in(3) {
            update = 0x28eaa;
        }
        let (slot, bob) = with_slot(&rig, &mut g, &mut s, template(t));
        if slot == 0 {
            continue;
        }
        cases += 1;
        vary(&mut g, &mut s, slot, bob, t);
        camera_near(&rig, &mut g, &mut s, slot);
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch().routine(h0(update)).expect("ported");
        compared += rig.check(&format!("update h0+${update:x}"), case, update, &s, |c| {
            port(c, slot, 0)
        }) as usize;
    }
    enough("updates", compared, cases);
}

#[test]
fn the_paths_end_where_the_listing_says() {
    assert_eq!(path_len(h0(0x28b8a), 4, 0xff80), 0x53);
    assert_eq!(path_len(h0(0x2900a), 2, 0x80), 0x1d);
}

// Markers 7–15 and 55 (T77).

const LEVEL_STATE: u32 = h0(0xa06);
const PATH_124: u32 = h0(0x291e2);
const PATH_WAVE: u32 = h0(0x3b48a);
const PATH_CHAIN: u32 = h0(0x3b63e);
const PATH_131: u32 = h0(0x29536);

/// Each type's template, init and update (markers 7–15).
const TYPES_B: [(u32, u32, u32); 10] = [
    (124, 0x291a0, 0x2921c),
    (125, 0x2924a, 0x29280),
    (126, 0x292a0, 0x292d2),
    (127, 0x29348, 0x293d0),
    (128, 0x29388, 0x293d0),
    (129, 0x29434, 0x29460),
    (130, 0x2949c, 0x294c8),
    (131, 0x29504, 0x29570),
    (132, 0x29666, 0x2967c),
    (133, 0x2969c, 0x296ea),
];

/// The chain's positions (longs) up to and with its `$fffe` end.
fn chain_len() -> u32 {
    path_len(PATH_CHAIN, 4, 0xfffe)
}

#[test]
fn the_second_half_of_the_kinds_and_markers_are_registered() {
    let d = sf_core::level::standard_dispatch();
    let m = bonus_level();
    let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
    for (t, init, update) in TYPES_B {
        assert!(d.kind(t as u16).is_some(), "template {t}");
        let tl = template(t);
        assert_eq!(long(tl), h0(init), "template {t}'s init");
        assert_eq!(long(tl + 4), h0(update), "template {t}'s update");
        assert!(d.routine(h0(init)).is_some() && d.routine(h0(update)).is_some());
    }
    assert!(d.routine(h0(0x29418)).is_some());
    for m in (7..=15).chain([55]) {
        assert!(d.marker(6, m).is_some(), "marker {m}");
    }
}

#[test]
fn the_second_half_s_paths_end_where_the_listing_says() {
    assert_eq!(path_len(PATH_124, 2, 0x80), 0x1d);
    assert_eq!(path_len(PATH_131, 2, 0x80), 0x1d);
    // The wave's end is a −1 long: a negative word for 126, 129 and 130, −1 for 133.
    assert_eq!(path_len_by(PATH_WAVE, 4, |w| w & 0x8000 != 0), 0x6d);
    assert_eq!(path_len_by(PATH_WAVE, 8, |w| w & 0x8000 != 0), 0x37);
    assert_eq!(path_len(PATH_WAVE + 2, 4, 0xffff), 0x6d);
    assert!(chain_len() > 2);
}

#[test]
fn the_second_half_s_spawners_match_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(771);
    let mut compared = 0;
    let stubs: [(u8, u32); 10] = [
        (7, 0x18b28),
        (8, 0x18b38),
        (9, 0x18b48),
        (10, 0x18b58),
        (11, 0x18b80),
        (12, 0x18b8a),
        (13, 0x18b9a),
        (14, 0x18baa),
        (15, 0x18bba),
        (55, 0x18abc),
    ];
    for case in 0..N {
        let mut s = Setup::default();
        let (m, entry) = g.pick(&stubs);
        let (other, _) = with_slot(&rig, &mut g, &mut s, template(123));
        let (col, row) = any_cell(&mut g);
        // Now and then a live slot's cell, or (for marker 10's 128) the cell above one.
        let at = match (other != 0, g.below(6)) {
            (true, 0) => rig.state().l(other + 0x40),
            (true, 1) => rig.state().l(other + 0x40).wrapping_sub(0x260),
            _ => cell(col, row),
        };
        s.poke_l(CAMERA_X, g.below(0x1300) as u32);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        s.poke_w(FLY_OFF, pick_or_any(&mut g, &[0, 1]));
        s.poke_l(LEVEL_STATE, g.pick(&[0, 1, 2]) as u32);
        s.a[3] = at;
        let spawn = sf_core::level::standard_dispatch().marker(6, m).expect("registered");
        compared += rig.check(&format!("marker {m}"), case, entry, &s, |c| spawn(c, at)) as usize;
    }
    enough("spawners", compared, N);
}

#[test]
fn the_second_half_s_inits_match_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(772);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, init, _) = g.pick(&TYPES_B);
        let (slot, _) = with_slot(&rig, &mut g, &mut s, template(t));
        if slot == 0 {
            continue;
        }
        cases += 1;
        camera_near(&rig, &mut g, &mut s, slot);
        s.poke_w(FLY_OFF, pick_or_any(&mut g, &[0, 1]));
        s.poke_w(slot + 8, near(&mut g, 0x800, 0x800));
        s.poke_w(slot + 0xa, near(&mut g, 0x200, 0x200));
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch().routine(h0(init)).expect("ported");
        compared += rig.check(&format!("init h0+${init:x}"), case, init, &s, |c| port(c, slot, 0))
            as usize;
    }
    enough("inits", compared, cases);
}

/// The fields a type's update reads, varied.
fn vary_b(g: &mut Gen, s: &mut Setup, slot: u32, bob: u32, t: u32) {
    s.poke_w(slot + 0xc, pick_or_any(g, &[0, 1, 0xffff, 0xfffe, 0xfffa, 8, 0xfff8, 0xc, 0xfff4]));
    s.poke_w(slot + 0xe, pick_or_any(g, &[0, 1, 0xffff, 2, 0xfffe, 0xc, 0xfff4]));
    s.poke_w(slot + 0x5c, near(g, 0x200, 0x200));
    s.poke_w(slot + 0x60, pick_or_any(g, &[1, 2, 0x20, 0x28, 0, 0xffff]));
    s.poke_w(bob + 0x1a, pick_or_any(g, &[0, 3, 0x7530]));
    match t {
        124 | 131 => {
            let path = if t == 124 { PATH_124 } else { PATH_131 };
            let n = path_len(path, 2, 0x80);
            s.poke_l(slot + 0x64, path + 2 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x68, path);
        }
        126 | 129 | 130 => {
            let stride = if t == 126 { 4 } else { 8 };
            let n = path_len_by(PATH_WAVE, stride, |w| w & 0x8000 != 0);
            s.poke_l(slot + 0x64, PATH_WAVE + stride * g.below(n as u64) as u32);
            s.poke_l(slot + 0x68, PATH_WAVE);
        }
        127 | 128 => {
            let n = chain_len();
            s.poke_l(slot + 0x54, PATH_CHAIN + 4 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, PATH_CHAIN);
            s.poke_w(slot + 0x60, near(g, 0x200, 0x200));
            s.poke_w(FLY_OFF, pick_or_any(g, &[0, 0, 1]));
        }
        133 => {
            let n = path_len(PATH_WAVE + 2, 4, 0xffff);
            s.poke_l(slot + 0x54, PATH_WAVE + 4 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, PATH_WAVE);
            s.poke_w(slot + 0x60, near(g, 0x200, 0x200));
        }
        _ => {}
    }
}

#[test]
fn the_second_half_s_updates_match_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(773);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, _, mut update) = g.pick(&TYPES_B);
        // A chain that has been told to fly off.
        if (127..=128).contains(&t) && g.one_in(3) {
            update = 0x29418;
        }
        let (slot, bob) = with_slot(&rig, &mut g, &mut s, template(t));
        if slot == 0 {
            continue;
        }
        cases += 1;
        vary_b(&mut g, &mut s, slot, bob, t);
        camera_near(&rig, &mut g, &mut s, slot);
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch().routine(h0(update)).expect("ported");
        compared += rig.check(&format!("update h0+${update:x}"), case, update, &s, |c| {
            port(c, slot, 0)
        }) as usize;
    }
    enough("updates", compared, cases);
}

#[test]
fn obj_path_xy_matches_the_original() {
    let mut rig = bonus_rig();
    let mut g = Gen::new(774);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (slot, _) = with_slot(&rig, &mut g, &mut s, template(133));
        if slot == 0 {
            continue;
        }
        cases += 1;
        // Along the wave or the chain, or at a −1 long of its own (it starts over).
        let (path, n) = if g.one_in(2) {
            (PATH_WAVE, path_len(PATH_WAVE + 2, 4, 0xffff))
        } else {
            (PATH_CHAIN, chain_len())
        };
        let at = if g.one_in(5) {
            s.poke_l(slot + 0x70, 0xffff_ffff);
            slot + 0x70
        } else {
            path + 4 * g.below(n as u64) as u32
        };
        s.poke_l(slot + 0x54, at);
        s.poke_l(slot + 0x58, path);
        let (dx, dy) = (g.next() as u16, g.next() as u16);
        s.d[0] = dx as u32;
        s.d[1] = dy as u32;
        s.a[0] = slot;
        compared += rig.check("h0+$1c41c", case, 0x1c41c, &s, |c| {
            objects::obj_path_xy(&mut c.bus, slot, dx, dy)
        }) as usize;
    }
    enough("h0+$1c41c", compared, cases);
}
