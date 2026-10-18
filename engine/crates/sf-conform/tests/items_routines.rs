//! The items' routines (`sf_core::items`, the object core in `sf_core::level::objects`, deaths in
//! `sf_core::level::death`, T15) against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at the first frame of W1-1 (its objects
//! live: an enemy and seven coins) and the port from the same memory transplanted, over generated
//! states. Every byte written, the sound driver's state and the Paula writes must match
//! (`sf_conform::routine`). Slots are made by the engine's own spawners (checked here first), so
//! a generated slot is as consistent as the game makes one.
//!
//! Lives here, not in `sf-core/tests/`, because the purity test keeps the emulator out of the pure
//! crates' tests.

mod common;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_core::frog::{FROG, f};
use sf_core::items::{self, fixtures, pickups, powerups, template};
use sf_core::layout::{LAYOUT, VarId, h0};
use sf_core::level::objects::{self, OBJ_SLOTS};
use sf_core::level::{Hw, LevelCtx, death};
use sf_core::mem::Bus;

/// States per routine: fewer than the frog's, as each builds its slots with the engine first.
const N: usize = 3000;

const ATTRACT_MODE: u32 = h0(0x18);
const OBJ_WORLD_OFFSET: u32 = h0(0x9fa);
const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);
const SCROLL_DIRS: u32 = h0(0x16682);
const COINS_LEFT: u32 = h0(0x832);
const SLOT: u32 = 0x7c;

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
        // The enemies the original has live (T16's) are gone first: their routines are not
        // ported.
        for k in 0..20 {
            let slot = OBJ_SLOTS + k * SLOT;
            if c.bus.l(slot + 0x40) != 0 && c.dispatch.routine(c.bus.l(slot + 4)).is_none() {
                objects::obj_free(&mut c, slot, 0);
            }
        }
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

/// A map cell somewhere on the level (0-indexed column and row, away from its edges).
fn any_cell(g: &mut Gen) -> (u32, u32) {
    (2 + g.below(290) as u32, 2 + g.below(78) as u32)
}

/// The camera somewhere near `(x, y)` (or anywhere), and the way it last scrolled.
fn camera_near(g: &mut Gen, s: &mut Setup, x: u32, y: u32) {
    let near = |g: &mut Gen, v: u32, span: u64| -> u32 {
        if g.one_in(6) {
            g.below(0x1400) as u32
        } else {
            (v + g.below(span) as u32).saturating_sub(span as u32 / 2 + 0x80)
        }
    };
    s.poke_l(CAMERA_X, near(g, x, 0x300));
    s.poke_l(CAMERA_Y, near(g, y, 0x200));
    s.poke_b(SCROLL_DIRS, g.next() as u8);
}

/// The slot the engine's spawn put `template` in, at `cell`, with the slots before it freed first
/// as `free` says (the rest as at W1-1's start), and that slot's address.
fn with_slot(
    rig: &Rig,
    g: &mut Gen,
    s: &mut Setup,
    t: u32,
    (col, row): (u32, u32),
    dy: u16,
) -> u32 {
    let pre = s.pokes.clone();
    // Free some of the live slots first, so the new one lands anywhere.
    let free: Vec<u32> = (0..20).filter(|_| g.one_in(3)).collect();
    let mut slot = 0;
    let cell = cell(col, row);
    let pokes = prepared(rig, &pre, |c| {
        for &k in &free {
            objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
        }
        slot = objects::obj_spawn_cell_alt(c, t, cell, 0, dy).a0;
    });
    s.pokes = pokes;
    slot
}

#[test]
fn obj_spawn_cell_and_its_variants_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(101);
    for case in 0..N {
        let mut s = Setup::default();
        // Fill some slots with coins first (evictable or not), so the search, the evictions and
        // the full table are all reached.
        let n = g.below(22);
        let mut pre = Vec::new();
        for _ in 0..n {
            let (col, row) = any_cell(&mut g);
            pre.push((col, row, g.one_in(2)));
        }
        let pokes = prepared(&rig, &[], |c| {
            for &(col, row, evict) in &pre {
                let r = objects::obj_spawn_cell(c, template(0), cell(col, row), 0, 0xd);
                if r.a0 != 0 && evict {
                    c.bus.set_w(r.a0 + 0x2a, 1);
                }
            }
        });
        s.pokes = pokes;
        let (col, row) = if g.one_in(4) && !pre.is_empty() {
            let k = g.below(pre.len() as u64) as usize;
            (pre[k].0, pre[k].1)
        } else {
            any_cell(&mut g)
        };
        camera_near(&mut g, &mut s, col * 16, row * 16);
        let at = cell(col, row);
        let t = template(g.pick(&[0, 2, 4, 5, 9, 10, 42, 43]));
        let (dx, dy) = (g.below(16) as u16, (g.below(32) as i16 - 16) as u16);
        s.a[0] = t;
        s.a[3] = at;
        s.d[0] = dx as u32;
        s.d[1] = dy as u32;
        match g.below(3) {
            0 => {
                rig.check("obj_spawn_cell", case, 0x1b3d8, &s, |c| {
                    objects::obj_spawn_cell(c, t, at, dx, dy);
                });
            }
            1 => {
                rig.check("obj_spawn_cell_alt", case, 0x1b4e6, &s, |c| {
                    objects::obj_spawn_cell_alt(c, t, at, dx, dy);
                });
            }
            _ => {
                rig.check("obj_spawn_coin_forced", case, 0x1b5d0, &s, |c| {
                    objects::obj_spawn_coin_forced(c, t, at, dx, dy);
                });
            }
        }
    }
}

