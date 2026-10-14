//! World 5's object types (`sf_core::enemies::w5`, T39) against the original's, on the routine
//! oracle, and the stubs of world 5's row of `obj_flag_jumptab`, other worlds' types included.
//!
//! Each routine runs on Moira from the original's memory at W5-1's first frame and the port from
//! the same memory transplanted, over generated states; every byte written, the sound driver's
//! state and the Paula writes must match (`sf_conform::routine`). Slots are made by the engine's
//! own spawners, so a generated slot is as consistent as the game makes one.
//!
//! Lives here, not in `sf-core/tests/`, because the purity test keeps the emulator out of the pure
//! crates' tests.

mod common;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_core::State;
use sf_core::enemies::w5::watcher;
use sf_core::frog::{FROG, f};
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
/// The template that fills a table in the generators: world 4's scenery, which W5 uses and which
/// has no hitbox.
const FILLER: u32 = 32;
/// World 5's row of `obj_flag_jumptab`.
const W5: u8 = 4;

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
fn after(rig: &Rig, s: &Setup) -> State {
    let mut st = rig.state().clone();
    for (a, b) in &s.pokes {
        st.poke(*a, b);
    }
    st
}

/// How many of `n` cases the original returned on: a crash or hang leaves a case uncompared
/// (DESIGN §2.11), and the generators must keep that rare.
fn enough(name: &str, compared: usize, n: usize) {
    assert!(n > 0, "{name}: no cases");
    assert!(
        compared * 100 >= n * 99,
        "{name}: only {compared} of {n} cases compared"
    );
}

/// A map cell somewhere on the level (0-indexed column and row, away from its edges).
fn any_cell(g: &mut Gen) -> (u32, u32) {
    (4 + g.below(290) as u32, 4 + g.below(74) as u32)
}

/// `v` moved by up to `span` either way, or now and then anywhere.
fn near(g: &mut Gen, v: u16, span: u16) -> u16 {
    if g.one_in(8) {
        g.next() as u16
    } else {
        v.wrapping_add(g.below(2 * span as u64 + 1) as u16)
            .wrapping_sub(span)
    }
}

/// One of `choices`, or now and then any word.
fn pick_or_any(g: &mut Gen, choices: &[u16]) -> u16 {
    if g.one_in(choices.len() as u64 + 1) {
        g.next() as u16
    } else {
        g.pick(choices)
    }
}

fn word(a: u32) -> u16 {
    let m = w5_level();
    (m.byte(a) as u16) << 8 | m.byte(a + 1) as u16
}

fn long(a: u32) -> u32 {
    (word(a) as u32) << 16 | word(a + 2) as u32
}

/// The steps (8 bytes each) of animation `anim`, up to its negative end mark.
fn steps(anim: u32) -> Vec<u32> {
    (0..64)
        .map(|k| anim + 8 * k)
        .take_while(|&s| (long(s) as i32) >= 0)
        .collect()
}

/// A generated table: some slots freed, now and then the rest filled with backdrops so a spawn
/// finds no room; then `t` spawned at a cell by the engine. The slot and its bob (0, 0 with no
/// room). Without `may_fill`, only the spawn: a second slot beside one already made.
fn with_slot(rig: &Rig, g: &mut Gen, s: &mut Setup, t: u32, may_fill: bool) -> (u32, u32) {
    let (col, row) = any_cell(g);
    with_slot_at(rig, g, s, t, cell(col, row), may_fill)
}

fn with_slot_at(
    rig: &Rig,
    g: &mut Gen,
    s: &mut Setup,
    t: u32,
    at: u32,
    may_fill: bool,
) -> (u32, u32) {
    let free: Vec<u32> = (0..20).filter(|_| may_fill && g.one_in(3)).collect();
    let fill = if may_fill && g.one_in(4) {
        g.below(21)
    } else {
        0
    };
    let fill_at: Vec<(u16, u16)> = (0..fill)
        .map(|_| (g.below(0x1300) as u16, g.below(0x500) as u16))
        .collect();
    let pre = s.pokes.clone();
    let (mut slot, mut bob) = (0, 0);
    let pokes = prepared(rig, &pre, |c| {
        for &k in &free {
            objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
        }
        let r = objects::obj_spawn_cell_alt(c, template(t), at, 0, 0xc);
        slot = r.a0;
        if slot != 0 {
            bob = c.bus.l(slot + 0x50);
        }
        for &(x, y) in &fill_at {
            objects::obj_spawn_at_xy_nodedup(c, template(FILLER), x, y);
        }
    });
    s.pokes = pokes;
    (slot, bob)
}

