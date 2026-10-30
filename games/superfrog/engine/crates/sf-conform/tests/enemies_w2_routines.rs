//! World 2's object types (`sf_core::enemies::w2`, T43) and the touch handlers and death updates
//! they share (`sf_core::enemies::common`) against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at W2-1's first frame and the port from
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
use sf_core::enemies::w2::{bouncer, flyer, mace, swooper};
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
/// The template that fills a table in the generators: the backdrop, which has no hitbox.
const FILLER: u32 = 52;
/// World 2's row of `obj_flag_jumptab`.
const W2: u8 = 1;

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
    let m = w2_level();
    (m.byte(a) as u16) << 8 | m.byte(a + 1) as u16
}

fn long(a: u32) -> u32 {
    (word(a) as u32) << 16 | word(a + 2) as u32
}

/// The entries of the table at `start`, `stride` bytes apart, up to and with the first word that
/// `is_end`.
fn table_len(start: u32, stride: u32, is_end: impl Fn(u16) -> bool) -> u32 {
    (0..)
        .find(|&k| is_end(word(start + stride * k)))
        .expect("an end mark")
        + 1
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
const TYPES: [(u32, u32, u32); 17] = [
    (38, 0x258dc, 0x25990),
    (75, 0x266ec, 0x26786),
    (74, 0x266a2, 0x26786),
    (52, 0x25fe8, 0x25ffe),
    (40, 0x25d10, 0x25d30),
    (39, 0x256b6, 0x256ea),
    (24, 0x24e88, 0x24ef6),
    (25, 0x24e9e, 0x24ef6),
    (26, 0x24eb4, 0x24ef6),
    (71, 0x2652c, 0x26692),
    (72, 0x265e0, 0x26692),
    (63, 0x26154, 0x2619c),
    (64, 0x26242, 0x262be),
    (53, 0x25e2a, 0x25fa8),
    (54, 0x25e46, 0x25fa8),
    (37, 0x25786, 0x257e4),
    (149, 0x273f6, 0x2740c),
];

/// Updates a slot turns to: (template, update).
const MORE: [(u32, u32); 6] = [
    (40, 0x25d9a),
    (53, 0x25f42),
    (54, 0x25f42),
    (37, 0x25874),
    (38, 0x23b34),
    (40, 0x23bee),
];

/// Marker stubs: (marker, entry).
const STUBS: [(u8, u32); 16] = [
    (1, 0x1811c),
    (2, 0x1812a),
    (3, 0x18148),
    (10, 0x18166),
    (11, 0x182a0),
    (12, 0x182b2),
    (13, 0x182c4),
    (14, 0x182d6),
    (15, 0x182e8),
    (16, 0x18176),
    (17, 0x181a6),
    (18, 0x1830c),
    (19, 0x1831e),
    (20, 0x18186),
    (21, 0x18196),
    (36, 0x182fa),
];

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
    for (m, _) in STUBS {
        assert!(d.marker(W2, m).is_some(), "marker {m}");
    }
    // Every marker of world 2's row that is not the row's do-nothing default has a spawner.
    for m in 0..64u32 {
        let at = 0x178ec + 0x100 + 4 * m;
        let target = at + 2 + word(h0(at + 2)) as i16 as i32 as u32;
        if target != 0x17fec {
            assert!(
                d.marker(W2, m as u8).is_some(),
                "marker {m} (h0+${target:x})"
            );
        }
    }
}

#[test]
fn the_tables_end_where_the_listing_says() {
    assert_eq!(table_len(bouncer::HOPS, 2, |w| w == 0xffff), 0x2b);
    assert_eq!(table_len(swooper::WAVE, 2, |w| w == 0x80), 0x10);
    // Four loops of `$cc` bytes, then the −1 long.
    let loops = (0..)
        .find(|&k| long(flyer::LOOPS + 4 * k) == 0xffff_ffff)
        .unwrap();
    assert_eq!(loops * 4, 4 * 0xcc);
    assert!(table_len(mace::SWING, 4, |w| w == 0xffff) > 8);
}