#[test]
fn obj_spawn_at_xy_and_nodedup_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(102);
    for case in 0..N {
        let mut s = Setup::default();
        let n = g.below(22);
        let mut pre = Vec::new();
        for _ in 0..n {
            let (col, row) = any_cell(&mut g);
            pre.push((
                col as u16 * 16 + g.below(16) as u16,
                row as u16 * 16,
                g.one_in(2),
            ));
        }
        let pokes = prepared(&rig, &[], |c| {
            for &(x, y, evict) in &pre {
                let r = objects::obj_spawn_at_xy(c, template(2), x, y);
                if r.a0 != 0 && evict {
                    c.bus.set_w(r.a0 + 0x2a, 1);
                }
            }
        });
        s.pokes = pokes;
        let (x, y) = if g.one_in(4) && !pre.is_empty() {
            let k = g.below(pre.len() as u64) as usize;
            (pre[k].0, pre[k].1)
        } else {
            let (col, row) = any_cell(&mut g);
            (
                col as u16 * 16 + g.below(16) as u16,
                row as u16 * 16 + g.below(16) as u16,
            )
        };
        camera_near(&mut g, &mut s, x as u32, y as u32);
        let t = template(g.pick(&[0, 2, 5]));
        s.a[0] = t;
        s.d[0] = x as u32;
        s.d[1] = y as u32;
        if g.one_in(2) {
            rig.check("obj_spawn_at_xy", case, 0x1b706, &s, |c| {
                objects::obj_spawn_at_xy(c, t, x, y);
            });
        } else {
            rig.check("obj_spawn_at_xy_nodedup", case, 0x1b822, &s, |c| {
                objects::obj_spawn_at_xy_nodedup(c, t, x, y);
            });
        }
    }
}

#[test]
fn freeing_and_despawning_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(103);
    for case in 0..N {
        let mut s = Setup::default();
        let (col, row) = any_cell(&mut g);
        let t = template(g.pick(&[0, 2, 5]));
        let slot = with_slot(&rig, &mut g, &mut s, t, (col, row), 0xd);
        camera_near(&mut g, &mut s, col * 16, row * 16);
        if g.one_in(4) {
            s.poke_w(h0(0x9dc), 1);
        }
        let slot = if g.one_in(8) {
            OBJ_SLOTS + SLOT * g.below(20) as u32
        } else {
            slot
        };
        s.a[0] = slot;
        match g.below(3) {
            0 => {
                rig.check("obj_free", case, 0x1b90e, &s, |c| {
                    objects::obj_free(c, slot, 0)
                });
            }
            1 => {
                rig.check("obj_despawn_if_offscreen", case, 0x1b95e, &s, |c| {
                    objects::obj_despawn_if_offscreen(c, slot, 0)
                });
            }
            _ => {
                rig.check("obj_free_if_left_behind", case, 0x1b940, &s, |c| {
                    objects::obj_free_if_left_behind(c, slot, 0)
                });
            }
        }
    }
}

/// A slot of `t` with its own fields varied as its routines read them.
fn any_object(rig: &Rig, g: &mut Gen, s: &mut Setup, t: u32) -> (u32, (u32, u32)) {
    let (col, row) = any_cell(g);
    let slot = with_slot(rig, g, s, t, (col, row), 0xd);
    camera_near(g, s, col * 16, row * 16);
    for o in [0x5c, 0x60] {
        s.poke_w(
            slot + o,
            g.pick(&[0, 1, 2, 4, 5, 6, 7, 0x10, 0xffff, 0xfffe]),
        );
    }
    s.poke_w(slot + 0xe, g.pick(&[0, 1, 7, 8, 0xffff, 0xfffc]));
    s.poke_w(slot + 0x68, g.below(2) as u16);
    s.poke_w(COINS_LEFT, g.pick(&[0, 1, 2, 30]));
    s.poke_l(OBJ_WORLD_OFFSET, 0x100 * g.below(6) as u32);
    s.poke_w(h0(0x9e6), g.below(3) as u16);
    // The cells below it, for a falling coin.
    for r in row..(row + 3).min(84) {
        s.poke_w(
            cell(col, r),
            (g.below(840) as u16) << 6 | g.pick(&[0, 0x3d, 0x3e, 0x3f, 9]),
        );
    }
    (slot, (col, row))
}

