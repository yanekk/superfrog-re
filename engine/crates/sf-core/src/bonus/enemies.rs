//! The bonus level's enemies of markers 1–6 (templates 113–123; T76). Each marker's spawner is
//! `obj_spawn_cell` from the `$600` row of `obj_flag_jumptab` (h0+$178ec); every type's touch
//! handler is `touch_bonus_enemy` (h0+$238e2, T78's). Most drift left at their template's speed
//! and are freed once the camera has left them behind; some fire or burst into others.
//!
//! The paths they follow are tables in the resident executable, read from the bus at their
//! addresses as `items` reads its own.

use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    bob_anim, bob_follow, obj_despawn_if_offscreen, obj_free, obj_free_if_left_behind,
    obj_spawn_at_xy_nodedup, obj_spawn_cell,
};
use crate::level::{Dispatch, LevelCtx, ObjectKind, n_clear, register};

const CAMERA_Y: u32 = h0(0x16660);
/// The bonus level's row of `obj_flag_jumptab` (`obj_world_offset` `$600`).
const BONUS: u8 = 6;

/// `bonus_toggle_114` (h0+$28ad8) and `bonus_toggle_113` (h0+$28b4a): flipped by each spawn of
/// that type; every other one takes the second animation.
pub const TOGGLE_114: u32 = h0(0x28ad8);
pub const TOGGLE_113: u32 = h0(0x28b4a);

/// Type 113's path (h0+$28b8a): y offsets, one each pass (a long apart: the second word of each
/// pair is unused), ending at `$ff80`.
const PATH_113: u32 = h0(0x28b8a);
/// Type 122's bob (h0+$2900a): y offsets a word apart, ending at `$80`.
const PATH_122: u32 = h0(0x2900a);
/// The twelve 116s' speeds (x, y) when a 115 bursts (h0+$28dec).
const BURST_SPEEDS: u32 = h0(0x28dec);
/// The update a 117, 118 or 119 switches to once it stops (h0+$28eaa).
const DRIFT_BACK: u32 = h0(0x28eaa);

/// Template 113 (marker 1): rides a wave from the camera's row.
pub static WAVE: ObjectKind = ObjectKind {
    name: "bonus wave rider",
    routines: &[(h0(0x28af2), wave_init), (h0(0x28b4c), wave_update)],
};

/// Template 114 (marker 2): flies straight left from a random height.
pub static DART: ObjectKind = ObjectKind {
    name: "bonus dart",
    routines: &[(h0(0x28a7a), dart_init), (h0(0x28ada), drift)],
};

/// Template 115 (marker 3): bursts into twelve 116s after a while.
pub static BURSTER: ObjectKind = ObjectKind {
    name: "bonus burster",
    routines: &[(h0(0x28d52), burster_init), (h0(0x28d80), burster_update)],
};

/// Template 116: a burst's shard, slowing down.
pub static SHARD: ObjectKind = ObjectKind {
    name: "bonus shard",
    routines: &[(h0(0x28cd4), shard_init), (h0(0x28cf0), shard_update)],
};

/// Template 117 (marker 4): slows to a stop, splits off a 118 and a 119, and drifts back.
pub static SPLITTER: ObjectKind = ObjectKind {
    name: "bonus splitter",
    routines: &[
        (h0(0x28e3c), splitter_init),
        (h0(0x28e52), splitter_update),
        (DRIFT_BACK, drift_back),
    ],
};

/// Templates 118 and 119: a splitter's halves, going up and down until they stop.
pub static HALF: ObjectKind = ObjectKind {
    name: "bonus splitter half",
    routines: &[
        (h0(0x28e3c), splitter_init),
        (h0(0x28ec6), half_up_update),
        (h0(0x28ef0), half_down_update),
        (DRIFT_BACK, drift_back),
    ],
};

/// Template 120 (marker 5): drops a 121 every 12 passes.
pub static DROPPER: ObjectKind = ObjectKind {
    name: "bonus dropper",
    routines: &[(h0(0x28f1a), dropper_init), (h0(0x28f36), dropper_update)],
};

