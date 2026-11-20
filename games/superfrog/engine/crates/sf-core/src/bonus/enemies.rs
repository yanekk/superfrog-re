//! The bonus level's enemies of markers 1–6 (templates 113–123; T76) and 7–15 (templates 124–133;
//! T77), and marker 55, the level's end. Each marker's spawner is
//! `obj_spawn_cell` from the `$600` row of `obj_flag_jumptab` (h0+$178ec); every type's touch
//! handler is `touch_bonus_enemy` (h0+$238e2, T78's). Most drift left at their template's speed
//! and are freed once the camera has left them behind; some fire or burst into others.
//!
//! The paths they follow are tables in the resident executable, read from the bus at their
//! addresses as `items` reads its own.

use crate::items::template;
use crate::layout::h0;
use crate::level::collision::LEVEL_STATE;
use crate::level::objects::{
    bob_anim, bob_follow, obj_despawn_if_offscreen, obj_free, obj_free_if_left_behind,
    obj_path_xy, obj_spawn_at_xy_nodedup, obj_spawn_cell,
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
    register(d, 124, &SWOOPER);
    register(d, 125, &GLIDER);
    register(d, 126, &GUNNER);
    register(d, 127, &CHAIN_HEAD);
    register(d, 128, &CHAIN_TAIL);
    register(d, 129, &WAVER);
    register(d, 130, &WAVER_B);
    register(d, 131, &SPRAYER);
    register(d, 132, &SPRAY);
    register(d, 133, &TRACER);
    d.register_marker(BONUS, 7, spawn_124);
    d.register_marker(BONUS, 8, spawn_125);
    d.register_marker(BONUS, 9, spawn_126);
    d.register_marker(BONUS, 10, spawn_127);
    d.register_marker(BONUS, 11, fly_off);
    d.register_marker(BONUS, 12, spawn_129);
    d.register_marker(BONUS, 13, spawn_130);
    d.register_marker(BONUS, 14, spawn_131);
    d.register_marker(BONUS, 15, spawn_133);
    d.register_marker(BONUS, 55, level_end);
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

// Markers 7–15 and 55 (T77).

/// `bonus_fly_off` (h0+$293ce): set by marker 11; while set, 127s and 128s fly off left.
pub const FLY_OFF: u32 = h0(0x293ce);

/// Type 124's dive (h0+$291e2): y offsets a word apart, taken negated, ending at `$80`.
const PATH_124: u32 = h0(0x291e2);
/// The wave 126, 129, 130 and 133 ride (h0+$3b48a): longs, the second word of each a y offset
/// for 126 (in 4-byte steps) and 129/130 (in 8-byte steps, less `$80`), both starting over at a
/// negative word; for 133 a path of positions ([`obj_path_xy`]).
const PATH_WAVE: u32 = h0(0x3b48a);
/// The chain 127 and 128 follow (h0+$3b63e, 128 from h0+$3b64e): positions, ending at `$fffe`.
const PATH_CHAIN: u32 = h0(0x3b63e);
const PATH_CHAIN_TAIL: u32 = h0(0x3b64e);
/// Type 131's bob (h0+$29536): y offsets a word apart, ending at `$80`.
const PATH_131: u32 = h0(0x29536);
/// The update a 127 or 128 switches to once [`FLY_OFF`] is set (h0+$29418).
const FLEE: u32 = h0(0x29418);

/// Template 124 (marker 7): dives and climbs on the spot.
pub static SWOOPER: ObjectKind = ObjectKind {
    name: "bonus swooper",
    routines: &[(h0(0x291a0), swooper_init), (h0(0x2921c), swooper_update)],
};

/// Template 125 (marker 8): flies at its template's speeds.
pub static GLIDER: ObjectKind = ObjectKind {
    name: "bonus glider",
    routines: &[(h0(0x2924a), glider_init), (h0(0x29280), glider_update)],
};

/// Template 126 (marker 9): rides the wave and fires a 123 left every `$20` passes.
pub static GUNNER: ObjectKind = ObjectKind {
    name: "bonus gunner",
    routines: &[(h0(0x292a0), gunner_init), (h0(0x292d2), gunner_update)],
};

/// Template 127 (marker 10): the head of a chain following a path.
pub static CHAIN_HEAD: ObjectKind = ObjectKind {
    name: "bonus chain head",
    routines: &[(h0(0x29348), chain_head_init), (h0(0x293d0), chain_update), (FLEE, flee)],
};

/// Template 128 (marker 10, a row below): the chain's tail, a step behind.
pub static CHAIN_TAIL: ObjectKind = ObjectKind {
    name: "bonus chain tail",
    routines: &[(h0(0x29388), chain_tail_init), (h0(0x293d0), chain_update), (FLEE, flee)],
};

/// Template 129 (marker 12): rides the wave at double step.
pub static WAVER: ObjectKind = ObjectKind {
    name: "bonus waver",
    routines: &[(h0(0x29434), waver_129_init), (h0(0x29460), waver_update)],
};

/// Template 130 (marker 13): 129 with another look.
pub static WAVER_B: ObjectKind = ObjectKind {
    name: "bonus waver (b)",
    routines: &[(h0(0x2949c), waver_130_init), (h0(0x294c8), waver_update)],
};

/// Template 131 (marker 14): bobs and sprays three 132s every `$28` passes.
pub static SPRAYER: ObjectKind = ObjectKind {
    name: "bonus sprayer",
    routines: &[(h0(0x29504), sprayer_init), (h0(0x29570), sprayer_update)],
};

/// Template 132: a sprayer's shot.
pub static SPRAY: ObjectKind = ObjectKind {
    name: "bonus spray",
    routines: &[(h0(0x29666), spray_init), (h0(0x2967c), spray_update)],
};

/// Template 133 (marker 15): traces the wave's positions about its spawn point.
pub static TRACER: ObjectKind = ObjectKind {
    name: "bonus tracer",
    routines: &[(h0(0x2969c), tracer_init), (h0(0x296ea), tracer_update)],
};

/// Marker 7 (h0+$18b28).
pub fn spawn_124(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(124), cell, 0, 0xc);
}