/// The ported update routines, by the original's address.
fn update_routines() -> Vec<(u32, u32)> {
    vec![
        (0, 0x23f86),
        (2, 0x23fae),
        (4, 0x2408a),
        (5, 0x2404c),
        (6, 0x241b2),
        (7, 0x241b2),
        (9, 0x240ea),
        (10, 0x240ea),
        (42, 0x255f6),
        (47, 0x255f6),
    ]
}

#[test]
fn object_updates_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(104);
    for case in 0..N {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        let (t, entry) = g.pick(&update_routines());
        let (slot, _) = any_object(&rig, &mut g, &mut s, template(t));
        if slot == 0 {
            continue;
        }
        any_frog_fields(&mut g, &mut s);
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(entry))
            .expect("ported");
        rig.check(&format!("update h0+${entry:x}"), case, entry, &s, |c| {
            port(c, slot, 0)
        });
    }
}

/// The bubble's update (template 86, T204): its wobble's step, its climb, and the cells round it
/// marked with each marker its tests name (`$18`, `$e`, `$f`, solid) or none of them.
#[test]
fn bubble_update_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(105);
    let (mut freed, mut kept) = (0, 0);
    for case in 0..N {
        let mut s = Setup::default();
        let (col, row) = any_cell(&mut g);
        let dy = g.below(16) as u16;
        let slot = with_slot(&rig, &mut g, &mut s, template(86), (col, row), dy);
        if slot == 0 {
            continue;
        }
        camera_near(&mut g, &mut s, col * 16, row * 16);
        s.poke_w(slot + 0x60, g.pick(&[0, 1, 2, 3, 0xffff]));
        s.poke_w(slot + 0x5c, (col * 16) as u16 + g.below(16) as u16);
        s.poke_w(slot + 0xe, g.pick(&[0xfffe, 0xfffe, 0, 2, 0xfff0]));
        let none = g.one_in(3);
        for r in row.saturating_sub(2)..(row + 3).min(84) {
            for c in col - 1..=col + 1 {
                let m = if none || g.one_in(3) {
                    g.pick(&[0, 0x39, 9])
                } else {
                    g.pick(&[0x18, 0xe, 0xf, 0x3a, 0x3d, 0x39])
                };
                s.poke_w(cell(c, r), (g.below(840) as u16) << 6 | m);
            }
        }
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(0x24236))
            .expect("ported");
        let mut gone = false;
        if rig.check("update h0+$24236", case, 0x24236, &s, |c| {
            port(c, slot, 0);
            gone = c.bus.l(slot + 0x40) == 0;
        }) {
            if gone { freed += 1 } else { kept += 1 }
        }
    }
    assert!(freed > 300 && kept > 300, "{freed} freed, {kept} kept");
}

/// The frog's fields an object's routine reads.
fn any_frog_fields(g: &mut Gen, s: &mut Setup) {
    s.poke_w(FROG + f::X, 48 + g.below(4600) as u16);
    s.poke_w(FROG + f::Y, 48 + g.below(1200) as u16);
    s.poke_w(FROG + f::VX, (g.below(25) as i16 - 12) as u16);
    s.poke_w(FROG + f::VY, (g.below(30) as i16 - 18) as u16);
    s.poke_w(FROG + f::ON_GROUND, g.below(2) as u16);
    s.poke_w(FROG + f::ON_PLATFORM, g.one_in(6) as u16);
    s.poke_w(FROG + f::AIR, g.pick(&[0, 1, 4, 0x10]));
    s.poke_w(FROG + f::SPUD_ON, g.below(2) as u16);
    s.poke_w(FROG + f::HEAD_BUMP, g.one_in(4) as u16);
    s.poke_w(FROG + f::CROUCH_TIME, g.below(3) as u16);
    s.poke_w(FROG + f::BOUNCE_VY, 0xfff0);
}

#[test]
fn touch_handlers_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(105);
    for case in 0..N {
        let mut s = Setup::default();
        let (t, entry) = g.pick(&[(0u32, 0x239f6u32), (2, 0x23a8e), (4, 0x23970)]);
        let (slot, _) = any_object(&rig, &mut g, &mut s, template(t));
        if slot == 0 {
            continue;
        }
        any_frog_fields(&mut g, &mut s);
        let d7 = g.below(2) as u16;
        s.a[5] = slot;
        s.a[0] = FROG;
        s.d[7] = d7 as u32;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(entry))
            .expect("ported");
        rig.check(&format!("touch h0+${entry:x}"), case, entry, &s, |c| {
            port(c, slot, d7)
        });
    }
}

