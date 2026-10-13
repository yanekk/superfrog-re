//! World 1's enemies (`sf_core::enemies::w1`, T42) and what they share (`sf_core::enemies::common`)
//! against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at W1-1's first frame and the port from
//! the same memory transplanted, over generated states: every byte written, the sound driver's
//! state and the Paula writes must match (`sf_conform::routine`). Slots are made by the engine's
//! own spawners, so a generated slot is as consistent as the game makes one; the map cells around
//! it are sprinkled with the markers the routines look for (`$25` turns, solid cells, the
//! patrollers' ends).
//!
//! Lives here, not in `sf-core/tests/` as T42's doc says, because the purity test keeps the
//! emulator out of the pure crates' tests.

mod common;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_core::enemies::w1::{dropper, hover, spout, watcher};
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
/// The shared word the crawler reads (h0+$2d128).
const CRAWLER_STOMPABLE: u32 = h0(0x2d128);
const ANIM_END: u32 = h0(0x2af0a);
const RNG_STATE: u32 = h0(0x160d4);
const SPLAT: u32 = h0(0x2cef4);

/// Each type's template, init and update.
const TYPES: [(u32, u32, u32); 16] = [
    (8, 0x2435a, 0x243b6),
    (12, 0x2493a, 0x249c2),
    (13, 0x2493a, 0x2496a),
    (14, 0x24a58, 0x24aac),
    (15, 0x24a74, 0x24aac),
    (16, 0x24a90, 0x24aac),
    (17, 0x24b0a, 0x24b20),
    (19, 0x2483e, 0x24854),
    (20, 0x24828, 0x24854),
    (21, 0x248b2, 0x248e2),
    (22, 0x24788, 0x247b8),
    (23, 0x24d3c, 0x24dca),
    (33, 0x2533a, 0x25358),
    (76, 0x268ee, 0x26904),
    (77, 0x269be, 0x26a2c),
    (78, 0x269f8, 0x26a2c),
];

/// The updates a slot turns to: a killed one's fall, a stomped one's, the dropper climbing, the
/// drop splatted; with a type that can reach each.
const LATER: [(u32, u32); 4] = [(0x23b34, 8), (0x23bee, 13), (0x24e48, 23), (0x24ba2, 17)];

/// Each marker's stub and the y offset it spawns at.
const STUBS: [(u8, u32); 12] = [
    (1, 0x18040),
    (2, 0x1804e),
    (3, 0x1805e),
    (10, 0x1806e),
    (11, 0x1807e),
    (12, 0x1808e),
    (13, 0x1809e),
    (14, 0x180ae),
    (15, 0x180be),
    (16, 0x180ce),
    (17, 0x180ec),
    (18, 0x1810a),
];

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
    assert!(compared * 100 >= n * 99, "{name}: only {compared} of {n} cases compared");
}

