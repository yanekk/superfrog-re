//! World 4's enemies (`sf_core::enemies::w4`, T41) and the touch handlers they use
//! (`sf_core::enemies::common`) against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at W4-1's first frame and the port from
//! the same memory transplanted, over generated states. Every byte written, the sound driver's
//! state and the Paula writes must match (`sf_conform::routine`). Slots are made by the engine's
//! own spawners, so a generated slot is as consistent as the game makes one; the map around it is
//! given the markers its routines look for.

mod common;

use std::sync::OnceLock;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_conform::{SyncPoint, sync};
use sf_core::enemies::w4::{W4, gate, mace, pacer, swooper};
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
const MAP_ROWS: u32 = h0(0x17334);

/// The original's memory at W4-1's first frame.
fn w4_level() -> &'static AmigaMemory {
    static M: OnceLock<AmigaMemory> = OnceLock::new();
    M.get_or_init(|| {
        sync::reach(
            &SyncPoint::AtLevel("W4-1".parse().unwrap()),
            &sf_conform::InputScript::idle(),
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"))
        .memory
    })
}

fn w4_rig() -> Rig<'static> {
    static D: OnceLock<std::sync::Arc<sf_core::DisplayData>> = OnceLock::new();
    let d = D.get_or_init(sf_conform::display_data);
    Rig::new(w4_level(), sound(), d)
}

fn word(a: u32) -> u16 {
    let m = w4_level();
    (m.byte(a) as u16) << 8 | m.byte(a + 1) as u16
}

