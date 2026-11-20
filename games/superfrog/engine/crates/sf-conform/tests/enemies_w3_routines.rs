//! World 3's enemies (`sf_core::enemies::w3`, T44) and the touch handlers they add to
//! `sf_core::enemies::common`, against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at W3-1's first frame and the port from
//! the same memory transplanted, over generated states: every byte written, the sound driver's
//! state and the Paula writes must match (`sf_conform::routine`). Slots are made by the engine's
//! own spawners, so a generated slot is as consistent as the game makes one; the map cells around
//! it are sprinkled with the markers the routines look for (`$25` turns, solid cells, the
//! flows' and pads' neighbours).
//!
//! Lives here, not in `sf-core/tests/` as T44's doc says, because the purity test keeps the
//! emulator out of the pure crates' tests (as T42's).

mod common;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_core::enemies::w1::hover;
use sf_core::enemies::w3::{hopper, scenery, swing};
use sf_core::frog::{FROG, f};
use sf_core::items::template;
use sf_core::layout::{LAYOUT, MAP_ROW_CELLS, Ram, VarId, h0};
use sf_core::level::objects::{self, OBJ_SLOTS};
use sf_core::level::{Hw, LevelCtx};
use sf_core::mem::Bus;

const N: usize = 10_000;

const ATTRACT_MODE: u32 = h0(0x18);
const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);
const SCROLL_DIRS: u32 = h0(0x16682);
const SLOT: u32 = 0x7c;
const ANIM_END: u32 = h0(0x2af0a);
const RNG_STATE: u32 = h0(0x160d4);
const SPLAT: u32 = h0(0x2cef4);
/// The crumble's last step (h0+$2e8ac).
const GIVEN_WAY: u32 = h0(0x2e8ac);

/// Each type's template, init and update.
const TYPES: [(u32, u32, u32); 18] = [
    (34, 0x249d2, 0x249e8),
    (55, 0x25e62, 0x25fa8),
    (56, 0x25e7e, 0x25fa8),
    (65, 0x2607c, 0x2619c),
    (66, 0x26260, 0x262be),
    (87, 0x2787e, 0x27894),
    (88, 0x25c8a, 0x25caa),
    (89, 0x27868, 0x27894),
    (90, 0x27852, 0x27894),
    (91, 0x27c90, 0x27cc0),
    (92, 0x278a4, 0x278d8),
    (94, 0x285a2, 0x2861e),
    (95, 0x27a18, 0x27a5e),
    (96, 0x27d1c, 0x27d4c),
    (97, 0x27d1c, 0x27da8),
    (98, 0x2872e, 0x287a8),
    (99, 0x287b8, 0x287e0),
    (150, 0x27466, 0x2747c),
];

/// The updates a slot turns to, with a type that can reach each: the dart flying, the hopper's
/// leap, the jumper's leap, circling and drop, a killed one's fall, a stomped one's.
const LATER: [(u32, u32); 7] = [
    (0x25f42, 55),
    (0x286ee, 94),
    (0x27b2a, 95),
    (0x27bb4, 95),
    (0x27be0, 95),
    (0x23b34, 95),
    (0x23bee, 91),
];

/// Each marker's stub.
const STUBS: [(u8, u32); 17] = [
    (5, 0x18344),
    (13, 0x184e2),
    (14, 0x1837c),
    (15, 0x18392),
    (16, 0x1836a),
    (17, 0x18526),
    (18, 0x18548),
    (19, 0x1855a),
    (20, 0x184a2),
    (21, 0x184b2),
    (23, 0x18516),
    (24, 0x18502),
    (25, 0x184d2),
    (26, 0x184c2),
    (27, 0x183a8),
    (28, 0x184f2),
    (36, 0x18536),
];

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
        if a == b {
            continue;
        }
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
    assert!(
        compared * 100 >= n * 99,
        "{name}: only {compared} of {n} cases compared"
    );
}