/// A map cell somewhere on the level (0-indexed column and row, away from its edges).
fn any_cell(g: &mut Gen) -> (u32, u32) {
    (4 + g.below(286) as u32, 4 + g.below(74) as u32)
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

/// The rig's state with `s`'s pokes in.
fn with_pokes(rig: &Rig, s: &Setup) -> sf_core::State {
    let mut st = rig.state().clone();
    for (a, b) in &s.pokes {
        st.poke(*a, b);
    }
    st
}

/// A generated table: some slots freed, now and then the rest filled with sitters so a spawn
/// finds no room; then `t` spawned at a cell by the engine, `dy` into it. The slot and its bob
/// (0, 0 with no room), and the cell.
fn with_slot(rig: &Rig, g: &mut Gen, s: &mut Setup, t: u32, dy: u16) -> (u32, u32, (u32, u32)) {
    let (col, row) = any_cell(g);
    let free: Vec<u32> = (0..20).filter(|_| g.one_in(3)).collect();
    let fill = if g.one_in(5) { g.below(21) } else { 0 };
    let fill_at: Vec<(u16, u16)> = (0..fill)
        .map(|_| (g.below(0x1200) as u16, g.below(0x500) as u16))
        .collect();
    let (phase, turn) = (g.below(4) as u16, g.below(3) as u16);
    let pre = s.pokes.clone();
    let (mut slot, mut bob) = (0, 0);
    let pokes = prepared(rig, &pre, |c| {
        c.bus.set_w(hover::PHASE, phase);
        c.bus.set_w(spout::TURN, turn);
        for &k in &free {
            objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
        }
        let r = objects::obj_spawn_cell_alt(c, template(t), cell(col, row), 0, dy);
        slot = r.a0;
        if slot != 0 {
            bob = c.bus.l(slot + 0x50);
        }
        for &(x, y) in &fill_at {
            objects::obj_spawn_at_xy_nodedup(c, template(12), x, y);
        }
    });
    s.pokes = pokes;
    (slot, bob, (col, row))
}

/// Markers the routines look for, sprinkled over the cells around (`col`, `row`), each cell's
/// tile kept.
fn sprinkle(rig: &Rig, g: &mut Gen, s: &mut Setup, (col, row): (u32, u32)) {
    let st = with_pokes(rig, s);
    for dr in 0..6 {
        for dc in 0..9 {
            if !g.one_in(4) {
                continue;
            }
            let (c, r) = ((col + dc).saturating_sub(4), (row + dr).saturating_sub(3));
            let at = cell(c.min(MAP_ROW_CELLS - 1), r);
            let m = pick_or_any(g, &[0, 0x25, 0x3a, 0x3d, 0x3e, 0x3f, 0x10, 0x11, 0x39, 9]) & 0x3f;
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

/// Any RNG state (it never moves in W1-1, so the rig's would draw the same number every case).
fn any_rng(g: &mut Gen, s: &mut Setup) {
    s.poke_l(RNG_STATE, g.next() as u32);
    s.poke_l(RNG_STATE + 4, g.next() as u32);
}

/// The frog somewhere near `slot`, or anywhere.
fn frog_near(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32) {
    let st = with_pokes(rig, s);
    let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
    s.poke_w(FROG + f::X, near(g, x, 0x80));
    s.poke_w(FROG + f::Y, near(g, y, 0x60));
}

/// The longs of the path at `start` up to and with the first that `is_end`.
fn longs_until(start: u32, is_end: impl Fn(u32) -> bool) -> u32 {
    let m = level();
    let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
    (0..).find(|&k| is_end(long(start + 4 * k))).expect("an end mark") + 1
}

/// The words of the path at `start` up to and with `end`.
fn words_until(start: u32, end: u16) -> u32 {
    let m = level();
    let word = |a: u32| (m.byte(a) as u16) << 8 | m.byte(a + 1) as u16;
    (0..).find(|&k| word(start + 2 * k) == end).expect("an end mark") + 1
}

#[test]
fn the_kinds_and_markers_are_registered() {
    let d = sf_core::level::standard_dispatch();
    let m = level();
    let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
    for (t, init, update) in TYPES {
        assert!(d.kind(t as u16).is_some(), "template {t}");
        assert_eq!(long(template(t)), h0(init), "template {t}'s init");
        assert_eq!(long(template(t) + 4), h0(update), "template {t}'s update");
        assert!(d.routine(h0(init)).is_some() && d.routine(h0(update)).is_some());
        let touch = long(template(t) + 0x10);
        assert!(d.routine(touch).is_some(), "template {t}'s touch ${touch:x}");
    }
    for (u, _) in LATER {
        assert!(d.routine(h0(u)).is_some(), "h0+${u:x}");
    }
    for (marker, _) in STUBS {
        assert!(d.marker(0, marker).is_some(), "marker {marker}");
    }
}

#[test]
fn the_paths_end_where_the_listing_says() {
    // Four phases of 51 positions, then a −1 long.
    assert_eq!(longs_until(hover::PATH, |l| l == 0xffff_ffff), 4 * 0x33 + 1);
    assert_eq!(words_until(dropper::PATH, 0x80), 0x2b);
    // The watcher's sixteen looks are each an animation in the bobs' records.
    let m = level();
    let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
    for k in 0..16 {
        assert!((h0(0x31000)..h0(0x34000)).contains(&long(watcher::LOOKS + 4 * k)));
    }
}

#[test]
fn spawners_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(4201);
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (marker, entry) = g.pick(&STUBS);
        // A table as `with_slot` makes one, a sitter standing in.
        let (other, _, _) = with_slot(&rig, &mut g, &mut s, 12, 4);
        let (col, row) = any_cell(&mut g);
        let mut at = cell(col, row);
        if other != 0 && g.one_in(6) {
            at = with_pokes(&rig, &s).l(other + 0x40);
        }
        // A patroller's other end somewhere along the row (or not), and now and then the
        // sitter spawned there, as if that end's patroller were out.
        if matches!(marker, 16 | 17) && !g.one_in(5) {
            let right = if g.one_in(5) { marker == 17 } else { marker == 16 };
            let k = 1 + g.below(0xa0) as u32;
            let end = if right { at.wrapping_add(2 * k) } else { at.wrapping_sub(2 * k) };
            let want = if marker == 16 { 0x11 } else { 0x10 };
            let st = with_pokes(&rig, &s);
            s.poke_w(end, st.w(end) & 0xffc0 | want);
            if g.one_in(3) {
                let pre = s.pokes.clone();
                s.pokes = prepared(&rig, &pre, |c| {
                    objects::obj_spawn_cell_alt(c, template(12), end, 0, 4);
                });
            }
        }
        s.poke_l(CAMERA_X, g.below(0x1300) as u32);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        s.poke_w(hover::PHASE, pick_or_any(&mut g, &[0, 1, 2, 3]));
        s.a[3] = at;
        let spawn = sf_core::level::standard_dispatch().marker(0, marker).expect("registered");
        compared +=
            rig.check(&format!("marker {marker}"), case, entry, &s, |c| spawn(c, at)) as usize;
    }
    enough("spawners", compared, N);
}

#[test]
fn inits_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(4202);
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
        s.poke_w(hover::PHASE, pick_or_any(&mut g, &[0, 1, 2, 3, 4, 0xffff]));
        s.poke_w(slot + 8, near(&mut g, 0x800, 0x800));
        s.poke_w(slot + 0xa, near(&mut g, 0x200, 0x200));
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch().routine(h0(init)).expect("ported");
        compared += rig.check(&format!("init h0+${init:x} (template {t})"), case, init, &s, |c| {
            port(c, slot, 0)
        }) as usize;
    }
    enough("inits", compared, cases);
}