/// A tile from world `w`'s pickup table (h0+$f382), or now and then any tile.
fn any_pickup_tile(g: &mut Gen, w: u32) -> u16 {
    use sf_core::layout::Ram;
    let m = level();
    let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
    if g.one_in(10) {
        return g.below(840) as u16;
    }
    let table = long(h0(0xf382) + 4 * w);
    let n = (0..)
        .take_while(|k| (long(table + 8 * k) as i32) >= 0)
        .count() as u64;
    long(table + 8 * g.below(n) as u32) as u16
}

#[test]
fn tile_pickups_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(106);
    for case in 0..N {
        let mut s = Setup::default();
        let w = g.below(6) as u32;
        s.poke_l(OBJ_WORLD_OFFSET, w * 0x100);
        let (col, row) = any_cell(&mut g);
        let at = cell(col, row);
        s.poke_w(at, any_pickup_tile(&mut g, w) << 6 | 55);
        s.poke_w(
            at - 0x260,
            any_pickup_tile(&mut g, w) << 6 | g.pick(&[0, 55]),
        );
        for k in 0..4 {
            s.poke_l(
                h0(0xfa68) + 4 * k,
                if g.one_in(3) { cell(col, row + 1) } else { 0 },
            );
        }
        any_frog_fields(&mut g, &mut s);
        s.poke_l(h0(0x834), g.below(200_000) as u32);
        s.poke_w(FROG + f::LIVES, g.below(6) as u16);
        s.poke_w(FROG + f::MAX_SPEED, g.below(14) as u16);
        s.poke_w(FROG + 0xb0, 6);
        s.poke_w(h0(0x9e6), g.below(3) as u16);
        s.poke_b(h0(0x876), g.below(10) as u8);
        s.poke_b(h0(0x877), g.below(10) as u8);
        s.a[0] = FROG;
        s.a[2] = at;
        rig.check("tile_pickup", case, 0xed8a, &s, |c| {
            pickups::tile_pickup(c, at)
        });
    }
}

#[test]
fn switches_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(107);
    for case in 0..N {
        let mut s = Setup::default();
        let (col, row) = any_cell(&mut g);
        let elsewhere = any_cell(&mut g);
        let slot_room = with_slot(&rig, &mut g, &mut s, template(5), elsewhere, 0xc);
        if slot_room != 0 && g.one_in(2) {
            // Make room again (the gate's lever needs a slot), or leave the table as it is.
            s.poke_l(slot_room + 0x40, 0);
        }
        let m = 0x30 + g.below(4) as u16;
        let at = cell(col, row);
        s.poke_w(at, (g.below(840) as u16) << 6 | m);
        let other = g.pick(&[at + 0x260, at - 0x260, at + 2]);
        s.poke_w(other, (g.below(840) as u16) << 6 | m);
        s.poke_w(
            at - 2,
            (g.below(840) as u16) << 6 | g.pick(&[0, 0x3e, 0x3d, 9]),
        );
        for k in 0..4 {
            let gate = if g.one_in(5) {
                0
            } else {
                let (c, r) = any_cell(&mut g);
                cell(c, r.min(70))
            };
            s.poke_l(h0(0x162e4) + 4 * k, gate);
        }
        s.poke_l(OBJ_WORLD_OFFSET, 0x100 * g.below(7) as u32);
        camera_near(&mut g, &mut s, col * 16, row * 16);
        s.a[0] = FROG;
        s.a[2] = at;
        s.d[0] = m as u32;
        rig.check("switch", case, 0xf6be, &s, |c| fixtures::switch(c, at));
    }
}