/// A map cell somewhere on the level (0-indexed column and row, away from its edges and with
/// room below for the swing's links).
fn any_cell(g: &mut Gen) -> (u32, u32) {
    (4 + g.below(286) as u32, 4 + g.below(72) as u32)
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

/// The rig's state with `s`'s pokes in.
fn with_pokes(rig: &Rig, s: &Setup) -> sf_core::State {
    let mut st = rig.state().clone();
    for (a, b) in &s.pokes {
        st.poke(*a, b);
    }
    st
}

fn rom_long(a: u32) -> u32 {
    let m = w3_level();
    (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32)
}

fn rom_word(a: u32) -> u16 {
    let m = w3_level();
    (m.byte(a) as u16) << 8 | m.byte(a + 1) as u16
}

/// A generated table: some slots freed, now and then the rest filled with scenery so a spawn
/// finds no room; then `t` spawned at a cell by the engine, `dy` into it. The slot and its bob
/// (0, 0 with no room, or when the filling evicted it), and the cell.
fn with_slot(rig: &Rig, g: &mut Gen, s: &mut Setup, t: u32, dy: u16) -> (u32, u32, (u32, u32)) {
    let (col, row) = any_cell(g);
    let free: Vec<u32> = (0..20).filter(|_| g.one_in(3)).collect();
    let fill = if g.one_in(5) { g.below(21) } else { 0 };
    let fill_at: Vec<(u16, u16)> = (0..fill)
        .map(|_| (g.below(0x1200) as u16, g.below(0x500) as u16))
        .collect();
    let index = pick_or_any(g, &[1, 2, 3, 4, 5]) as u32;
    let pre = s.pokes.clone();
    let (mut slot, mut bob) = (0, 0);
    let pokes = prepared(rig, &pre, |c| {
        c.bus.set_l(swing::LINK_INDEX, index);
        for &k in &free {
            objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
        }
        let r = objects::obj_spawn_cell_alt(c, template(t), cell(col, row), 0, dy);
        slot = r.a0;
        if slot != 0 {
            bob = c.bus.l(slot + 0x50);
        }
        for &(x, y) in &fill_at {
            objects::obj_spawn_at_xy_nodedup(c, template(34), x, y);
        }
        // A full table evicts an evictable slot off the screen: maybe this one.
        if slot != 0 && c.bus.l(slot + 0x40) != cell(col, row) {
            slot = 0;
        }
    });
    s.pokes = pokes;
    (slot, bob, (col, row))
}

/// Markers the routines look for, sprinkled over the cells around (`col`, `row`), each cell's
/// tile kept.
fn sprinkle(rig: &Rig, g: &mut Gen, s: &mut Setup, (col, row): (u32, u32)) {
    let st = with_pokes(rig, s);
    for dr in 0..8 {
        for dc in 0..9 {
            if !g.one_in(4) {
                continue;
            }
            let (c, r) = ((col + dc).saturating_sub(4), (row + dr).saturating_sub(3));
            let at = cell(c.min(MAP_ROW_CELLS - 1), r);
            let m = pick_or_any(
                g,
                &[0, 0x25, 0x3a, 0x39, 0x3c, 0x3d, 0x3e, 0x3f, 5, 0x18, 9],
            ) & 0x3f;
            s.poke_w(at, st.w(at) & 0xffc0 | m);
        }
    }
}

/// The camera somewhere near `slot`'s position, or anywhere, and the way it last scrolled.
fn camera_near(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32) {
    let st = with_pokes(rig, s);
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    s.poke_l(CAMERA_X, near(g, x.saturating_sub(0xa0), 0x200) as u32);
    s.poke_l(CAMERA_Y, near(g, y.saturating_sub(0x80), 0x100) as u32);
    s.poke_b(SCROLL_DIRS, g.next() as u8);
}

/// Any RNG state.
fn any_rng(g: &mut Gen, s: &mut Setup) {
    s.poke_l(RNG_STATE, g.next() as u32);
    s.poke_l(RNG_STATE + 4, g.next() as u32);
}

/// The frog somewhere near `slot`, or anywhere, falling or rising, on the ground or not.
fn frog_near(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32) {
    let st = with_pokes(rig, s);
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    s.poke_w(FROG + f::X, near(g, x, 0x80));
    s.poke_w(FROG + f::Y, near(g, y, 0x60));
    s.poke_w(
        FROG + f::VY,
        pick_or_any(g, &[0, 1, 7, 8, 9, 0xffff, 0xfffc, 0x8000]),
    );
    s.poke_w(FROG + f::ON_GROUND, pick_or_any(g, &[0, 1]));
}

/// The longs of the path at `start` up to and with the first that `is_end`.
fn longs_until(start: u32, is_end: impl Fn(u32) -> bool) -> u32 {
    (0..)
        .find(|&k| is_end(rom_long(start + 4 * k)))
        .expect("an end mark")
        + 1
}

/// The words of the path at `start` up to and with `end`.
fn words_until(start: u32, end: u16) -> u32 {
    (0..)
        .find(|&k| rom_word(start + 2 * k) == end)
        .expect("an end mark")
        + 1
}

/// A bob's current step made to point at a frame record the slot's own spare bytes hold
/// (`+$74`), which is `frame`.
fn fake_step(s: &mut Setup, slot: u32, bob: u32, frame: u32) {
    s.poke_l(slot + 0x74, frame);
    s.poke_l(bob + 0x16, slot + 0x74);
}

/// Some slots' bobs made to show (or start) one of the swing links' or balls' animations, so a
/// ball counts its links and a link finds its ball (or not).
fn some_swing_bobs(rig: &Rig, g: &mut Gen, s: &mut Setup) {
    let anims = [
        0x31560, 0x31c54, 0x32bb8, 0x3227c, 0x32288, 0x3156c, 0x31c60, 0x32bc4,
    ];
    let st = with_pokes(rig, s);
    let n = g.below(9);
    for _ in 0..n {
        let k = g.below(20) as u32;
        let bob = st.l(OBJ_SLOTS + k * SLOT + 0x50);
        if bob == 0 {
            continue;
        }
        let at = if g.one_in(2) { 8 } else { 0x42 };
        s.poke_l(bob + at, h0(g.pick(&anims)));
    }
}

#[test]
fn the_kinds_and_markers_are_registered() {
    let d = sf_core::level::standard_dispatch();
    for (t, init, update) in TYPES {
        assert!(d.kind(t as u16).is_some(), "template {t}");
        assert_eq!(rom_long(template(t)), h0(init), "template {t}'s init");
        assert_eq!(
            rom_long(template(t) + 4),
            h0(update),
            "template {t}'s update"
        );
        assert!(d.routine(h0(init)).is_some() && d.routine(h0(update)).is_some());
        let touch = rom_long(template(t) + 0x10);
        assert!(
            d.routine(touch).is_some(),
            "template {t}'s touch ${touch:x}"
        );
    }
    for (u, _) in LATER {
        assert!(d.routine(h0(u)).is_some(), "h0+${u:x}");
    }
    for (marker, entry) in STUBS {
        assert!(d.marker(2, marker).is_some(), "marker {marker}");
        // The stub is the one world 3's row of `obj_flag_jumptab` branches to.
        let at = 0x178ec + 0x200 + 4 * marker as u32;
        let target = at + 2 + rom_word(h0(at + 2)) as i16 as i32 as u32;
        assert_eq!(target, entry, "marker {marker}'s stub");
    }
}

#[test]
fn the_paths_end_where_the_listing_says() {
    assert_eq!(longs_until(swing::PATH, |l| l == 0xffff_ffff), 0x65);
    assert_eq!(words_until(hopper::PATH, 0x80), 0x17);
    assert_eq!(scenery::FLOW_END - scenery::FLOW_ANIM, 0x44);
}

#[test]
fn spawners_match_the_original() {
    let mut rig = w3_rig();
    let mut g = Gen::new(4401);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (marker, entry) = g.pick(&STUBS);
        // A table as `with_slot` makes one, a scenery slot standing in.
        let (other, _, _) = with_slot(&rig, &mut g, &mut s, 34, 0xc);
        let (col, row) = any_cell(&mut g);
        let mut at = cell(col, row);
        if other != 0 && g.one_in(6) {
            at = with_pokes(&rig, &s).l(other + 0x40);
        }
        // A pad's right neighbour (or not); the cells a swing's links go in, some taken.
        let st = with_pokes(&rig, &s);
        if g.one_in(2) {
            s.poke_w(
                at + 2,
                st.w(at + 2) & 0xffc0 | pick_or_any(&mut g, &[5, 0, 4]) & 0x3f,
            );
        }
        if marker == 27 && g.one_in(3) {
            let below = at + 0x260 * (1 + g.below(5) as u32);
            let pre = s.pokes.clone();
            s.pokes = prepared(&rig, &pre, |c| {
                objects::obj_spawn_cell_alt(c, template(34), below, 0, 0);
            });
        }
        s.poke_l(CAMERA_X, g.below(0x1300) as u32);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        s.poke_l(swing::LINK_INDEX, g.next() as u32);
        s.a[3] = at;
        let spawn = sf_core::level::standard_dispatch()
            .marker(2, marker)
            .expect("registered");
        compared += rig.check(&format!("marker {marker}"), case, entry, &s, |c| {
            spawn(c, at)
        }) as usize;
    }
    enough("spawners", compared, N);
}