/// Marker 8 (h0+$18b38).
pub fn spawn_125(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(125), cell, 0, 0xc);
}

/// Marker 9 (h0+$18b48).
pub fn spawn_126(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(126), cell, 0, 0xc);
}

/// Marker 10 (h0+$18b58): a 127 at the cell and a 128 in the cell below.
pub fn spawn_127(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(127), cell, 0, 0xc);
    obj_spawn_cell(c, template(128), cell.wrapping_add(0x260), 0, 0xc);
}

/// Marker 11 (h0+$18b80): the chains fly off.
pub fn fly_off(c: &mut LevelCtx, _cell: u32) {
    c.bus.set_w(FLY_OFF, 1);
}

/// Marker 12 (h0+$18b8a).
pub fn spawn_129(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(129), cell, 0, 0xc);
}

/// Marker 13 (h0+$18b9a).
pub fn spawn_130(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(130), cell, 0, 0xc);
}

/// Marker 14 (h0+$18baa).
pub fn spawn_131(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(131), cell, 0, 0xc);
}

/// Marker 15 (h0+$18bba).
pub fn spawn_133(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(133), cell, 0, 0xc);
}

/// Marker 55 (h0+$18abc): the level is over (`level_state` 2).
pub fn level_end(c: &mut LevelCtx, _cell: u32) {
    c.bus.set_l(LEVEL_STATE, 2);
}

/// A path of words at `+$64` (its start in `+$68`), starting over at `restart`: the step to
/// take, the pointer moved on by `stride`.
fn path_step(b: &mut crate::mem::Bus, slot: u32, restart: impl Fn(u16) -> bool, stride: u32) -> u32 {
    let mut at = b.l(slot + 0x64);
    if restart(b.w(at)) {
        at = b.l(slot + 0x68);
        b.set_l(slot + 0x64, at);
    }
    b.set_l(slot + 0x64, b.l(slot + 0x64).wrapping_add(stride));
    at
}