#[test]
fn obj_run_all_and_collide_all_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(108);
    for case in 0..N {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        // Coins, springs and a lock around the frog.
        let (fx, fy) = (48 + g.below(4500) as u32, 48 + g.below(1100) as u32);
        let n = 1 + g.below(6);
        let mut pre = Vec::new();
        for _ in 0..n {
            let col = (fx / 16 + g.below(5) as u32).saturating_sub(2) + 1;
            let row = (fy / 16 + g.below(6) as u32).saturating_sub(1) + 1;
            pre.push((g.pick(&[0, 0, 2, 4, 5]), col, row));
        }
        // Some made stompable, or not passive (they hurt), or of no contact: what enemies are.
        let flags: Vec<(u16, u16, u16)> = pre
            .iter()
            .map(|_| match g.below(4) {
                0 => (1, 0, g.pick(&[0x1f, 0x7d00, 0xffff])),
                1 => (0, 0, g.pick(&[0x1f, 0x7d00, 0xffff])),
                _ => (0, 1, 0x7d00),
            })
            .collect();
        let pokes = prepared(&rig, &[], |c| {
            for (&(t, col, row), &(stomp, passive, hp)) in pre.iter().zip(&flags) {
                let r = objects::obj_spawn_cell_alt(c, template(t), cell(col, row), 0, 0xd);
                if r.a0 != 0 {
                    c.bus.set_w(r.a0 + 0x2e, stomp);
                    c.bus.set_w(r.a0 + 0x30, passive);
                    c.bus.set_w(r.a0 + 0x1c, hp);
                }
            }
            objects::obj_build_hitboxes(c);
        });
        s.pokes = pokes;
        s.poke_w(FROG + f::X, fx as u16);
        s.poke_w(FROG + f::Y, fy as u16);
        s.poke_w(FROG + f::VY, (g.below(30) as i16 - 18) as u16);
        s.poke_w(FROG + f::ON_GROUND, g.below(2) as u16);
        s.poke_w(FROG + f::HURT, g.one_in(6) as u16);
        s.poke_w(FROG + f::CROUCH, g.one_in(4) as u16);
        s.poke_w(FROG + 0x5a, g.below(4) as u16);
        s.poke_w(h0(0x9e6), g.below(3) as u16);
        // Frame 0's drop-in hides the frog, which leaves it out of every touch.
        s.poke_w(h0(0x9e2), g.one_in(10) as u16);
        s.poke_l(FROG + f::ANIM, g.pick(&[h0(0x10d8e), h0(0x10b8a)]));
        s.poke_w(COINS_LEFT, g.pick(&[0, 1, 2, 30]));
        // The spud over one of them now and then.
        if g.one_in(3) {
            s.poke_w(
                FROG + 0x74,
                (fx as u16)
                    .wrapping_add(g.below(64) as u16)
                    .wrapping_sub(32),
            );
            s.poke_w(FROG + 0x76, (fy as u16).wrapping_add(g.below(64) as u16));
        }
        camera_near(&mut g, &mut s, fx, fy);
        if g.one_in(2) {
            rig.check("collide_all", case, 0xd0cc, &s, objects::collide_all);
        } else {
            rig.check("obj_run_all", case, 0x1b324, &s, objects::obj_run_all);
        }
    }
}

/// `collide_all`'s frog walk (T102 review): a frame the original ended part way through the walk
/// ends at the entry it had got to (step 2 once the frog's box is taken, one more per entry looked
/// at), the entries after it looked at past the vertical blank. Two entries over the frog, a
/// passive one doing nothing, then one the frog lands on: cut at step 3 the bounce is not in the
/// frame's state, at step 4 it is.
#[test]
fn the_frog_walk_ends_its_frame_at_the_entry_the_original_had_got_to() {
    use sf_core::level::beam::{Cut, FrameSpec, Frames};
    let rig = rig();
    let display = sf_conform::display_data();
    for (step, bounced) in [(3, false), (4, true)] {
        let mut st = rig.state().clone();
        let mut paula = Vec::new();
        let mut c = LevelCtx::new(
            Bus::new(&mut st, display.rom()),
            sf_core::level::standard_dispatch(),
            Hw::default(),
            sound(),
            &mut paula,
        );
        let b = &mut c.bus;
        let (x, y) = (1000u16, 500u16);
        b.set_w(FROG + f::X, x);
        b.set_w(FROG + f::Y, y);
        for o in [f::VY, f::HURT, f::CROUCH] {
            b.set_w(FROG + o, 0);
        }
        b.set_w(h0(0x9e2), 0);
        b.set_w(h0(0x9dc), 0);
        for (k, stomp, passive) in [(0, 0, 1), (1, 1, 0)] {
            let (at, slot) = (objects::HITBOXES + 12 * k, OBJ_SLOTS + SLOT * k);
            b.set_w(at, x);
            b.set_w(at + 2, x + 40);
            b.set_w(at + 4, y);
            b.set_w(at + 6, y + 80);
            b.set_l(at + 8, slot);
            b.set_w(slot + 0x2e, stomp);
            b.set_w(slot + 0x30, passive);
            b.set_w(slot + 0xa, y + 0x40);
            b.set_l(slot + 0x10, h0(0x7d4));
        }
        b.set_w(objects::HITBOXES + 24, 0x8000);
        c.enter_phase(5);
        let cut = Cut {
            writes: 0,
            line: 0,
            hpos: 0,
            phase: 5,
            step,
            point: 0,
            blanks: 0,
        };
        c.frames = Some(Box::new(Frames {
            specs: vec![FrameSpec {
                hw: Hw::default(),
                cut: Some(cut),
            }],
            at: Some(0),
            in_pass: true,
            ..Frames::default()
        }));
        objects::collide_all(&mut c);
        let fr = c.frames.take().expect("frames");
        let mid = fr.mid.expect("the frame ended in the walk");
        assert_eq!(mid.w(FROG + f::VY) != 0, bounced, "cut at step {step}");
        assert_ne!(c.bus.w(FROG + f::VY), 0, "the walk runs on past the cut");
    }
}