/// A flow's neighbour two cells left or right, marked 24 and spawned there (or not), its bob on
/// some step.
fn flow_neighbour(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32) {
    let st = with_pokes(rig, s);
    let at = st.l(slot + 0x40);
    for side in [at.wrapping_sub(4), at.wrapping_add(4)] {
        if g.one_in(2) {
            s.poke_w(
                side,
                st.w(side) & 0xffc0 | pick_or_any(g, &[0x18, 0x18, 0]) & 0x3f,
            );
        }
    }
    if g.one_in(4) {
        return;
    }
    let side = if g.one_in(2) {
        at.wrapping_sub(4)
    } else {
        at.wrapping_add(4)
    };
    let pre = s.pokes.clone();
    let mut other = 0;
    s.pokes = prepared(rig, &pre, |c| {
        other = objects::obj_spawn_cell_alt(c, template(98), side, 0, 0xc).a0;
    });
    if other == 0 {
        return;
    }
    let ob = with_pokes(rig, s).l(other + 0x50);
    let step = scenery::FLOW_ANIM + 4 * g.below(0x13) as u32;
    s.poke_l(ob + 0x28, if g.one_in(3) { 0 } else { step });
    s.poke_l(ob + 0x16, if g.one_in(3) { g.next() as u32 } else { step });
}

