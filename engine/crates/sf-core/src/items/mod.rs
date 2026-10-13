//! Everything in a level that is not an enemy (T15): the coins and the exit lock that waits for
//! them, the hidden coin spring, the pickups drawn in the map's tiles ([`pickups`]), the power-ups
//! they give ([`powerups`]) and the fixtures the frog sets off ([`fixtures`]: pads, launchers,
//! switches). Each kind is the original's template with its routines
//! (docs/09 "Types per world": shared by worlds 1–6), registered into the level's dispatch.

pub mod fixtures;
pub mod pickups;
pub mod powerups;

use crate::frog::FROG;
use crate::layout::h0;
use crate::level::objects::{
    OBJ_TEMPLATES, bob_anim, bob_follow, nothing, obj_despawn_if_offscreen, obj_free,
    obj_spawn_cell, obj_spawn_cell_alt, obj_spawn_coin_forced,
};
use crate::level::{Dispatch, LevelCtx, ObjectKind, n_clear, register};

/// A template's address by its index.
pub const fn template(index: u32) -> u32 {
    OBJ_TEMPLATES + index * 0xae
}

/// `coins_left_for_exit` (h0+$832): the frog's `+$5c`.
pub const COINS_LEFT: u32 = h0(0x832);
/// `coins_collected` (h0+$445e).
pub const COINS_COLLECTED: u32 = h0(0x445e);
/// `score` (h0+$834): the frog's `+$5e`.
pub const SCORE: u32 = h0(0x834);
/// `map_rows` (h0+$17334).
const MAP_ROWS: u32 = h0(0x17334);

/// The coins' animations: spinning (h0+$2ce04), the sparkle when taken (h0+$2ce48).
const COIN_SPIN: u32 = h0(0x2ce04);
const COIN_SPARKLE: u32 = h0(0x2ce48);

/// Template 0: a coin (markers 8 and 9).
pub static COIN: ObjectKind = ObjectKind {
    name: "coin",
    routines: &[
        (h0(0x23f60), coin_init),
        (h0(0x23f86), coin_update),
        (h0(0x239f6), touch_coin),
    ],
};

/// Template 2: a coin thrown out of a hidden coin spring; it falls to the ground.
pub static FALLING_COIN: ObjectKind = ObjectKind {
    name: "falling coin",
    routines: &[
        (h0(0x23f60), coin_init),
        (h0(0x23fae), falling_coin_update),
        (h0(0x23a8e), touch_falling_coin),
    ],
};

/// Template 4: a hidden coin spring (marker 45): invisible; the frog's head or the spud knocks
/// five coins out of it, one at a time.
pub static COIN_SPRING: ObjectKind = ObjectKind {
    name: "hidden coin spring",
    routines: &[
        (h0(0x2406a), coin_spring_init),
        (h0(0x2408a), coin_spring_update),
        (h0(0x23970), touch_coin_spring),
    ],
};

/// Template 5: the exit's lock (marker 29), shown while coins are still needed.
pub static EXIT_LOCK: ObjectKind = ObjectKind {
    name: "exit lock",
    routines: &[
        (h0(0x24036), exit_lock_init),
        (h0(0x2404c), exit_lock_update),
        (h0(0x7d4), nothing),
        (h0(0x29708), nothing),
    ],
};

/// Every item kind and spawner, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 0, &COIN);
    register(d, 2, &FALLING_COIN);
    register(d, 4, &COIN_SPRING);
    register(d, 5, &EXIT_LOCK);
    for world in 0..6 {
        d.register_marker(world, 8, spawn_coin);
        d.register_marker(world, 9, spawn_coin);
        d.register_marker(world, 29, spawn_exit_lock);
        d.register_marker(world, 45, spawn_coin_spring);
    }
    pickups::register(d);
    fixtures::register_all(d);
}

/// Markers 8 and 9 (h0+$17fee): a coin, 13 pixels down its cell.
fn spawn_coin(c: &mut LevelCtx, cell: u32) {
    obj_spawn_coin_forced(c, template(0), cell, 0, 0xd);
}

/// Marker 45 (h0+$18012): a hidden coin spring.
fn spawn_coin_spring(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(4), cell, 0, 0xd);
}

/// Marker 29 (h0+$18024): the exit's lock, while coins are still needed.
fn spawn_exit_lock(c: &mut LevelCtx, cell: u32) {
    if c.bus.w(COINS_LEFT) != 0 {
        obj_spawn_cell(c, template(5), cell, 0, 0xc);
    }
}

/// `coin_init` (h0+$23f60): spinning; not taken (`+$5c` -1); `+$60` 4, `+$68` 0.
fn coin_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, COIN_SPIN);
    b.set_w(slot + 0x5c, 0xffff);
    b.set_w(slot + 0x60, 4);
    b.set_w(slot + 0x68, 0);
}

/// `coin_update` (h0+$23f86): once taken, it sparkles for `$10` passes (`+$5c`) and goes.
fn coin_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x60, 0xffff);
    if n_clear(b.w(slot + 0x5c)) {
        let t = b.w(slot + 0x5c).wrapping_sub(1);
        b.set_w(slot + 0x5c, t);
        if t == 0 {
            return obj_free(c, slot, 0);
        }
    }
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// `touch_coin` (h0+$239f6): the frog takes the coin: its score (`+$1e`), its marker cleared so it
/// stays taken, the sparkle; one coin fewer needed for the exit (with a fanfare on the last).
fn touch_coin(c: &mut LevelCtx, slot: u32, _d7: u16) {
    if n_clear(c.bus.w(slot + 0x5c)) {
        return;
    }
    let b = &mut c.bus;
    let cell = b.l(slot + 0x40);
    b.set_w(cell, b.w(cell) & 0xffc0);
    take_coin(c, slot);
}