/// The camera somewhere near `slot`'s position, or anywhere.
fn camera_near(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32) {
    let st = after(rig, s);
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    s.poke_l(CAMERA_X, near(g, x.saturating_sub(0xa0), 0x200) as u32);
    s.poke_l(CAMERA_Y, near(g, y.saturating_sub(0x80), 0x100) as u32);
}

/// Set marker `m` (keeping the tile) at map cell `at` of the setup.
fn poke_marker(rig: &Rig, s: &mut Setup, at: u32, m: u16) {
    let v = after(rig, s).w(at);
    s.poke_w(at, v & 0xffc0 | m);
}

/// A marker that is now and then solid, now and then not.
fn some_marker(g: &mut Gen) -> u16 {
    g.pick(&[0, 0x10, 0x39, 0x3a, 0x3d, 0x3e, 0x3f, 0x25, 2, 3])
}

/// A marker where an update looks, now and then.
fn probe(rig: &Rig, g: &mut Gen, s: &mut Setup, at: u32) {
    let map = sf_core::layout::h1(0x218e0);
    if (map..map + 0xc800).contains(&at) && at % 2 == 0 && g.one_in(2) {
        let m = some_marker(g);
        poke_marker(rig, s, at, m);
    }
}

/// Each type's template, init and update (the updates a slot turns to listed in `MORE`).
const TYPES: [(u32, u32, u32); 13] = [
    (80, 0x26840, 0x26874),
    (79, 0x26806, 0x26874),
    (82, 0x26ae0, 0x26b14),
    (81, 0x26aa6, 0x26b14),
    (84, 0x26bce, 0x26c08),
    (83, 0x26b8e, 0x26c08),
    (85, 0x26cfa, 0x26d14),
    (28, 0x24f06, 0x24f1c),
    (30, 0x25146, 0x2515c),
    (31, 0x2525e, 0x25274),
    (29, 0x2500e, 0x2502a),
    (140, 0x2708c, 0x270a8),
    (32, 0x24eca, 0x24ef6),
];

/// Updates a slot turns to: (template, update).
const MORE: [(u32, u32); 12] = [
    (84, 0x26cb6),
    (83, 0x26cb6),
    (28, 0x24f58),
    (28, 0x24f88),
    (28, 0x24ff8),
    (30, 0x251a8),
    (30, 0x251d8),
    (30, 0x25248),
    (31, 0x25306),
    (29, 0x250d0),
    (140, 0x270d2),
    (140, 0x27100),
];

/// Marker stubs: (marker, entry).
const STUBS: [(u8, u32); 16] = [
    (2, 0x1880a),
    (3, 0x18828),
    (10, 0x188d0),
    (11, 0x188e2),
    (12, 0x188f4),
    (13, 0x18846),
    (14, 0x18864),
    (15, 0x18906),
    (16, 0x18918),
    (17, 0x181a6),
    (18, 0x1830c),
    (19, 0x1831e),
    (20, 0x18882),
    (21, 0x188a0),
    (22, 0x188be),
    (24, 0x18502),
];

/// A pair's partner marker for a stub's marker, and whether it is looked for to the right.
fn partner(m: u8) -> Option<(u16, bool)> {
    match m {
        2 => Some((3, true)),
        3 => Some((2, false)),
        13 => Some((14, true)),
        14 => Some((13, false)),
        20 => Some((21, true)),
        21 => Some((20, false)),
        _ => None,
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
        assert!(d.routine(h0(init)).is_some() && d.routine(h0(update)).is_some());
        let touch = long(tl + 0x10);
        assert!(
            d.routine(touch).is_some(),
            "template {t}'s touch ${:x}",
            touch - h0(0)
        );
    }
    for (_, update) in MORE {
        assert!(d.routine(h0(update)).is_some(), "h0+${update:x}");
    }
    // Every marker of world 5's row that is not the row's do-nothing default has a spawner, and
    // every one this file tests is that row's stub.
    for m in 0..64u32 {
        let at = 0x178ec + 0x400 + 4 * m;
        let target = at + 2 + word(h0(at + 2)) as i16 as i32 as u32;
        if target != 0x17fec {
            assert!(
                d.marker(W5, m as u8).is_some(),
                "marker {m} (h0+${target:x})"
            );
        }
        if let Some(&(_, entry)) = STUBS.iter().find(|s| s.0 as u32 == m) {
            assert_eq!(target, entry, "marker {m}'s stub");
        }
    }
}