#[test]
fn spawners_match_the_original() {
    let mut rig = w2_rig();
    let mut g = Gen::new(4301);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (m, entry) = g.pick(&STUBS);
        // A table as `with_slot` makes one, now and then full (marker 17's links then write
        // their ball to `$64`, the level 1 interrupt's vector, as the original does).
        let (other, _) = with_slot(&rig, &mut g, &mut s, FILLER, true);
        let (col, row) = any_cell(&mut g);
        let at = if other != 0 && g.one_in(5) {
            after(&rig, &s).l(other + 0x40)
        } else {
            cell(col, row)
        };
        // Bouncers: their partner marker somewhere along the row, now and then with a live slot.
        if (m == 2 || m == 3) && !g.one_in(4) {
            // Now and then at the search's last cell (`$98`) or just past it.
            let k = if g.one_in(3) {
                g.pick(&[0x97, 0x98, 0x99])
            } else {
                1 + g.below(0xa0) as u32
            };
            let p = if m == 2 { at + 2 * k } else { at - 2 * k };
            poke_marker(&rig, &mut s, p, 5 - m as u16);
            if g.one_in(3) {
                with_slot_at(&rig, &mut g, &mut s, FILLER, p, false);
            }
        }
        s.poke_l(CAMERA_X, g.below(0x1300) as u32);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        s.poke_w(flyer::NEXT_LOOP, pick_or_any(&mut g, &[0, 1, 2, 3]));
        s.a[3] = at;
        let spawn = sf_core::level::standard_dispatch()
            .marker(W2, m)
            .expect("registered");
        compared += rig.check(&format!("marker {m}"), case, entry, &s, |c| spawn(c, at)) as usize;
    }
    enough("spawners", compared, N);
}