/// What taking a coin does (h0+$23a00 / $23aa0 on): the score, the sparkle for `$10` passes, the
/// sound, `coins_collected`, the coins left for the exit and the panel's count.
fn take_coin(c: &mut LevelCtx, slot: u32) {
    let b = &mut c.bus;
    b.set_l(SCORE, b.l(SCORE).wrapping_add(b.w(slot + 0x1e) as u32));
    b.set_w(slot + 0x5c, 0x10);
    bob_anim(b, slot, COIN_SPARKLE);
    c.sfx(0, 3);
    let b = &mut c.bus;
    b.set_l(COINS_COLLECTED, b.l(COINS_COLLECTED).wrapping_add(1));
    if b.w(COINS_LEFT) != 0 {
        let left = b.w(COINS_LEFT).wrapping_sub(1);
        b.set_w(COINS_LEFT, left);
        if left == 0 {
            c.sfx(0x18, 2);
            c.sfx(0x18, 3);
        }
    }
    crate::level::panel::sprite_digits(&mut c.bus);
}

/// h0+$23fae: the falling coin: `+$60` counts down the passes before it can be taken; it falls
/// (speed up to 8) until its centre reaches solid ground (marker `$3e` or more) or it is taken.
fn falling_coin_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if n_clear(b.w(slot + 0x60)) {
        b.set_w(slot + 0x60, b.w(slot + 0x60).wrapping_sub(1));
    }
    if n_clear(b.w(slot + 0x5c)) {
        let t = b.w(slot + 0x5c).wrapping_sub(1);
        b.set_w(slot + 0x5c, t);
        if t == 0 {
            return obj_free(c, slot, 0);
        }
    }
    if b.w(slot + 0xe) != 8 {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    if n_clear(b.w(slot + 0xe)) {
        let col = (b.w(slot + 8).wrapping_add(8) >> 4).wrapping_mul(2);
        let row = (b.w(slot + 0xa).wrapping_add(8) >> 4).wrapping_mul(4);
        let cell = b
            .l(MAP_ROWS.wrapping_add(row as i16 as i32 as u32))
            .wrapping_add(col as u32)
            .wrapping_add(2);
        if n_clear((b.w(cell) & 0x3f).wrapping_sub(0x3e)) {
            b.set_w(slot + 0xe, 0);
        }
    }
    if b.w(slot + 0x68) != 0 {
        b.set_w(slot + 0xe, 0);
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$23a8e: the falling coin is taken once `+$60` has run out: it stops where it is.
fn touch_falling_coin(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if n_clear(b.w(slot + 0x60)) || n_clear(b.w(slot + 0x5c)) {
        return;
    }
    b.set_w(slot + 0xe, 0);
    b.set_w(slot + 0x68, 1);
    take_coin(c, slot);
}

/// h0+$2406a: five coins; invisible (the parked frame).
fn coin_spring_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x5c, 5);
    b.set_w(slot + 0x60, 0);
    bob_anim(b, slot, h0(0x2aefe));
}

/// h0+$2408a: `+$60` counts down the passes until it can be hit again; its bob stays far off.
fn coin_spring_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if b.w(slot + 0x60) != 0 {
        b.set_w(slot + 0x60, b.w(slot + 0x60).wrapping_sub(1));
    }
    let bob = b.l(slot + 0x50);
    b.set_w(bob.wrapping_sub(4), 0x55f0);
    b.set_w(bob.wrapping_sub(2), 0x55f0);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$23970: the frog jumping into it from below (rising faster than 2, off the ground) stops
/// short, or the spud hits it: a coin falls out (every fourth pass at most); after the fifth the
/// spring is gone for good.
fn touch_coin_spring(c: &mut LevelCtx, slot: u32, d7: u16) {
    use crate::frog::f;
    if d7 == 0 {
        let b = &mut c.bus;
        if n_clear(b.w(FROG + f::VY).wrapping_sub(0xfffe)) || b.w(FROG + f::ON_GROUND) != 0 {
            return;
        }
        if !n_clear(b.w(FROG + f::VY)) {
            b.set_w(FROG + f::VY, 0);
        }
        b.set_w(FROG + f::VY, b.w(FROG + f::VY).wrapping_add(1));
    }
    if c.bus.w(slot + 0x60) != 0 {
        return;
    }
    c.bus.set_w(slot + 0x60, 4);
    let cell = c.bus.l(slot + 0x40);
    obj_spawn_cell_alt(c, template(2), cell, 0, 8);
    c.sfx(9, 2);
    let b = &mut c.bus;
    let left = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, left);
    if left != 0 {
        return;
    }
    let cell = b.l(slot + 0x40);
    b.set_w(cell, b.w(cell) & 0xffc0);
    obj_free(c, slot, 0);
}

/// h0+$24036: the lock's picture.
fn exit_lock_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x2d114));
}

/// h0+$2404c: gone once every coin is taken.
fn exit_lock_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    if c.bus.w(COINS_LEFT) == 0 {
        return obj_free(c, slot, 0);
    }
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