/// x moves by the speed (`+$c`); the bob follows; freed once left behind.
fn drift_x(c: &mut LevelCtx, slot: u32) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    bob_follow(b, slot);
    obj_free_if_left_behind(c, slot, 0);
}

/// Both speeds move it; the bob follows.
fn fly(b: &mut crate::mem::Bus, slot: u32) {
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    bob_follow(b, slot);
}

/// A slot's path (`+$64`) and its start (`+$68`) at `path`.
fn set_path(b: &mut crate::mem::Bus, slot: u32, path: u32) {
    b.set_l(slot + 0x64, path);
    b.set_l(slot + 0x68, path);
}

/// h0+$291a0: a cell lower, up to `$110` further left, on its dive.
fn swooper_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(0x10));
    bob_anim(b, slot, h0(0x33048));
    set_path(b, slot, PATH_124);
    let dx = c.rng_range(0x110);
    c.bus.set_w(slot + 8, c.bus.w(slot + 8).wrapping_sub(dx));
}

/// h0+$2921c: y moves up by the dive's next step; it does not move across.
fn swooper_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let at = path_step(b, slot, |w| w == 0x80, 2);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_sub(b.w(at)));
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$2924a: a cell higher, from `$90` right to `$9c` left of the cell.
fn glider_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x33074));
    let dx = 0x90u16.wrapping_sub(c.rng_range(0x12c));
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(dx));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_sub(0x10));
}

/// h0+$29280.
fn glider_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    fly(&mut c.bus, slot);
    obj_free_if_left_behind(c, slot, 0);
}

/// h0+$292a0: on the wave about its spawn row (`+$5c`); it first fires in `$20` passes.
fn gunner_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x330f8));
    set_path(b, slot, PATH_WAVE);
    b.set_w(slot + 0x5c, b.w(slot + 0xa));
    b.set_w(slot + 0x60, 0x20);
}

/// h0+$292d2: y is the spawn row plus the wave's next step; every `$20` passes a 123 flies off
/// left from `$10` below it; x drifts.
fn gunner_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let at = path_step(b, slot, |w| !n_clear(w), 4);
    b.set_w(slot + 0xa, b.w(slot + 0x5c).wrapping_add(b.w(at + 2)));
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if t == 0 {
        b.set_w(slot + 0x60, 0x20);
        let (x, y) = (b.w(slot + 8), b.w(slot + 0xa).wrapping_add(0x10));
        let r = obj_spawn_at_xy_nodedup(c, template(123), x, y);
        if r.d0 == 0 && r.a0 != 0 {
            c.bus.set_w(r.a0 + 0xc, 0xfff8);
            c.bus.set_w(r.a0 + 0xe, 0);
        }
    }
    drift_x(c, slot);
}

/// The chain's start: its path from `path` (the start in `+$58`), its spawn x (`+$5c`) and
/// a row from the camera's (`+$60`); [`FLY_OFF`] cleared.
fn chain_init(b: &mut crate::mem::Bus, slot: u32, path: u32) {
    b.set_l(slot + 0x58, PATH_CHAIN);
    b.set_l(slot + 0x54, path);
    b.set_w(slot + 0x5c, b.w(slot + 8));
    b.set_w(slot + 0x60, b.w(CAMERA_Y + 2).wrapping_add(0x10));
    b.set_w(FLY_OFF, 0);
}

/// h0+$29348.
fn chain_head_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x3310c));
    chain_init(b, slot, PATH_CHAIN);
}

/// h0+$29388: a cell up (back where the head is), a step along the path, its first look its own.
fn chain_tail_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_sub(0x10));
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, h0(0x33114));
    b.set_l(bob + 0x42, h0(0x3310c));
    chain_init(b, slot, PATH_CHAIN_TAIL);
}

/// h0+$293d0: its spawn x creeps right by 2; it takes the path's next position about it (`$140`
/// left) and the row; freed when the path is spent (`$fffe`); once [`FLY_OFF`] is set it stops
/// and flies off ([`FLEE`]).
fn chain_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let x0 = b.w(slot + 0x5c).wrapping_add(2);
    b.set_w(slot + 0x5c, x0);
    obj_path_xy(b, slot, x0.wrapping_sub(0x140), b.w(slot + 0x60));
    if b.w(b.l(slot + 0x54)) == 0xfffe {
        return obj_free(c, slot, 0);
    }
    if b.w(FLY_OFF) != 0 {
        b.set_l(slot + 4, FLEE);
        b.set_w(slot + 0xc, 0);
    }
    bob_follow(b, slot);
}