/// A frame the original ended inside `obj_despawn_if_offscreen` under a slot's update (its step
/// counted done) ends at the point it had got to (`sf_conform::sync::pass_point`; T212: W2-2 1228 at
/// h0+$1b9ce, its last `camera_y` test made after the vertical blank), before the slot's step is
/// done; a cut with no point does not end it there.
#[test]
fn a_frame_ends_inside_the_despawn_test_at_the_point_the_original_had_got_to() {
    use sf_core::level::beam::{Cut, FrameSpec, Frames};
    let rig = rig();
    let display = sf_conform::display_data();
    for point in [0, 1, 3, 5] {
        let mut st = rig.state().clone();
        let mut paula = Vec::new();
        let mut c = LevelCtx::new(
            Bus::new(&mut st, display.rom()),
            sf_core::level::standard_dispatch(),
            Hw::default(),
            sound(),
            &mut paula,
        );
        let slot = OBJ_SLOTS + SLOT * 4;
        let b = &mut c.bus;
        b.set_l(slot + 0x40, 1);
        b.set_l(slot + 0x50, objects::SPUD_BOB);
        b.set_b(SCROLL_DIRS, 0);
        b.set_w(h0(0x9dc), 0);
        c.enter_phase(1);
        c.pass_step = 4;
        let cut = Cut {
            writes: 0,
            line: 0,
            hpos: 0,
            phase: 1,
            step: 5,
            point,
            blanks: 0,
        };
        c.frames = Some(Box::new(Frames {
            specs: vec![FrameSpec {
                hw: Hw::default(),
                cut: Some(cut),
            }],
            at: Some(0),
            in_pass: true,
            ..Frames::default()
        }));
        objects::obj_despawn_if_offscreen(&mut c, slot, 0);
        let fr = c.frames.take().expect("frames");
        assert_eq!(fr.mid.is_some(), point != 0, "point {point}");
    }
}

/// A frame the original ended in `bob_draw`'s draw loop with `k` bobs drawn (step `2 + k`,
/// `sf_conform::sync::bob_draw_step`) ends with the same `k` drawn, going back up the list: the
/// ones after it in the frame's state, the ones before it not yet (T212 review: W4-2 9090 needs
/// only that the frame not end before the swing ball's bob, so its case stays green with the loop
/// unstepped).
#[test]
fn a_frame_ends_in_bob_draw_after_the_bobs_the_original_had_drawn() {
    use sf_core::level::beam::{Cut, FrameSpec, Frames};
    let rig = rig();
    let display = sf_conform::display_data();
    // A bob's animation fields, which only the first part's draw moves.
    let anim = |s: &sf_core::state::State, bob: u32| (s.w(bob + 0x1a), s.l(bob + 0x16), s.l(bob + 0x5c));
    // The list and each bob's fields before the draw and after a whole one.
    let mut st = rig.state().clone();
    let mut paula = Vec::new();
    let (bobs, before, after, writes) = {
        let mut c = LevelCtx::new(
            Bus::new(&mut st, display.rom()),
            sf_core::level::standard_dispatch(),
            Hw::default(),
            sound(),
            &mut paula,
        );
        c.enter_phase(4);
        objects::build_bob_draw_list(&mut c);
        c.bus.set_w(h0(0x9da), 1);
        let mut bobs = Vec::new();
        while c.bus.l(objects::BOB_DRAW_LIST + 4 * bobs.len() as u32) != 0 {
            bobs.push(c.bus.l(objects::BOB_DRAW_LIST + 4 * bobs.len() as u32));
        }
        let before: Vec<_> = bobs.iter().map(|&b| anim(&c.bus.st, b)).collect();
        let mut whole = c.bus.st.clone();
        let mut p2 = Vec::new();
        let mut w = LevelCtx::new(
            Bus::new(&mut whole, display.rom()),
            sf_core::level::standard_dispatch(),
            Hw::default(),
            sound(),
            &mut p2,
        );
        w.enter_phase(5);
        objects::bob_draw(&mut w);
        let after: Vec<_> = bobs.iter().map(|&b| anim(&w.bus.st, b)).collect();
        drop(w);
        // The draw loop writes no register: the original's frame ending in it had made every
        // write before the first part's closing DMACON.
        let writes = p2.iter().position(|p| (p.reg, p.value) == (0x96, 0x0400)).expect("the first part ends") as u16;
        (bobs, before, after, writes)
    };
    let n = bobs.len();
    assert!(n >= 3, "bobs listed: {n}");
    let moved: Vec<usize> = (0..n).filter(|&i| before[i] != after[i]).collect();
    assert!(moved.iter().any(|&i| i + 1 < n), "a bob the draw moves before the last: {moved:?}");
    for k in [0, 1, n / 2, n - 1] {
        let mut s = st.clone();
        let mut paula = Vec::new();
        let mut c = LevelCtx::new(
            Bus::new(&mut s, display.rom()),
            sf_core::level::standard_dispatch(),
            Hw::default(),
            sound(),
            &mut paula,
        );
        c.enter_phase(5);
        c.frames = Some(Box::new(Frames {
            specs: vec![FrameSpec {
                hw: Hw::default(),
                cut: Some(Cut {
                    writes,
                    line: 0,
                    hpos: 0,
                    phase: 5,
                    step: 2 + k as u16,
                    point: 0,
                    blanks: 0,
                }),
            }],
            at: Some(0),
            in_pass: true,
            ..Frames::default()
        }));
        objects::bob_draw(&mut c);
        let fr = c.frames.take().expect("frames");
        let mid = fr.mid.expect("the frame ended in the draw");
        for i in 0..n {
            let drawn = i >= n - k;
            let want = if drawn { after[i] } else { before[i] };
            assert_eq!(anim(&mid, bobs[i]), want, "cut at {k} drawn: bob {i} of {n}");
        }
    }
}