/// Template 121: a dropper's bomb, falling every other pass.
pub static BOMB: ObjectKind = ObjectKind {
    name: "bonus bomb",
    routines: &[(h0(0x28f7a), bomb_init), (h0(0x28fae), bomb_update)],
};

/// Template 122 (marker 6): bobs and fires four 123s every `$28` passes.
pub static TURRET: ObjectKind = ObjectKind {
    name: "bonus turret",
    routines: &[(h0(0x28fd8), turret_init), (h0(0x29044), turret_update)],
};

/// Template 123: a turret's bullet (also T77's 126's).
pub static BULLET: ObjectKind = ObjectKind {
    name: "bonus bullet",
    routines: &[(h0(0x2916a), bullet_init), (h0(0x29180), bullet_update)],
};

/// Every kind and spawner here, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 113, &WAVE);
    register(d, 114, &DART);
    register(d, 115, &BURSTER);
    register(d, 116, &SHARD);
    register(d, 117, &SPLITTER);
    register(d, 118, &HALF);
    register(d, 119, &HALF);
    register(d, 120, &DROPPER);
    register(d, 121, &BOMB);
    register(d, 122, &TURRET);
    register(d, 123, &BULLET);
    d.register_marker(BONUS, 1, spawn_113);
    d.register_marker(BONUS, 2, spawn_114);
    d.register_marker(BONUS, 3, spawn_115);
    d.register_marker(BONUS, 4, spawn_117);
    d.register_marker(BONUS, 5, spawn_120);
    d.register_marker(BONUS, 6, spawn_122);
}

/// Marker 1 (h0+$18ac8).
pub fn spawn_113(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(113), cell, 0, 0xa);
}

/// Marker 2 (h0+$18ad8).
pub fn spawn_114(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(114), cell, 0, 0xa);
}

/// Marker 3 (h0+$18ae8).
pub fn spawn_115(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(115), cell, 0, 0xa);
}

/// Marker 4 (h0+$18af8).
pub fn spawn_117(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(117), cell, 0, 0xa);
}

/// Marker 5 (h0+$18b08).
pub fn spawn_120(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(120), cell, 0, 0xa);
}

/// Marker 6 (h0+$18b18).
pub fn spawn_122(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(122), cell, 0, 0xc);
}

/// Flip `toggle`; when it comes out set, the second animation.
fn alternate(c: &mut LevelCtx, slot: u32, toggle: u32, second: u32) {
    let b = &mut c.bus;
    let t = !b.w(toggle);
    b.set_w(toggle, t);
    if t != 0 {
        bob_anim(b, slot, second);
    }
}

/// x moves by the speed (`+$c`); the bob follows; freed once left behind.
fn drift(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    bob_follow(b, slot);
    obj_free_if_left_behind(c, slot, 0);
}

/// h0+$28af2: on the camera's row (rounded to a cell, two cells down), its start kept in `+$64`
/// and its path in `+$54` (the start in `+$58`).
fn wave_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let y = b.w(CAMERA_Y + 2).wrapping_add(0x20) & 0xfff0;
    b.set_w(slot + 0xa, y);
    bob_anim(b, slot, h0(0x32f0c));
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x58, PATH_113);
    b.set_l(slot + 0x54, PATH_113);
    alternate(c, slot, TOGGLE_113, h0(0x32f50));
}

/// h0+$28b4c: y is the start's plus `$a` plus the path's next step (the path starts over at its
/// end mark); x drifts.
fn wave_update(c: &mut LevelCtx, slot: u32, d7: u16) {
    let b = &mut c.bus;
    let y0 = b.w(slot + 0x66);
    let mut at = b.l(slot + 0x54);
    if b.w(at) == 0xff80 {
        at = b.l(slot + 0x58);
        b.set_l(slot + 0x54, at);
    }
    let y = b.w(at).wrapping_add(y0.wrapping_add(0xa));
    b.set_w(slot + 0xa, y);
    b.set_l(slot + 0x54, b.l(slot + 0x54).wrapping_add(4));
    drift(c, slot, d7);
}