#[test]
fn inits_match_the_original() {
    let mut rig = w3_rig();
    let mut g = Gen::new(4402);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (t, init, _) = g.pick(&TYPES);
        let (slot, _, _) = with_slot(&rig, &mut g, &mut s, t, 0xc);
        if slot == 0 {
            continue;
        }
        cases += 1;
        camera_near(&rig, &mut g, &mut s, slot);
        if t == 98 {
            flow_neighbour(&rig, &mut g, &mut s, slot);
        }
        s.poke_l(swing::LINK_INDEX, pick_or_any(&mut g, &[1, 2, 5]) as u32);
        s.poke_w(slot + 8, near(&mut g, 0x800, 0x800));
        s.poke_w(slot + 0xa, near(&mut g, 0x200, 0x200));
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(init))
            .expect("ported");
        compared += rig.check(
            &format!("init h0+${init:x} (template {t})"),
            case,
            init,
            &s,
            |c| port(c, slot, 0),
        ) as usize;
    }
    enough("inits", compared, cases);
}

/// The fields a type's update reads, varied.
fn vary(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32, bob: u32, t: u32) {
    // Speeds the game gives them (any word would carry a position off the map).
    s.poke_w(slot + 0xc, g.pick(&[0, 1, 0xffff, 2, 0xfffe, 8, 0xfff8]));
    s.poke_w(
        slot + 0xe,
        g.pick(&[
            0, 1, 0xffff, 2, 5, 6, 7, 8, 9, 0xfff0, 0xfff4, 0xfff7, 0xfff8,
        ]),
    );
    s.poke_w(slot + 0x5c, pick_or_any(g, &[0, 1, 2, 3, 0xffff, 0x19]));
    s.poke_w(
        slot + 0x60,
        pick_or_any(g, &[0, 1, 2, 7, 8, 9, 0xffff, 0x4b]),
    );
    s.poke_w(slot + 0x64, pick_or_any(g, &[0, 1, 2, 0xffff, 0x20, 0x60]));
    // Near the map's left and top edges now and then.
    if g.one_in(4) {
        s.poke_w(slot + 8, g.pick(&[0, 4, 0xa, 0xb, 0x10, 0x1f, 0x20, 0x21]));
    }
    if g.one_in(6) {
        s.poke_w(slot + 0xa, g.pick(&[0x10, 0x11, 0x14, 0x18, 0x19, 0x20]));
    }
    // A bob's step timer, and a step that is (or is not) one an update looks for.
    s.poke_w(
        bob + 0x1a,
        pick_or_any(g, &[0, 0x1ff, 0x200, 0x4e20, 0x7d00]),
    );
    if g.one_in(2) {
        s.poke_l(bob + 8, g.pick(&[SPLAT, h0(0x3215c), h0(0x32140)]));
    }
    if g.one_in(2) {
        let frame = g.pick(&[ANIM_END, SPLAT, GIVEN_WAY]);
        fake_step(s, slot, bob, frame);
        // The shuffler reads the step's word +6: the frame record's, here the spare bytes'.
        s.poke_w(slot + 0x7a, pick_or_any(g, &[0, 0, 1]));
    }
    match t {
        65 => {
            let n = longs_until(swing::PATH, |l| l == 0xffff_ffff);
            s.poke_l(slot + 0x54, swing::PATH + 4 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, swing::PATH);
            some_swing_bobs(rig, g, s);
        }
        66 => {
            // Its ball: the slot made by another spawn, or any slot, or nothing.
            let pre = s.pokes.clone();
            let (col, row) = any_cell(g);
            let mut ball = 0;
            s.pokes = prepared(rig, &pre, |c| {
                ball = objects::obj_spawn_cell_alt(c, template(65), cell(col, row), 0, 0xc).a0;
            });
            let ball = match g.below(6) {
                0 => OBJ_SLOTS + SLOT * g.below(20) as u32,
                1 => 0,
                _ => ball,
            };
            s.poke_l(slot + 0x64, ball);
            if ball != 0 && g.one_in(2) {
                let st = with_pokes(rig, s);
                let (bx, by) = (st.w(ball + 8), st.w(ball + 0xa));
                s.poke_w(ball + 0x6c, near(g, bx, 0x60));
                s.poke_w(ball + 0x6e, near(g, by, 0x60));
                s.poke_w(ball + 0x70, near(g, bx, 0x60));
                s.poke_w(ball + 0x72, near(g, by, 0x60));
            }
            s.poke_l(slot + 0x68, pick_or_any(g, &[1, 2, 3, 4, 5, 6]) as u32);
            some_swing_bobs(rig, g, s);
        }
        94 => {
            let n = words_until(hopper::PATH, 0x80);
            s.poke_l(slot + 0x68, hopper::PATH + 2 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x6c, hopper::PATH);
            s.poke_w(slot + 0x70, near(g, 0x200, 0x200));
        }
        95 => {
            let n = longs_until(hover::PATH, |l| l == 0xffff_ffff);
            s.poke_l(slot + 0x54, hover::PATH + 4 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, hover::PATH);
            let st = with_pokes(rig, s);
            let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
            s.poke_w(slot + 0x68, near(g, x, 0x30));
            s.poke_w(slot + 0x6a, near(g, y, 0x30));
        }
        99 if g.one_in(3) => {
            s.poke_w(slot + 8, 0x6d60);
            s.poke_w(slot + 0xa, 0x6d60);
        }
        _ => {}
    }
}