#[test]
fn the_watchers_looks_are_sixteen() {
    assert_eq!(long(watcher::LOOKS), h0(0x329a4));
    assert_eq!(long(watcher::LOOKS + 4 * 15), h0(0x329f8));
    // Code follows the table.
    assert_eq!(word(watcher::LOOKS + 4 * 16), 0x2268);
}

#[test]
fn spawners_match_the_original() {
    let mut rig = w5_rig();
    let mut g = Gen::new(3901);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (m, entry) = g.pick(&STUBS);
        // A table as `with_slot` makes one, now and then full.
        let (other, _) = with_slot(&rig, &mut g, &mut s, FILLER, true);
        let (col, row) = any_cell(&mut g);
        let at = if other != 0 && g.one_in(5) {
            after(&rig, &s).l(other + 0x40)
        } else {
            cell(col, row)
        };
        // Pairs: the partner marker somewhere along the row, now and then with a live slot.
        if let Some((pm, right)) = partner(m)
            && !g.one_in(4)
        {
            // Now and then at the search's last cell (`$98`) or just past it.
            let k = if g.one_in(3) {
                g.pick(&[0x97, 0x98, 0x99])
            } else {
                1 + g.below(0xa0) as u32
            };
            let p = if right { at + 2 * k } else { at - 2 * k };
            poke_marker(&rig, &mut s, p, pm);
            if g.one_in(3) {
                with_slot_at(&rig, &mut g, &mut s, FILLER, p, false);
            }
        }
        s.poke_l(CAMERA_X, g.below(0x1300) as u32);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        s.a[3] = at;
        let spawn = sf_core::level::standard_dispatch()
            .marker(W5, m)
            .expect("registered");
        compared += rig.check(&format!("marker {m}"), case, entry, &s, |c| spawn(c, at)) as usize;
    }
    enough("spawners", compared, N);
}

#[test]
fn inits_match_the_original() {
    let mut rig = w5_rig();
    let mut g = Gen::new(3902);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, init, _) = g.pick(&TYPES);
        let (slot, _) = with_slot(&rig, &mut g, &mut s, t, true);
        if slot == 0 {
            continue;
        }
        cases += 1;
        camera_near(&rig, &mut g, &mut s, slot);
        s.poke_w(slot + 8, near(&mut g, 0x800, 0x800));
        s.poke_w(slot + 0xa, near(&mut g, 0x200, 0x200));
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