/// h0+$28a7a: up to `$e0` lower, at a random speed −(4..10).
fn dart_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32f94));
    let dy = c.rng_range(0xe0);
    c.bus.set_w(slot + 0xa, c.bus.w(slot + 0xa).wrapping_add(dy));
    let v = c.rng_range(6).wrapping_add(4);
    c.bus.set_w(slot + 0xc, v.wrapping_neg());
    alternate(c, slot, TOGGLE_114, h0(0x32fb0));
}

/// h0+$28d52: bursts after 30..60 passes (`+$5c`).
fn burster_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32fcc));
    let t = c.rng_range(0x1e).wrapping_add(0x1e);
    c.bus.set_w(slot + 0x5c, t);
}

/// h0+$28d80: drifts until its time runs out, then twelve 116s spawn `$10` right of and below it,
/// with the burst's speeds, and it is gone (at once if a spawn finds no room).
fn burster_update(c: &mut LevelCtx, slot: u32, d7: u16) {
    let t = c.bus.w(slot + 0x5c).wrapping_sub(1);
    c.bus.set_w(slot + 0x5c, t);
    if n_clear(t) {
        return drift(c, slot, d7);
    }
    for k in 0..12 {
        let x = c.bus.w(slot + 8).wrapping_add(0x10);
        let y = c.bus.w(slot + 0xa).wrapping_add(0x10);
        let r = obj_spawn_at_xy_nodedup(c, template(116), x, y);
        if r.d0 != 0 {
            return obj_free(c, slot, 0);
        }
        let b = &mut c.bus;
        b.set_w(r.a0 + 0xc, b.w(BURST_SPEEDS + 4 * k));
        b.set_w(r.a0 + 0xe, b.w(BURST_SPEEDS + 4 * k + 2));
    }
    obj_free(c, slot, 0)
}

/// h0+$28cd4: its speeds next decay in 4 passes.
fn shard_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32ff0));
    c.bus.set_w(slot + 0x5c, 3);
}

/// One step of a speed toward 0.
fn decay(v: u16) -> u16 {
    match v {
        0 => 0,
        v if !n_clear(v) => v.wrapping_add(1),
        v => v - 1,
    }
}

/// h0+$28cf0: every 4th pass both speeds step toward 0; it moves by them, and is freed once its
/// animation has run out (its bob's `+$1a` at `$7530`).
fn shard_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if !n_clear(t) {
        b.set_w(slot + 0xc, decay(b.w(slot + 0xc)));
        b.set_w(slot + 0xe, decay(b.w(slot + 0xe)));
        b.set_w(slot + 0x5c, 3);
    }
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let bob = b.l(slot + 0x50);
    if n_clear(b.w(bob + 0x1a).wrapping_sub(0x7530)) {
        return obj_free(c, slot, 0);
    }
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$28e3c (117, 118 and 119).
fn splitter_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x3300c));
}

/// h0+$28e52: its speed rises by one each pass; when it reaches 0 a 118 and a 119 spawn where
/// it is and it drifts back ([`DRIFT_BACK`]).
fn splitter_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let v = b.w(slot + 0xc).wrapping_add(1);
    b.set_w(slot + 0xc, v);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(v));
    if v == 0 {
        let (x, y) = (c.bus.w(slot + 8), c.bus.w(slot + 0xa));
        obj_spawn_at_xy_nodedup(c, template(118), x, y);
        let (x, y) = (c.bus.w(slot + 8), c.bus.w(slot + 0xa));
        obj_spawn_at_xy_nodedup(c, template(119), x, y);
        c.bus.set_l(slot + 4, DRIFT_BACK);
    }
    bob_follow(&mut c.bus, slot);
    obj_free_if_left_behind(c, slot, 0);
}

/// h0+$28eaa: speeding up to the left, one more each pass.
fn drift_back(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let v = b.w(slot + 0xc).wrapping_sub(1);
    b.set_w(slot + 0xc, v);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(v));
    bob_follow(b, slot);
    obj_free_if_left_behind(c, slot, 0);
}