#[test]
fn updates_match_the_original() {
    let mut rig = w3_rig();
    let mut g = Gen::new(4403);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (mut t, _, mut update) = g.pick(&TYPES);
        if g.one_in(4) {
            (update, t) = g.pick(&LATER);
        }
        let dy = g.pick(&[0, 4, 0xa, 0xc, 0x11]);
        let (slot, bob, at) = with_slot(&rig, &mut g, &mut s, t, dy);
        if slot == 0 {
            continue;
        }
        cases += 1;
        // The init it ran set its fields; now vary them.
        vary(&rig, &mut g, &mut s, slot, bob, t);
        sprinkle(&rig, &mut g, &mut s, at);
        camera_near(&rig, &mut g, &mut s, slot);
        frog_near(&rig, &mut g, &mut s, slot);
        any_rng(&mut g, &mut s);
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
    let mut rig = w3_rig();
    let mut g = Gen::new(4404);
    let mut compared = 0;
    let mut cases = 0;
    // The touch handlers world 3 adds or names, each with types that name it.
    let touches: [(u32, &[u32]); 4] = [
        (0x236ac, &[55, 56, 65]),
        (0x23774, &[150]),
        (0x23582, &[88, 94, 95]),
        (0x2360e, &[91, 92, 95, 96, 97, 99]),
    ];
    for case in 0..N {
        let mut s = Setup::default();
        let (touch, types) = g.pick(&touches);
        let t = g.pick(types);
        let (slot, bob, at) = with_slot(&rig, &mut g, &mut s, t, 4);
        if slot == 0 {
            continue;
        }
        cases += 1;
        s.poke_w(
            slot + 0x1c,
            pick_or_any(&mut g, &[0x1f, 0x20, 0x3f, 0x7d00, 0]),
        );
        s.poke_w(slot + 0x5c, pick_or_any(&mut g, &[0xffff, 0, 1, 4]));
        s.poke_w(
            bob + 0x1a,
            pick_or_any(&mut g, &[0, 0x1ff, 0x200, 0x201, 0x7d00, 0x8000]),
        );
        sprinkle(&rig, &mut g, &mut s, at);
        camera_near(&rig, &mut g, &mut s, slot);
        frog_near(&rig, &mut g, &mut s, slot);
        // Hurting needs a frog that can be hurt: hearts, and now and then hurt already.
        s.poke_w(FROG + 0x5a, pick_or_any(&mut g, &[0, 1, 2, 3]));
        s.poke_w(FROG + f::HURT, pick_or_any(&mut g, &[0, 0, 1]));
        any_rng(&mut g, &mut s);
        let d7 = g.below(2) as u16;
        s.a[5] = slot;
        s.d[7] = d7 as u32;
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

/// A link's place along its chain, at the divide's edges: a long chain (any span either way)
/// with indices whose quotient overflows `divu.w #6`.
#[test]
fn a_link_sits_as_the_original_at_the_divides_edges() {
    let mut rig = w3_rig();
    let mut g = Gen::new(4405);
    let mut case = 0;
    for span in [0u16, 1, 5, 6, 0x7fff, 0x8000, 0x8001, 0xffff, 0xfffa] {
        for index in [0u32, 1, 5, 6, 7, 0xffff, 0x1_0003] {
            let mut s = Setup::default();
            let (link, _, _) = with_slot(&rig, &mut g, &mut s, 66, 0);
            let pre = s.pokes.clone();
            let mut ball = 0;
            s.pokes = prepared(&rig, &pre, |c| {
                ball = objects::obj_spawn_cell_alt(c, template(65), cell(150, 30), 0, 0xc).a0;
            });
            if link == 0 || ball == 0 {
                continue;
            }
            s.poke_l(link + 0x64, ball);
            s.poke_l(link + 0x68, index);
            s.poke_w(ball + 0x70, 0x4000);
            s.poke_w(ball + 0x6c, 0x4000u16.wrapping_sub(span));
            s.poke_w(ball + 0x72, 0x200);
            s.poke_w(ball + 0x6e, 0x200u16.wrapping_add(span));
            s.a[0] = link;
            let port = sf_core::level::standard_dispatch()
                .routine(swing::LINK_UPDATE)
                .expect("ported");
            assert!(rig.check("link at an edge", case, 0x262be, &s, |c| port(c, link, 0)));
            case += 1;
        }
    }
    assert!(case > 40, "only {case} cases");
}