/// h0+$29418: speeding up to the left, one more each pass.
fn flee(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let v = b.w(slot + 0xc).wrapping_sub(1);
    b.set_w(slot + 0xc, v);
    drift_x(c, slot);
}

/// h0+$29434 / h0+$2949c: on the wave about its spawn row (`+$5c`).
fn waver_init(c: &mut LevelCtx, slot: u32, anim: u32) {
    let b = &mut c.bus;
    bob_anim(b, slot, anim);
    set_path(b, slot, PATH_WAVE);
    b.set_w(slot + 0x5c, b.w(slot + 0xa));
}

fn waver_129_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    waver_init(c, slot, h0(0x33168));
}

fn waver_130_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    waver_init(c, slot, h0(0x33174));
}

/// h0+$29460 / h0+$294c8: y is the spawn row plus every other step of the wave, less `$80`;
/// x drifts.
fn waver_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let at = path_step(b, slot, |w| !n_clear(w), 8);
    let y = b.w(slot + 0x5c).wrapping_add(b.w(at + 2)).wrapping_sub(0x80);
    b.set_w(slot + 0xa, y);
    drift_x(c, slot);
}

/// h0+$29504: its bob about its spawn row (`+$5c`); it first sprays in `$20` passes.
fn sprayer_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x33180));
    set_path(b, slot, PATH_131);
    b.set_w(slot + 0x5c, b.w(slot + 0xa));
    b.set_w(slot + 0x60, 0x20);
}

/// A sprayer's three shots' speeds (x, y): up, right, left (h0+$295d0..$29646).
const SHOTS_131: [(u16, u16); 3] = [(0, 0xfff4), (0xc, 0), (0xfff4, 0)];

/// h0+$29570: y is the spawn row plus the bob's next step; every `$28` passes its animation
/// starts over and three 132s fly off from where it is; x drifts.
fn sprayer_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let at = path_step(b, slot, |w| w == 0x80, 2);
    b.set_w(slot + 0xa, b.w(slot + 0x5c).wrapping_add(b.w(at)));
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if t == 0 {
        b.set_w(slot + 0x60, 0x28);
        let bob = b.l(slot + 0x50);
        b.set_w(bob + 0x1a, 0);
        for (vx, vy) in SHOTS_131 {
            let (x, y) = (c.bus.w(slot + 8), c.bus.w(slot + 0xa));
            let r = obj_spawn_at_xy_nodedup(c, template(132), x, y);
            if r.d0 == 0 && r.a0 != 0 {
                c.bus.set_w(r.a0 + 0xc, vx);
                c.bus.set_w(r.a0 + 0xe, vy);
            }
        }
    }
    drift_x(c, slot);
}

/// h0+$29666.
fn spray_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x33194));
}

/// h0+$2967c: it moves by its speeds.
fn spray_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    fly(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$2969c: the wave's positions about its spawn point (`+$5c`, `+$60`), the row moved
/// −`$10`..`$70`.
fn tracer_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, h0(0x331d8));
    b.set_l(bob + 0x42, h0(0x331b0));
    b.set_l(slot + 0x58, PATH_WAVE);
    b.set_l(slot + 0x54, PATH_WAVE);
    b.set_w(slot + 0x5c, b.w(slot + 8));
    b.set_w(slot + 0x60, b.w(slot + 0xa));
    let dy = c.rng_range(0x80).wrapping_sub(0x40).wrapping_add(0x30);
    c.bus.set_w(slot + 0x60, c.bus.w(slot + 0x60).wrapping_add(dy));
}

/// h0+$296ea.
fn tracer_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    obj_path_xy(b, slot, b.w(slot + 0x5c), b.w(slot + 0x60));
    bob_follow(b, slot);
    obj_free_if_left_behind(c, slot, 0);
}