#[test]
fn spud_update_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(109);
    for case in 0..N {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        any_frog_fields(&mut g, &mut s);
        let (x, y) = (48 + g.below(4500) as u16, 48 + g.below(1100) as u16);
        s.poke_w(FROG + f::X, x);
        s.poke_w(FROG + f::Y, y);
        s.poke_w(FROG + f::SPUD_ON, (!g.one_in(6)) as u16);
        s.poke_w(FROG + f::FACING, g.pick(&[0, 1, 0xffff]));
        s.poke_w(FROG + 0x72, g.below(2) as u16);
        s.poke_w(
            FROG + 0x74,
            x.wrapping_add(g.below(300) as u16).wrapping_sub(150),
        );
        s.poke_w(
            FROG + 0x76,
            y.wrapping_add(g.below(200) as u16).wrapping_sub(100),
        );
        s.poke_w(FROG + 0x78, (g.below(40) as i16 - 20) as u16);
        s.poke_w(FROG + 0x7a, (g.below(40) as i16 - 20) as u16);
        s.poke_w(FROG + 0xae, g.pick(&[0, 1, 2, 0xffff]));
        let anim = g.pick(&[h0(0x2d0b0), h0(0x2cf7c), h0(0x2aefe)]);
        s.poke_l(objects::SPUD_BOB + 8, anim);
        s.poke_l(objects::SPUD_BOB + 0x42, g.pick(&[anim, h0(0x2d0b0)]));
        s.poke_w(objects::SPUD_BOB + 0x1a, g.pick(&[0, 3, 0x4e20, 0x4e1f]));
        s.poke_l(
            objects::SPUD_BOB + 0x16,
            g.pick(&[h0(0x2d0b0), h0(0x2cf7c)]),
        );
        s.poke_w(h0(0x9e2), g.one_in(10) as u16);
        camera_near(&mut g, &mut s, x as u32, y as u32);
        let (col, row) = ((s_x(&s) >> 4) as u32, (s_y(&s) >> 4) as u32);
        for r in row.saturating_sub(1)..(row + 2).min(84) {
            for c in col.saturating_sub(1)..(col + 2).min(300) {
                s.poke_w(
                    cell(c, r),
                    (g.below(840) as u16) << 6 | g.pick(&[0, 0x3a, 0x3e, 9]),
                );
            }
        }
        rig.check("spud_update", case, 0x80f0, &s, powerups::spud_update);
    }
}

fn s_x(s: &Setup) -> u16 {
    poked_w(s, FROG + 0x74)
}

fn s_y(s: &Setup) -> u16 {
    poked_w(s, FROG + 0x76)
}

fn poked_w(s: &Setup, at: u32) -> u16 {
    s.pokes
        .iter()
        .rev()
        .find(|(a, b)| *a == at && b.len() == 2)
        .map_or(0, |(_, b)| u16::from_be_bytes([b[0], b[1]]))
}