fn long(a: u32) -> u32 {
    (word(a) as u32) << 16 | word(a + 2) as u32
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
    (4 + g.below(286) as u32, 4 + g.below(66) as u32)
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
const TYPES: [(u32, u32, u32); 31] = [
    (27, 0x24ee0, 0x24ef6),
    (32, 0x24eca, 0x24ef6),
    (59, 0x25ed2, 0x25fa8),
    (60, 0x25eee, 0x25fa8),
    (61, 0x25f0a, 0x25fa8),
    (62, 0x25f26, 0x25fa8),
    (69, 0x2610c, 0x2619c),
    (70, 0x2629c, 0x262be),
    (93, 0x2795e, 0x27992),
    (135, 0x25936, 0x25990),
    (136, 0x26d7e, 0x26e0c),
    (137, 0x26dc8, 0x26e0c),
    (138, 0x26e8c, 0x26eee),
    (139, 0x26ec0, 0x26eee),
    (141, 0x26f64, 0x26f80),
    (142, 0x26f64, 0x27006),
    (143, 0x27704, 0x27720),
    (144, 0x27704, 0x27720),
    (145, 0x2712e, 0x27144),
    (146, 0x27166, 0x2717c),
    (147, 0x27276, 0x2728c),
    (148, 0x27386, 0x2739c),
    (151, 0x274d6, 0x274f0),
    (152, 0x275fa, 0x276aa),
    (153, 0x27610, 0x276aa),
    (154, 0x27626, 0x276aa),
    (155, 0x2763c, 0x276aa),
    (156, 0x27652, 0x276aa),
    (157, 0x27668, 0x276aa),
    (158, 0x2767e, 0x276aa),
    (159, 0x27694, 0x276aa),
];

/// Marker stubs: (marker, stub).
const STUBS: [(u8, u32); 22] = [
    (1, 0x1856c),
    (2, 0x1857a),
    (3, 0x18598),
    (10, 0x185b6),
    (11, 0x185c8),
    (12, 0x185da),
    (13, 0x186e4),
    (14, 0x18702),
    (15, 0x18720),
    (16, 0x18732),
    (17, 0x185ea),
    (18, 0x18744),
    (19, 0x18756),
    (20, 0x18768),
    (21, 0x1877a),
    (22, 0x1878c),
    (24, 0x1879e),
    (25, 0x187b0),
    (26, 0x187c2),
    (27, 0x187d4),
    (32, 0x187e6),
    (33, 0x187f8),
];

/// The markers W4's routines look for in the map: pairs' ends, turning and stopping markers,
/// solid ground, nothing.
const MARKERS: [u16; 9] = [2, 3, 0xd, 0xe, 0x17, 0x25, 0x3a, 0x3e, 0];

/// Map cell `col`, `row` given marker `m`, its tile kept.
fn set_marker(rig: &Rig, s: &mut Setup, col: u32, row: u32, m: u16) {
    let a = cell(col, row);
    let st = poked(rig, s);
    s.poke_w(a, (st.w(a) & 0xffc0) | m);
}

/// Now and then markers from [`MARKERS`] in the cells around pixel `x`, `y`.
fn plant(rig: &Rig, g: &mut Gen, s: &mut Setup, x: u16, y: u16) {
    if g.one_in(3) {
        return;
    }
    let (col, row) = ((x >> 4) as i32, (y >> 4) as i32);
    for _ in 0..1 + g.below(6) {
        let c = col + g.below(7) as i32 - 3;
        let r = row + g.below(5) as i32 - 2;
        if (0..300).contains(&c) && (0..84).contains(&r) {
            let m = g.pick(&MARKERS);
            set_marker(rig, s, c as u32, r as u32, m);
        }
    }
}

/// A generated table: some slots freed, now and then the rest filled with drops (152) so a spawn
/// finds no room (never under marker 17's: see `spawn_69`); then `t` spawned at a cell by the
/// engine (marker 17's stub for 69 and 70, the link picked for 70). The slot and its bob (0, 0
/// with no room).
fn with_slot(rig: &Rig, g: &mut Gen, s: &mut Setup, t: u32) -> (u32, u32) {
    let (col, row) = any_cell(g);
    let free: Vec<u32> = (0..20).filter(|_| g.one_in(3)).collect();
    let fill = if t != 69 && t != 70 && g.one_in(4) {
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
        if t == 69 || t == 70 {
            for k in 0..20 {
                objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
            }
            let at = cell(col, row.min(60));
            mace::spawn_69(c, at);
            let want = if t == 69 { at } else { at + link * 0x260 };
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
            objects::obj_spawn_at_xy_nodedup(c, template(152), x, y);
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
    s.poke_w(FROG + 6, pick_or_any(g, &[0, 1, 7, 8, 9, 0xffff, 0xfff8]));
    s.poke_w(FROG + 0x24, pick_or_any(g, &[0, 1]));
    s.poke_w(FROG + 0x3a, pick_or_any(g, &[0, 0, 1]));
}

/// The frame records W4's routines compare against: the gates' tables', the ledge's open one, the
/// fountain's throwing one, the killed look's last.
fn frames() -> Vec<u32> {
    let mut v = vec![h0(0x2dfdc), h0(0x2e36c), h0(0x2af0a), h0(0x2aefe)];
    for table in [gate::TABLE_A, gate::TABLE_B] {
        let mut at = table;
        while long(at) & 0x8000_0000 == 0 {
            v.push(long(at));
            at += 0x10;
        }
    }
    v
}

/// An animation step for `bob` to show: its `+$16` pointed at `slot + $6a`, so the step's `+$06`
/// word is the slot's `+$70`, and its frame (the long it starts with) the slot's `+$6a`.
/// Half the time the frame is one `t` itself looks for.
fn step_word(g: &mut Gen, s: &mut Setup, slot: u32, bob: u32, t: u32) {
    s.poke_l(bob + 0x16, slot + 0x6a);
    s.poke_w(slot + 0x70, pick_or_any(g, &[0, 1, 2]));
    let all = frames();
    let own: &[u32] = match t {
        146 => &all[4..12],
        147 => &all[12..20],
        148 => &all[0..1],
        151 => &all[1..2],
        _ => &all,
    };
    let f = if g.one_in(2) {
        g.pick(own)
    } else {
        g.pick(&all)
    };
    s.poke_l(slot + 0x6a, f);
}

/// Half the time, the marker `t`'s update looks for in the cells it looks at (around them, as
/// it moves first).
fn aim(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32, t: u32) {
    if g.one_in(2) {
        return;
    }
    let st = poked(rig, s);
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    let base = st.l(slot + 0x64);
    let (cells, m): (Vec<u32>, u16) = match t {
        93 => (
            (0..4)
                .map(|k| {
                    base.wrapping_add(2 * ((x.wrapping_add(3) >> 4) as u32 + k))
                        .wrapping_sub(0x260)
                })
                .collect(),
            0x25,
        ),
        136..=139 => (
            (0..4)
                .map(|k| base.wrapping_add(2 * ((x >> 4) as u32 + k)).wrapping_sub(2))
                .collect(),
            if t <= 137 {
                g.pick(&[2, 3])
            } else {
                g.pick(&[0xd, 0xe])
            },
        ),
        143 | 144 => {
            let row = st.l(MAP_ROWS.wrapping_add(((y >> 4) as u32) * 4));
            (
                vec![row.wrapping_add(2 * ((x.wrapping_add(0x11) >> 4) as u32))],
                0x17,
            )
        }
        _ => return,
    };
    let lo = cell(0, 0);
    let hi = cell(0, 84);
    for a in cells {
        if (lo..hi).contains(&a) && !g.one_in(4) {
            s.poke_w(a, (st.w(a) & 0xffc0) | m);
        }
    }
}

#[test]
fn the_kinds_and_markers_are_registered() {
    let d = sf_core::level::standard_dispatch();
    for (t, init, update) in TYPES {
        assert!(d.kind(t as u16).is_some(), "template {t}");
        let tl = template(t);
        assert_eq!(long(tl), h0(init), "template {t}'s init");
        assert_eq!(long(tl + 4), h0(update), "template {t}'s update");
        assert!(d.routine(long(tl + 0x10)).is_some(), "template {t}'s touch");
        assert!(d.routine(h0(init)).is_some() && d.routine(h0(update)).is_some());
    }
    for a in [
        0x25f42, 0x26faa, 0x26fd8, 0x27030, 0x2705e, 0x27792, 0x23b34, 0x23bee, 0x23662,
    ] {
        assert!(d.routine(h0(a)).is_some(), "h0+${a:x}");
    }
    // Every W4 marker the jump table sends anywhere but the empty stub.
    let jt = h0(0x178ec) + 0x300;
    for k in 0..64u32 {
        let to = jt + 4 * k + 2 + word(jt + 4 * k + 2) as i16 as i32 as u32;
        if to != h0(0x17fec) {
            assert!(d.marker(W4, k as u8).is_some(), "marker {k}");
        }
    }
    assert_eq!(
        STUBS.len() + 4,
        (0..64u32)
            .filter(|k| {
                let to = jt + 4 * k + 2 + word(jt + 4 * k + 2) as i16 as i32 as u32;
                to != h0(0x17fec)
            })
            .count(),
        "every stub but the coins', the lock's and the spring's is tried"
    );
}

#[test]
fn the_paths_end_where_the_listing_says() {
    // The swooper's loop: four quarters of `$cc` bytes, then a −1 long.
    assert_eq!(long(swooper::LOOP + 4 * 0xcc), 0xffff_ffff);
    assert!((0..4 * 0xcc / 4).all(|k| long(swooper::LOOP + 4 * k) != 0xffff_ffff));
    // The pacer's wave: 42 words, then `$ffff`.
    assert_eq!(word(pacer::WAVE + 2 * 42), 0xffff);
    assert!((0..42).all(|k| word(pacer::WAVE + 2 * k) != 0xffff));
    assert!(frames().len() == 4 + 16);
}

#[test]
fn spawners_match_the_original() {
    let mut rig = w4_rig();
    let mut g = Gen::new(4101);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (m, entry) = g.pick(&STUBS);
        let (other, _) = with_slot(&rig, &mut g, &mut s, if m == 17 { 93 } else { 152 });
        let (col, row) = any_cell(&mut g);
        let row = row.min(60);
        let at = if other != 0 && g.one_in(5) {
            rig.state().l(other + 0x40)
        } else {
            cell(col, row)
        };
        // A pair's partner on the row, now and then live.
        if matches!(m, 2 | 3 | 13 | 14) && at == cell(col, row) && !g.one_in(4) {
            let partner = match m {
                2 => 3,
                3 => 2,
                13 => 0xe,
                _ => 0xd,
            };
            let right = matches!(m, 2 | 13);
            let span = if g.one_in(8) { 0xa0 } else { 0x20 };
            let reach = 1 + g.below(span) as u32;
            let pc = if right {
                col + reach
            } else {
                col.wrapping_sub(reach)
            };
            if pc < 304 {
                set_marker(&rig, &mut s, pc, row, partner);
                if other != 0 && g.one_in(3) {
                    s.poke_l(other + 0x40, cell(pc, row));
                }
            }
        }
        // Marker 17 needs room for its ball and links (see `spawn_69`).
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
        s.poke_w(swooper::SWOOPER_PHASE, pick_or_any(&mut g, &[0, 1, 2, 3]));
        s.a[3] = at;
        let spawn = sf_core::level::standard_dispatch()
            .marker(W4, m)
            .expect("registered");
        compared += rig.check(&format!("marker {m}"), case, entry, &s, |c| spawn(c, at)) as usize;
    }
    enough("spawners", compared, cases);
}

#[test]
fn inits_match_the_original() {
    let mut rig = w4_rig();
    let mut g = Gen::new(4102);
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
        s.poke_w(
            swooper::SWOOPER_PHASE,
            pick_or_any(&mut g, &[0, 1, 2, 3, 4]),
        );
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
fn vary(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32, bob: u32, t: u32) {
    s.poke_w(
        slot + 0xc,
        pick_or_any(g, &[0, 1, 0xffff, 3, 0xfffd, 8, 0xfff8, 2, 0xfffe]),
    );
    s.poke_w(slot + 0xe, pick_or_any(g, &[0, 2, 8, 0xfff8, 0xffff]));
    s.poke_w(slot + 0x5c, pick_or_any(g, &[0, 1, 2, 3, 0xffff, 0x14]));
    s.poke_w(slot + 0x60, pick_or_any(g, &[0, 1, 2, 7, 8, 9]));
    if (59..=62).contains(&t) {
        s.poke_w(slot + 0x64, pick_or_any(g, &[0, 1, 2, 0x20]));
    }
    s.poke_w(
        bob + 0x1a,
        pick_or_any(g, &[0, 1, 2, 0x1ff, 0x200, 0x201, 0x7530, 0x8000]),
    );
    // The step `step_word` plants lies over `+$6a`..`+$73`, which the pacer and the mace keep.
    if !matches!(t, 69 | 136 | 137) && g.one_in(2) {
        step_word(g, s, slot, bob, t);
    }
    match t {
        69 => {
            let n = 0x194 / 4;
            s.poke_l(slot + 0x54, mace::SWING + 4 * g.below(n) as u32);
            if g.one_in(4) {
                for k in 0..20 {
                    if g.one_in(4) {
                        s.poke_l(OBJ_SLOTS + k * SLOT + 0x40, 0);
                    }
                }
            }
        }
        70 => {
            if g.one_in(8) {
                s.poke_l(slot + 0x68, g.next() as u32);
            }
        }
        135 => {
            s.poke_l(
                slot + 0x54,
                swooper::LOOP + 4 * g.below(4 * 0xcc / 4 + 1) as u32,
            );
            s.poke_l(slot + 0x58, swooper::LOOP);
            s.poke_w(slot + 0x64, near(g, 0x800, 0x800));
            s.poke_w(slot + 0x66, near(g, 0x200, 0x200));
        }
        136 | 137 => {
            s.poke_l(slot + 0x54, pacer::WAVE + 2 * g.below(43) as u32);
            s.poke_l(slot + 0x58, pacer::WAVE);
            s.poke_w(slot + 0x6a, near(g, 0x200, 0x200));
        }
        _ => {}
    }
    let st = poked(rig, s);
    plant(rig, g, s, st.w(slot + 8), st.w(slot + 0xa));
    aim(rig, g, s, slot, t);
}

/// Each type's other states' updates, by template.
fn other_states(t: u32) -> &'static [u32] {
    match t {
        59..=62 => &[0x25f42],
        141 => &[0x26faa, 0x26fd8],
        142 => &[0x27030, 0x2705e],
        143 | 144 => &[0x27792, 0x23bee],
        93 | 138 | 139 | 145 | 152..=159 => &[0x23bee],
        135..=137 => &[0x23b34],
        _ => &[],
    }
}

#[test]
fn updates_match_the_original() {
    let mut rig = w4_rig();
    let mut g = Gen::new(4103);
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
        vary(&rig, &mut g, &mut s, slot, bob, t);
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
    let mut rig = w4_rig();
    let mut g = Gen::new(4104);
    let mut compared = 0;
    let mut cases = 0;
    let touches = [0x23582, 0x2360e, 0x23662, 0x236ac, 0x23774, 0x23774];
    for case in 0..N {
        let mut s = Setup::default();
        let (t, _, _) = g.pick(&TYPES);
        let touch = g.pick(&touches);
        let (slot, bob) = with_slot(&rig, &mut g, &mut s, t);
        if slot == 0 {
            continue;
        }
        cases += 1;
        vary(&rig, &mut g, &mut s, slot, bob, t);
        camera_and_frog_near(&rig, &mut g, &mut s, slot);
        s.poke_w(
            slot + 0x1c,
            pick_or_any(&mut g, &[0x18, 0x1f, 0x20, 0x21, 0x3f, 0x7d00, 0]),
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
fn the_counters_are_layout_variables() {
    for a in [mace::CHAIN_LINK_COUNT, swooper::SWOOPER_PHASE] {
        assert!(sf_core::layout::index_of(a).is_some(), "${a:x}");
    }
}