#[test]
fn inits_match_the_original() {
    let mut rig = w2_rig();
    let mut g = Gen::new(4302);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, init, _) = g.pick(&TYPES);
        let (col, row) = any_cell(&mut g);
        let at = cell(col, row);
        // Spikes: a neighbour of the kind two cells left or right, now and then with a slot.
        if (t == 71 || t == 72) && !g.one_in(4) {
            let m = if t == 71 { 0x10 } else { 0x14 };
            let side = if g.one_in(2) { at - 4 } else { at + 4 };
            poke_marker(&rig, &mut s, side, m);
            if g.one_in(3) {
                poke_marker(&rig, &mut s, if side < at { at + 4 } else { at - 4 }, m);
            }
            if !g.one_in(4) {
                let (n, nb) = with_slot_at(&rig, &mut g, &mut s, t, side, false);
                if n != 0 {
                    let anim = after(&rig, &s).l(nb + 0x42);
                    let st = steps(anim);
                    let step = if st.is_empty() { anim } else { g.pick(&st) };
                    if g.one_in(2) {
                        s.poke_l(nb + 0x28, 0);
                        s.poke_l(nb + 0x16, step);
                    } else {
                        s.poke_l(nb + 0x28, step);
                    }
                }
            }
        }
        let (slot, _) = with_slot_at(&rig, &mut g, &mut s, t, at, true);
        if slot == 0 {
            continue;
        }
        cases += 1;
        camera_near(&rig, &mut g, &mut s, slot);
        s.poke_w(flyer::NEXT_LOOP, pick_or_any(&mut g, &[0, 1, 2, 3, 4]));
        s.poke_l(mace::LINK_COUNT, g.pick(&[1, 2, 3, 4, 5, 6]) as u32);
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
        pick_or_any(g, &[0, 1, 0xffff, 2, 0xfffe, 8, 0xfff8]),
    );
    s.poke_w(slot + 0xe, pick_or_any(g, &[0, 1, 0xffff, 7, 8, 9, 0xfff4]));
    s.poke_w(slot + 0x5c, pick_or_any(g, &[0, 1, 2, 3, 0xffff, 0x8000]));
    s.poke_w(slot + 0x60, pick_or_any(g, &[0, 1, 2, 0x40, 0xffff]));
    // `+$64` is a word only to the darts (a pointer to the bouncer, the walker and a link).
    if matches!(t, 53 | 54) {
        s.poke_w(slot + 0x64, pick_or_any(g, &[0, 1, 0x20, 0xffff, 0x60]));
    }
    s.poke_w(bob + 0x1a, pick_or_any(g, &[0, 1, 0x1ff, 0x200, 0x7d00]));
    let st = after(rig, s);
    let anim = st.l(bob + 0x42);
    let any_step = |g: &mut Gen, anim: u32| {
        let v = steps(anim);
        if v.is_empty() { anim } else { g.pick(&v) }
    };
    if g.one_in(2) {
        let step = any_step(g, anim);
        s.poke_l(bob + 0x16, step);
        if g.one_in(3) {
            s.poke_l(bob + 8, step);
        }
    }
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    let cell_at = |st: &State, x: u16, y: u16| {
        let row = (y >> 4).wrapping_mul(4);
        st.l(h0(0x17334).wrapping_add(row as i16 as u32))
            .wrapping_add((x >> 4).wrapping_mul(2) as u32)
    };
    match (t, update) {
        (38, 0x25990) => {
            let n = (4 * 0xcc) / 4;
            s.poke_l(slot + 0x54, flyer::LOOPS + 4 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, flyer::LOOPS);
            s.poke_w(slot + 0x64, near(g, x, 0x40));
            s.poke_w(slot + 0x66, near(g, y, 0x40));
        }
        (74 | 75, _) => {
            let n = table_len(bouncer::HOPS, 2, |w| w == 0xffff);
            s.poke_l(slot + 0x54, bouncer::HOPS + 2 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, bouncer::HOPS);
            s.poke_w(slot + 0x6a, near(g, y, 0x40));
            let vx = st.w(slot + 0xc);
            let at = st
                .l(slot + 0x64)
                .wrapping_add((x.wrapping_add(vx) >> 4).wrapping_mul(2) as u32)
                .wrapping_add(4);
            if g.one_in(2) {
                let m = g.pick(&[2, 3]);
                poke_marker(rig, s, at, m);
            } else {
                probe(rig, g, s, at);
            }
        }
        (40, 0x25d30) => {
            if g.one_in(2) {
                s.poke_l(bob + 0x16, h0(0x3162c));
            }
        }
        (40, 0x25d9a) => {
            let vx = st.w(slot + 0xc);
            let vy = st.w(slot + 0xe);
            probe(
                rig,
                g,
                s,
                cell_at(
                    &st,
                    x.wrapping_add(vx).wrapping_sub(0xa),
                    y.wrapping_add(vy),
                ) + 0x262,
            );
        }
        (39, _) => {
            let vx = pick_or_any(g, &[1, 0xffff]);
            s.poke_w(slot + 0xc, vx);
            if g.one_in(2) {
                let off = if (vx as i16) < 0 { 0xf8 } else { 0xf0 };
                s.poke_l(bob + 0x16, st.l(bob + 8) + off);
            }
            for dx in [0, vx] {
                let at = ((x.wrapping_add(dx).wrapping_add(3) >> 4).wrapping_mul(2) as u32)
                    .wrapping_add(st.l(slot + 0x64))
                    .wrapping_sub(0x25e);
                if g.one_in(2) {
                    poke_marker(rig, s, at, 0x25);
                } else {
                    probe(rig, g, s, at);
                }
            }
        }
        (63, _) => {
            let n = table_len(mace::SWING, 4, |w| w == 0xffff);
            s.poke_l(slot + 0x54, mace::SWING + 4 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, mace::SWING);
            s.poke_w(slot + 0x64, near(g, x, 0x60));
            s.poke_w(slot + 0x66, near(g, y, 0x40));
        }
        (53 | 54, 0x25f42) => {
            let vx = st.w(slot + 0xc);
            probe(
                rig,
                g,
                s,
                cell_at(
                    &st,
                    x.wrapping_add(vx).wrapping_add(vx.wrapping_mul(2)),
                    y.wrapping_sub(8),
                ) + 2,
            );
        }
        (37, _) => {
            let n = table_len(swooper::WAVE, 2, |w| w == 0x80);
            s.poke_l(slot + 0x64, swooper::WAVE + 2 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x68, swooper::WAVE);
            s.poke_w(slot + 0x5c, near(g, y, 0x40));
            s.poke_w(slot + 0x6c, pick_or_any(g, &[0, 1, 0x40]));
            s.poke_w(slot + 0x70, pick_or_any(g, &[0, 1, 0x40]));
        }
        (149, _) => {
            if g.one_in(3) {
                let broken = steps(anim).into_iter().find(|&st| long(st) == h0(0x2ebac));
                if let Some(b) = broken {
                    s.poke_l(bob + 0x16, b);
                }
            }
        }
        (38, 0x23b34) => {
            // A falling enemy's speed is −1..8 (a wild one reads the map's row pointers off
            // their table, into code the engine keeps as the executable has it).
            let vy = (g.below(10) as u16).wrapping_sub(1);
            s.poke_w(slot + 0xe, vy);
            probe(
                rig,
                g,
                s,
                cell_at(&st, x.wrapping_add(8), y.wrapping_add(vy).wrapping_add(8)) + 2,
            );
            if g.one_in(2) {
                s.poke_l(bob + 8, h0(0x2cef4));
                let v = steps(h0(0x2cef4));
                if !v.is_empty() {
                    let step = g.pick(&v);
                    s.poke_l(bob + 0x16, step);
                }
            }
        }
        _ => {}
    }
}