#[test]
fn frog_hurt_and_time_up_match_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(110);
    for case in 0..N {
        let mut s = Setup::default();
        any_frog_fields(&mut g, &mut s);
        for (o, v) in [
            (f::HURT, g.one_in(4) as u16),
            (f::DYING, g.one_in(6) as u16),
            (f::BLINK, if g.one_in(6) { 5 } else { 0 }),
            (f::SHIELD, g.one_in(8) as u16),
            (0x5a, g.below(5) as u16),
            (f::LIVES, g.below(5) as u16),
            (f::WALL_LEFT, g.one_in(4) as u16),
            (f::WALL_RIGHT, g.one_in(4) as u16),
        ] {
            s.poke_w(FROG + o, v);
        }
        s.poke_l(
            FROG + f::ANIM,
            g.pick(&[h0(0x10d8e), h0(0x10566), h0(0x10b8a), h0(0x10cd6)]),
        );
        let (col, row) = any_cell(&mut g);
        let ground = cell(col, row);
        s.poke_l(FROG + f::GROUND_CELL, ground);
        for o in [0x4bc, 0x4be, 0x4c2, 0x4c4] {
            s.poke_w(
                ground - o,
                (g.below(840) as u16) << 6 | g.pick(&[0, 0x3a, 0x3e, 9]),
            );
        }
        s.poke_w(h0(0x9e6), g.below(3) as u16);
        s.poke_w(h0(0x9dc), g.one_in(10) as u16);
        s.poke_w(h0(0x14), g.one_in(10) as u16);
        s.poke_l(ATTRACT_MODE, if g.one_in(10) { 0xffff_ffff } else { 0 });
        s.poke_l(
            h0(0x876),
            if g.one_in(2) {
                0
            } else {
                g.next() as u32 & 0x0909_0909
            },
        );
        let slot = OBJ_SLOTS;
        if g.one_in(2) {
            s.a[0] = FROG;
            s.a[5] = slot;
            rig.check("frog_hurt", case, 0x1c5bc, &s, |c| {
                death::frog_hurt(c, slot)
            });
        } else {
            rig.check("time_up_check", case, 0xdd6, &s, death::time_up_check);
        }
    }
}

/// `obj_flag_jumptab` (h0+$178ec): 7 worlds × 64 `bra.w` stubs.
const JUMPTAB: u32 = h0(0x178ec);

/// `map_spawn_scan_screen` (h0+$1777c), the level's opening scan: the camera anywhere in W1-1,
/// some slots freed first, and the markers the engine has no spawner for returning at once in
/// the original (`rts; nop` over their stubs, as the `enemies` freeze has them).
#[test]
fn map_spawn_scan_screen_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(110);
    let frozen: Vec<(u32, Vec<u8>)> = sf_conform::freeze::frozen_markers()
        .into_iter()
        .map(|(w, m)| {
            (
                JUMPTAB + 4 * (w as u32 * 64 + m as u32),
                vec![0x4e, 0x75, 0x4e, 0x71],
            )
        })
        .collect();
    for case in 0..300 {
        let mut s = Setup::default();
        let free: Vec<u32> = (0..20).filter(|_| g.one_in(3)).collect();
        s.pokes = prepared(&rig, &[], |c| {
            for &k in &free {
                objects::obj_free(c, OBJ_SLOTS + k * SLOT, 0);
            }
        });
        s.pokes.extend(frozen.iter().cloned());
        s.poke_l(CAMERA_X, g.below(300 * 16 - 320) as u32);
        s.poke_l(CAMERA_Y, g.below(84 * 16 - 192) as u32);
        rig.check(
            "map_spawn_scan_screen",
            case,
            0x1777c,
            &s,
            sf_core::level::spawn::scan_screen,
        );
    }
}

#[test]
fn the_items_register_their_markers_in_every_world() {
    let d = sf_core::level::standard_dispatch();
    for w in 0..6 {
        for m in [8, 9, 29, 45] {
            assert!(d.marker(w, m).is_some(), "world {w} marker {m}");
        }
    }
    assert!(d.tile_pickup.is_some() && d.bump_block.is_some());
    let _ = items::COINS_LEFT;
}

/// For a failing case: the slots as `s` leaves them (`DEBUG_SLOTS=1`).
#[allow(dead_code)]
fn dump(rig: &Rig, s: &Setup) {
    if std::env::var("DEBUG_SLOTS").is_err() {
        return;
    }
    let mut st = rig.state().clone();
    for (a, b) in &s.pokes {
        st.poke(*a, b);
    }
    for k in 0..20 {
        let sl = OBJ_SLOTS + k * SLOT;
        let bob = st.l(sl + 0x50);
        let frame = if bob != 0 { st.l(st.l(bob + 0x16)) } else { 0 };
        eprintln!(
            "slot {k}: cell {:x} upd {:x} x {:x} y {:x} ev {} bob {:x} step {:x} frame {:x} w {:x} h {:x}",
            st.l(sl + 0x40),
            st.l(sl + 4).wrapping_sub(h0(0)),
            st.w(sl + 8),
            st.w(sl + 0xa),
            st.w(sl + 0x2a),
            bob,
            if bob != 0 { st.l(bob + 0x16) } else { 0 },
            frame,
            if frame != 0 { st.w(frame + 4) } else { 0 },
            if frame != 0 { st.w(frame + 6) } else { 0 },
        );
    }
    eprintln!(
        "camera {:x},{:x} dirs {:x} free {:x?}",
        st.l(CAMERA_X),
        st.l(CAMERA_Y),
        st.b(SCROLL_DIRS),
        (0..21)
            .map(|k| st.l(h0(0x1ba16) + 4 * k))
            .collect::<Vec<_>>()
    );
}