/// The fields a type's update reads, varied.
fn vary(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32, bob: u32, t: u32, update: u32) {
    s.poke_w(
        slot + 0xc,
        pick_or_any(g, &[0, 3, 0xfffd, 8, 0xfff8, 0xfff6]),
    );
    s.poke_w(
        slot + 0xe,
        pick_or_any(g, &[0, 1, 0xffff, 0xb, 0xc, 0xd, 0xe, 0xf, 0x10, 0xfffd]),
    );
    s.poke_w(slot + 0x5c, pick_or_any(g, &[0, 1, 2, 8, 0xffff, 0x8000]));
    s.poke_w(
        slot + 0x60,
        pick_or_any(g, &[0, 1, 2, 9, 0xa, 0x7d00, 0xffff]),
    );
    s.poke_w(bob + 0x1a, pick_or_any(g, &[0, 1, 3, 4, 5, 0x7d00]));
    s.poke_w(slot + 0x46, near(g, 0x200, 0x200));
    let st = after(rig, s);
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    let (vx, vy) = (st.w(slot + 0xc), st.w(slot + 0xe));
    let v = steps(st.l(bob + 0x42));
    if !v.is_empty() && g.one_in(2) {
        let step = g.pick(&v);
        s.poke_l(bob + 0x16, step);
    }
    // The frog somewhere about it.
    s.poke_w(FROG + f::X, near(g, x, 0x60));
    s.poke_w(FROG + f::Y, near(g, y, 0x60));
    let cell_at = |st: &State, x: u16, y: u16| {
        let row = (y >> 4).wrapping_mul(4);
        st.l(h0(0x17334).wrapping_add(row as i16 as u32))
            .wrapping_add((x >> 4).wrapping_mul(2) as u32)
    };
    match (t, update) {
        // Pacers and shooters: their turning cell, now and then an end marker.
        (79..=84, 0x26874 | 0x26b14 | 0x26c08) => {
            let ends: &[u16] = match t {
                79 | 80 => &[2, 3],
                81 | 82 => &[0xd, 0xe],
                _ => &[0x14, 0x15],
            };
            let at = ((x.wrapping_add(vx).wrapping_sub(0x10) >> 4).wrapping_mul(2) as u32)
                .wrapping_add(st.l(slot + 0x64))
                .wrapping_add(4);
            if g.one_in(2) {
                let m = g.pick(ends);
                poke_marker(rig, s, at, m);
            } else {
                probe(rig, g, s, at);
            }
        }
        (85, _) => {
            for dy in [0, 1] {
                let at = cell_at(
                    &st,
                    x.wrapping_add(vx),
                    y.wrapping_add(vy).wrapping_add(dy).wrapping_sub(8),
                );
                probe(rig, g, s, at);
            }
        }
        (28, 0x24f88) | (30, 0x251d8) => {
            for dy in [0, 1] {
                let at = cell_at(&st, x, y.wrapping_add(vy).wrapping_add(dy)) + 0x260;
                probe(rig, g, s, at);
            }
        }
        (31, 0x25274) => {
            for dy in [0, 1] {
                let at = cell_at(&st, x, y.wrapping_add(vy).wrapping_add(dy));
                probe(rig, g, s, at);
            }
        }
        (30, 0x2515c) => {
            s.poke_l(CAMERA_Y, near(g, y.wrapping_sub(0x20), 0x10) as u32);
        }
        _ => {}
    }
}

#[test]
fn updates_match_the_original() {
    let mut rig = w5_rig();
    let mut g = Gen::new(3903);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, update) = if g.one_in(3) {
            g.pick(&MORE)
        } else {
            let (t, _, u) = g.pick(&TYPES);
            (t, u)
        };
        let (slot, bob) = with_slot(&rig, &mut g, &mut s, t, true);
        if slot == 0 {
            continue;
        }
        cases += 1;
        s.poke_l(slot + 4, h0(update));
        camera_near(&rig, &mut g, &mut s, slot);
        vary(&rig, &mut g, &mut s, slot, bob, t, update);
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(update))
            .expect("ported");
        compared += rig.check(
            &format!("update h0+${update:x} ({t})"),
            case,
            update,
            &s,
            |c| port(c, slot, 0),
        ) as usize;
    }
    enough("updates", compared, cases);
}

/// Touch handlers: (template, handler).
const TOUCHES: [(u32, u32); 5] = [
    (80, 0x23582),
    (84, 0x23582),
    (85, 0x23582),
    (28, 0x23582),
    (140, 0x2360e),
];

#[test]
fn touches_match_the_original() {
    let mut rig = w5_rig();
    let mut g = Gen::new(3904);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N / 3 {
        let mut s = Setup::default();
        let (t, touch) = g.pick(&TOUCHES);
        let (slot, _) = with_slot(&rig, &mut g, &mut s, t, true);
        if slot == 0 {
            continue;
        }
        cases += 1;
        s.poke_w(
            slot + 0x1c,
            pick_or_any(&mut g, &[0x1f, 0x20, 0x40, 0, 0x7d00]),
        );
        camera_near(&rig, &mut g, &mut s, slot);
        let d7 = g.below(2) as u16;
        s.d[7] = d7 as u32;
        s.a[5] = slot;
        s.a[0] = FROG;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(touch))
            .expect("ported");
        compared += rig.check(
            &format!("touch h0+${touch:x} ({t})"),
            case,
            touch,
            &s,
            |c| port(c, slot, d7),
        ) as usize;
    }
    enough("touches", compared, cases);
}