/// h0+$28ec6 / h0+$28ef0: y moves by `+$e`, which steps by `step` until it reaches 0; then it
/// drifts back ([`DRIFT_BACK`]).
fn half_update(c: &mut LevelCtx, slot: u32, step: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let v = b.w(slot + 0xe).wrapping_add(step);
    b.set_w(slot + 0xe, v);
    if v == 0 {
        b.set_l(slot + 4, DRIFT_BACK);
    }
    bob_follow(b, slot);
    obj_free_if_left_behind(c, slot, 0);
}

fn half_up_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    half_update(c, slot, 1);
}

fn half_down_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    half_update(c, slot, 0xffff);
}

/// h0+$28f1a: its first bomb in 13 passes.
fn dropper_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x33018));
    c.bus.set_w(slot + 0x5c, 0xc);
}

/// h0+$28f36: every 13th pass a 121 spawns `$20` right of and `$1c` below it; it drifts.
fn dropper_update(c: &mut LevelCtx, slot: u32, d7: u16) {
    let t = c.bus.w(slot + 0x5c).wrapping_sub(1);
    c.bus.set_w(slot + 0x5c, t);
    if !n_clear(t) {
        c.bus.set_w(slot + 0x5c, 0xc);
        let x = c.bus.w(slot + 8).wrapping_add(0x20);
        let y = c.bus.w(slot + 0xa).wrapping_add(0x1c);
        obj_spawn_at_xy_nodedup(c, template(121), x, y);
    }
    drift(c, slot, d7)
}

/// h0+$28f7a: 4 pixels up, at a random x speed −3..3.
fn bomb_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x33024));
    c.bus.set_w(slot + 0x5c, 0);
    let v = c.rng_range(6).wrapping_sub(3);
    let b = &mut c.bus;
    b.set_w(slot + 0xc, v);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_sub(4));
}

/// h0+$28fae: it falls faster every other pass (`+$5c` flips).
fn bomb_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    let t = !b.w(slot + 0x5c);
    b.set_w(slot + 0x5c, t);
    if t != 0 {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$28fd8: its bob in `+$64` (the start in `+$68`) about its spawn row (`+$5c`); it first fires
/// in `$20` passes (`+$60`).
fn turret_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x33030));
    b.set_l(slot + 0x64, PATH_122);
    b.set_l(slot + 0x68, PATH_122);
    b.set_w(slot + 0x5c, b.w(slot + 0xa));
    b.set_w(slot + 0x60, 0x20);
}

/// Where a turret fires its four bullets from (dx, dy) and their speeds (x, y): up, right, down,
/// left (h0+$29072..$29150).
const SHOTS_122: [(u16, u16, u16, u16); 4] = [
    (8, 0, 0, 0xfff8),
    (0x18, 0x10, 8, 0),
    (8, 0x20, 0, 8),
    (0xfff8, 0x10, 0xfff8, 0),
];

/// h0+$29044: y is the spawn row plus the bob's next step (the bob starts over at its end mark);
/// every `$28` passes four 123s fly off; x drifts.
fn turret_update(c: &mut LevelCtx, slot: u32, d7: u16) {
    let b = &mut c.bus;
    let mut at = b.l(slot + 0x64);
    if b.w(at) == 0x80 {
        at = b.l(slot + 0x68);
        b.set_l(slot + 0x64, at);
    }
    b.set_l(slot + 0x64, b.l(slot + 0x64).wrapping_add(2));
    b.set_w(slot + 0xa, b.w(slot + 0x5c).wrapping_add(b.w(at)));
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if t == 0 {
        b.set_w(slot + 0x60, 0x28);
        for (dx, dy, vx, vy) in SHOTS_122 {
            let x = c.bus.w(slot + 8).wrapping_add(dx);
            let y = c.bus.w(slot + 0xa).wrapping_add(dy);
            let r = obj_spawn_at_xy_nodedup(c, template(123), x, y);
            if r.d0 == 0 && r.a0 != 0 {
                c.bus.set_w(r.a0 + 0xc, vx);
                c.bus.set_w(r.a0 + 0xe, vy);
            }
        }
    }
    drift(c, slot, d7)
}

/// h0+$2916a.
fn bullet_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x3303c));
}

/// h0+$29180: it moves by its speeds.
fn bullet_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