/// The fields a type's update reads, varied.
fn vary(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32, bob: u32, t: u32) {
    // Speeds the game gives them (any word would carry a position off the map).
    s.poke_w(slot + 0xc, g.pick(&[0, 1, 0xffff, 2, 0xfffe, 4, 0xfffc, 6, 0xfffa]));
    s.poke_w(slot + 0xe, g.pick(&[0, 1, 0xffff, 7, 8, 9, 0xb, 0xc, 0xd, 0xfff4, 0xfff9, 0xffe0]));
    s.poke_w(slot + 0x5c, pick_or_any(g, &[0, 1, 2, 0xffff, 0x1e, 0x32]));
    s.poke_w(slot + 0x60, pick_or_any(g, &[0, 1, 2, 0xffff, 0x4b]));
    // Near the map's left and top edges now and then (a position off the map reads row
    // pointers from memory no variable keeps: nothing the game reaches).
    if g.one_in(4) {
        s.poke_w(slot + 8, g.pick(&[0, 4, 0x1f, 0x20, 0x21, 0x24, 0xfffc]));
    }
    if g.one_in(6) {
        s.poke_w(slot + 0xa, g.pick(&[0x10, 0x11, 0x14, 0x18, 0x19, 0x20]));
    }
    s.poke_w(CRAWLER_STOMPABLE, pick_or_any(g, &[0, 1, 2, 3, 0xffff]));
    s.poke_w(spout::TURN, g.below(4) as u16);
    // A splat's step timer, and a step that is (or is not) the animation's end.
    s.poke_w(bob + 0x1a, pick_or_any(g, &[0, 0x4e1f, 0x4e20, 0x4e21, 0x7d00]));
    if g.one_in(2) {
        s.poke_l(bob + 8, SPLAT);
    }
    if g.one_in(2) {
        s.poke_l(slot + 0x74, if g.one_in(2) { ANIM_END } else { SPLAT });
        s.poke_l(bob + 0x16, slot + 0x74);
    }
    match t {
        8 => {
            let n = longs_until(hover::PATH, |l| l == 0xffff_ffff);
            s.poke_l(slot + 0x54, hover::PATH + 4 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x58, hover::PATH);
            let st = with_pokes(rig, s);
            let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
            s.poke_w(slot + 0x64, near(g, x, 0x30));
            s.poke_w(slot + 0x66, near(g, y, 0x30));
            s.poke_w(slot + 0x68, near(g, x, 0x30));
            s.poke_w(slot + 0x6a, near(g, y, 0x30));
        }
        23 => {
            let n = words_until(dropper::PATH, 0x80);
            s.poke_l(slot + 0x64, dropper::PATH + 2 * g.below(n as u64) as u32);
            s.poke_l(slot + 0x68, dropper::PATH);
            s.poke_w(slot + 0x6c, near(g, 0x200, 0x200));
        }
        _ => {}
    }
}