#[test]
fn updates_match_the_original() {
    let mut rig = w2_rig();
    let mut g = Gen::new(4303);
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
        // Chain links: their ball, now and then gone or not a ball.
        if t == 64 {
            let (ball, bb) = with_slot(&rig, &mut g, &mut s, 63, false);
            let to = match g.below(6) {
                0 => 0,
                1 => slot,
                _ => ball,
            };
            s.poke_l(slot + 0x64, to);
            s.poke_l(slot + 0x68, g.pick(&[1, 2, 3, 4, 5, 0x1234, 0xffff]) as u32);
            if ball != 0 {
                let st = after(&rig, &s);
                let (bx, by) = (st.w(ball + 8), st.w(ball + 0xa));
                s.poke_w(ball + 0x6c, near(&mut g, bx, 0x80));
                s.poke_w(ball + 0x6e, near(&mut g, by, 0x80));
                s.poke_w(ball + 0x70, near(&mut g, bx, 0x80));
                s.poke_w(ball + 0x72, near(&mut g, by, 0x80));
                if g.one_in(4) {
                    s.poke_l(bb + 8, h0(0x31560));
                    s.poke_l(bb + 0x42, h0(0x31560));
                }
            }
        }
        // The ball: some links about.
        if t == 63 {
            for _ in 0..g.below(7) {
                let (_, lb) = with_slot(&rig, &mut g, &mut s, 64, false);
                if lb != 0 && g.one_in(4) {
                    s.poke_l(lb + 8, 0);
                }
            }
        }
        cases += 1;
        s.poke_l(slot + 4, h0(update));
        vary(&rig, &mut g, &mut s, slot, bob, t, update);
        camera_near(&rig, &mut g, &mut s, slot);
        s.poke_l(sf_core::items::SCORE, g.below(100_000) as u32);
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
const TOUCHES: [(u32, u32); 9] = [
    (38, 0x23582),
    (75, 0x23582),
    (40, 0x2360e),
    (39, 0x2360e),
    (40, 0x23662),
    (63, 0x236ac),
    (71, 0x237c0),
    (72, 0x237c0),
    (149, 0x23774),
];

#[test]
fn touches_match_the_original() {
    let mut rig = w2_rig();
    let mut g = Gen::new(4304);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, touch) = g.pick(&TOUCHES);
        let (slot, bob) = with_slot(&rig, &mut g, &mut s, t, true);
        if slot == 0 {
            continue;
        }
        cases += 1;
        s.poke_w(
            slot + 0x1c,
            pick_or_any(&mut g, &[0x1f, 0x20, 0x3f, 0x40, 0, 0x7d00]),
        );
        s.poke_w(slot + 0x5c, pick_or_any(&mut g, &[0, 1, 0xffff]));
        s.poke_w(bob + 0x1a, pick_or_any(&mut g, &[0, 0x1ff, 0x200, 0x7d00]));
        let anim = after(&rig, &s).l(bob + 0x42);
        let v = steps(anim);
        if !v.is_empty() && g.one_in(2) {
            let step = g.pick(&v);
            s.poke_l(bob + 0x16, step);
        }
        s.poke_w(
            FROG + f::VY,
            pick_or_any(&mut g, &[0, 1, 7, 8, 9, 0xffff, 0xfffc]),
        );
        s.poke_w(FROG + f::ON_GROUND, pick_or_any(&mut g, &[0, 1]));
        for o in [f::BLINK, f::SHIELD, f::HURT, f::DYING] {
            if g.one_in(6) {
                s.poke_w(FROG + o, 1);
            }
        }
        s.poke_w(FROG + 0x5a, pick_or_any(&mut g, &[0, 1, 2, 3]));
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