#[test]
fn updates_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(4203);
    let mut compared = 0;
    let mut cases = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (mut t, _, mut update) = g.pick(&TYPES);
        if g.one_in(4) {
            (update, t) = g.pick(&LATER);
        }
        let dy = g.pick(&[0, 4, 7, 0xc, 0x16]);
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
        if std::env::var("T42DBG").is_ok_and(|v| v == case.to_string()) {
            let st = with_pokes(&rig, &s);
            let (x, y, vx) = (st.w(slot + 8), st.w(slot + 0xa), st.w(slot + 0xc));
            let rows = st.l(h0(0x17334) + (((y.wrapping_sub(8)) >> 4).wrapping_mul(4)) as i16 as i32 as u32);
            let a = rows + ((x >> 4) * 2) as u32 + 2;
            eprintln!("slot {slot:x} x {x:x} y {y:x} vx {vx:x} rows {rows:x} cell {a:x} = {:x} slot40 {:x}", st.w(a), st.l(slot+0x40));
        }
        let port = sf_core::level::standard_dispatch().routine(h0(update)).expect("ported");
        compared += rig.check(&format!("update h0+${update:x} (template {t})"), case, update, &s, |c| {
            port(c, slot, 0)
        }) as usize;
    }
    enough("updates", compared, cases);
}

#[test]
fn touches_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(4204);
    let mut compared = 0;
    let mut cases = 0;
    // The touch handlers, each with a type that names it (or turns to it).
    let touches: [(u32, u32); 3] = [(0x23582, 8), (0x2360e, 13), (0x23662, 12)];
    for case in 0..N {
        let mut s = Setup::default();
        let (touch, mut t) = g.pick(&touches);
        if touch == 0x23582 && g.one_in(2) {
            t = g.pick(&[19, 20, 77, 78]);
        }
        if touch != 0x23582 && g.one_in(2) {
            t = g.pick(&[12, 13, 21, 22]);
        }
        let (slot, _, at) = with_slot(&rig, &mut g, &mut s, t, 4);
        if slot == 0 {
            continue;
        }
        cases += 1;
        s.poke_w(slot + 0x1c, pick_or_any(&mut g, &[0x1f, 0x20, 0x3f, 0x40, 0x7d00, 0]));
        s.poke_w(slot + 0x5c, pick_or_any(&mut g, &[0xffff, 0, 1, 4]));
        sprinkle(&rig, &mut g, &mut s, at);
        camera_near(&rig, &mut g, &mut s, slot);
        any_rng(&mut g, &mut s);
        let d7 = g.below(2) as u16;
        s.a[5] = slot;
        s.d[7] = d7 as u32;
        let port = sf_core::level::standard_dispatch().routine(h0(touch)).expect("ported");
        compared += rig.check(&format!("touch h0+${touch:x} (template {t})"), case, touch, &s, |c| {
            port(c, slot, d7)
        }) as usize;
    }
    enough("touches", compared, cases);
}

/// The watcher's direction to the frog (h0+$da62) at and either side of each of its four
/// steepness thresholds, in each quadrant, and with either off the screen (clamped).
#[test]
fn the_watcher_looks_as_the_original_at_every_threshold() {
    let mut rig = rig();
    let mut g = Gen::new(4205);
    let mut cases = Vec::new();
    for t in [0x26u32, 0x80, 0x11f, 0x3c5] {
        for q in [t - 1, t, t + 1] {
            // A dy, dx on the screen with dy * $c0 / dx == q.
            if let Some((dy, dx)) = (1..0xe0u32)
                .flat_map(|dy| (1..0x140u32).map(move |dx| (dy, dx)))
                .find(|&(dy, dx)| dy * 0xc0 / dx == q)
            {
                for (sx, sy) in [(1i32, 1i32), (-1, 1), (-1, -1), (1, -1)] {
                    cases.push((sx * dx as i32, sy * dy as i32, true));
                }
            }
        }
    }
    for _ in 0..400 {
        cases.push((g.below(0x400) as i32 - 0x200, g.below(0x300) as i32 - 0x180, false));
    }
    cases.extend(
        [(0, 0), (0, 5), (0, -5), (5, 0), (-5, 0), (0x150, -0xee), (1, 0xef)].map(|(x, y)| (x, y, false)),
    );
    for (case, (dx, dy, on_screen)) in cases.into_iter().enumerate() {
        let mut s = Setup::default();
        let (slot, _, _) = with_slot(&rig, &mut g, &mut s, 76, 0xc);
        if slot == 0 {
            continue;
        }
        let st = with_pokes(&rig, &s);
        let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
        // The watcher anywhere on the screen, or off it.
        // A threshold case keeps both on the screen, so neither is clamped.
        let (wx, wy) = if on_screen {
            (
                if dx < 0 { 0x153 - g.below(0x153 - (-dx) as u64) as u16 } else { g.below(0x153 - dx as u64) as u16 },
                if dy > 0 { 0xef - g.below(0xef - dy as u64) as u16 } else { g.below(0xef - (-dy) as u64) as u16 },
            )
        } else if g.one_in(5) {
            (g.below(0x200) as u16, g.below(0x140) as u16)
        } else {
            (0x20 + g.below(0x100) as u16, 0x20 + g.below(0xa0) as u16)
        };
        s.poke_l(CAMERA_X, x.wrapping_sub(wx) as u32);
        s.poke_l(CAMERA_Y, y.wrapping_sub(wy) as u32);
        // Screen y grows down; the direction's dy grows up.
        s.poke_w(FROG + f::X, x.wrapping_add(dx as u16));
        s.poke_w(FROG + f::Y, y.wrapping_sub(dy as u16));
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch().routine(h0(0x26904)).expect("ported");
        assert!(rig.check("watcher", case, 0x26904, &s, |c| port(c, slot, 0)));
    }
    // The clamp's edges: the watcher on the screen's last column or line, the frog level with it
    // on, at or past that edge (the clamp makes them one point), and either past the first.
    let edges: [(u16, u16, u16, u16); 10] = [
        (0x153, 0x60, 0x153, 0x60),
        (0x153, 0x60, 0x154, 0x60),
        (0x153, 0x60, 0x160, 0x60),
        (0x80, 0xef, 0x80, 0xf0),
        (0x80, 0xef, 0x80, 0x100),
        (0x80, 0xef, 0x80, 0xef),
        (0, 0x60, 0xffff, 0x60),
        (0, 0x60, 0xff00, 0x60),
        (0x80, 0, 0x80, 0xffff),
        (0xffff, 0xffff, 0, 0),
    ];
    for (case, (wx, wy, fx, fy)) in edges.into_iter().enumerate() {
        let mut s = Setup::default();
        let (slot, _, _) = with_slot(&rig, &mut g, &mut s, 76, 0xc);
        assert!(slot != 0);
        let st = with_pokes(&rig, &s);
        let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
        let (cx, cy) = (x.wrapping_sub(wx), y.wrapping_sub(wy));
        s.poke_l(CAMERA_X, cx as u32);
        s.poke_l(CAMERA_Y, cy as u32);
        s.poke_w(FROG + f::X, cx.wrapping_add(fx));
        s.poke_w(FROG + f::Y, cy.wrapping_add(fy));
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch().routine(h0(0x26904)).expect("ported");
        assert!(rig.check("watcher at an edge", case, 0x26904, &s, |c| port(c, slot, 0)));
    }
}
