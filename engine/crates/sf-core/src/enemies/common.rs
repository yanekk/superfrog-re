//! What enemies of every world share (docs/09 "Touch handlers"): the touch handlers that kill an
//! enemy in one hit or by the spud's damage, or hurt the frog, the updates a killed enemy turns to,
//! and the small helpers every update ends with. Created by T43 (world 2); the other enemy tasks
//! (T39–T42, T44) extend it.

use crate::frog::{FROG, f};
use crate::items::SCORE;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_free};
use crate::level::{LevelCtx, n_clear};
use crate::mem::Bus;

/// `map_rows` (h0+$17334).
pub const MAP_ROWS: u32 = h0(0x17334);
/// `touch_kill_1hit` (h0+$23582).
pub const TOUCH_KILL_1HIT: u32 = h0(0x23582);
/// `touch_spud_damage` (h0+$2360e).
pub const TOUCH_SPUD_DAMAGE: u32 = h0(0x2360e);
/// `touch_hurt` (h0+$236ac).
pub const TOUCH_HURT: u32 = h0(0x236ac);
/// h0+$23b34: a one-hit kill falls until it lands.
pub const KILLED_FALL: u32 = h0(0x23b34);
/// h0+$23bee / h0+$23662: an enemy the spud killed lies stunned, then a stomp makes it a reward.
pub const STUNNED: u32 = h0(0x23bee);
pub const TOUCH_STUNNED: u32 = h0(0x23662);
/// The touch handler of scenery: nothing (h0+$7d4).
pub const NOTHING: u32 = h0(0x7d4);
/// The fallen enemy's animation (h0+$2cef4) and the frame it ends on (h0+$2af0a).
const FALLEN: u32 = h0(0x2cef4);
const FALLEN_END: u32 = h0(0x2af0a);

/// The routines here, by the original's address.
pub const ROUTINES: &[(u32, crate::level::SlotFn)] = &[
    (TOUCH_KILL_1HIT, touch_kill_1hit),
    (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    (TOUCH_HURT, touch_hurt),
    (KILLED_FALL, killed_fall),
    (STUNNED, stunned),
    (TOUCH_STUNNED, touch_stunned),
];

/// The marker of map cell `cell` (its low six bits).
pub fn marker_at(b: &Bus, cell: u32) -> u16 {
    b.w(cell) & 0x3f
}

/// The map cell under level pixel `x`, `y` as the enemies find it: `map_rows[(y >> 4) * 4]` plus
/// `(x >> 4) * 2` (a word index, sign-extended as `(a1,d1.w)` reads it).
pub fn cell_at(b: &Bus, x: u16, y: u16) -> u32 {
    let row = (y >> 4).wrapping_mul(4);
    let col = (x >> 4).wrapping_mul(2);
    b.l(MAP_ROWS.wrapping_add(row as i16 as u32))
        .wrapping_add(col as u32)
}

/// The end of most updates: the bob takes the slot's position, then the slot is freed once it is
/// off the screen.
pub fn follow_and_despawn(c: &mut LevelCtx, slot: u32) {
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// `touch_kill_1hit` (h0+$23582): a stomp or the spud takes `$20` hit points; below none the
/// enemy falls ([`killed_fall`]) with no touch, its bob's step counter at `$7d00`, its marker
/// cleared so it never comes back, and sfx 7 or 8 (the RNG's) on channel 3.
pub fn touch_kill_1hit(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let hp = b.w(slot + 0x1c).wrapping_sub(0x20);
    b.set_w(slot + 0x1c, hp);
    if n_clear(hp) {
        return;
    }
    b.set_l(slot + 4, KILLED_FALL);
    b.set_l(slot + 0x10, NOTHING);
    let bob = b.l(slot + 0x50);
    b.set_w(bob + 0x1a, 0x7d00);
    let cell = b.l(slot + 0x40);
    b.set_w(cell, b.w(cell) & 0xffc0);
    let effect = c.rng_range(1).wrapping_add(7);
    c.sfx(effect as u8, 3);
}

/// h0+$23b34: a one-hit kill falls, one pixel a pass faster up to 8; evictable, passive. Once the
/// cell under its middle is solid (marker `$3a` or above) it shows the fallen animation; when that
/// has reached its last frame the enemy is gone for 200 points, else it hops up a pixel's speed.
pub fn killed_fall(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let mut vy = b.w(slot + 0xe).wrapping_add(1);
    if n_clear(vy.wrapping_sub(8)) {
        vy = 8;
    }
    b.set_w(slot + 0xe, vy);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(vy));
    bob_follow(b, slot);
    b.set_w(slot + 0x2a, 1);
    b.set_w(slot + 0x2c, 0);
    b.set_w(slot + 0x2e, 0);
    b.set_w(slot + 0x30, 1);
    let x = b.w(slot + 8).wrapping_add(8);
    let y = b.w(slot + 0xa).wrapping_add(8);
    let cell = cell_at(b, x, y).wrapping_add(2);
    if !n_clear(marker_at(b, cell).wrapping_sub(0x3a)) {
        return;
    }
    let bob = b.l(slot + 0x50);
    if b.l(bob + 8) != FALLEN {
        b.set_l(bob + 0x28, FALLEN);
        b.set_l(bob + 0x42, FALLEN);
    }
    if b.l(b.l(bob + 0x16)) != FALLEN_END {
        b.set_w(slot + 0xe, 0xffff);
        return;
    }
    b.set_l(SCORE, b.l(SCORE).wrapping_add(0xc8));
    obj_free(c, slot, 0);
}

/// `touch_spud_damage` (h0+$2360e): the spud or a stomp takes `$20` hit points; below none the
/// enemy is stunned for four passes ([`stunned`]), shows its death animation (`+$18`), its marker
/// is cleared, and sfx 6 plays on channel 2.
pub fn touch_spud_damage(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let hp = b.w(slot + 0x1c).wrapping_sub(0x20);
    b.set_w(slot + 0x1c, hp);
    if n_clear(hp) {
        return;
    }
    b.set_w(slot + 0x5c, 4);
    b.set_w(slot + 0x30, 1);
    b.set_l(slot + 4, STUNNED);
    b.set_l(slot + 0x10, TOUCH_STUNNED);
    let anim = b.l(slot + 0x18);
    bob_anim(b, slot, anim);
    let cell = b.l(slot + 0x40);
    b.set_w(cell, b.w(cell) & 0xffc0);
    c.sfx(6, 2);
}

/// h0+$23bee: a stunned enemy counts its four passes down (to -1), can be stomped, and stays
/// where it is until it leaves the screen.
pub fn stunned(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x5c);
    if n_clear(t) {
        b.set_w(slot + 0x5c, t.wrapping_sub(1));
    }
    b.set_w(slot + 0x2c, 0);
    b.set_w(slot + 0x2e, 1);
    follow_and_despawn(c, slot);
}

/// h0+$23662: a stunned enemy stomped once its passes are over: sfx 6 on channel 2, three times
/// in ten a croak (sample 2), and it becomes a reward 8 pixels right.
pub fn touch_stunned(c: &mut LevelCtx, slot: u32, _d7: u16) {
    if n_clear(c.bus.w(slot + 0x5c)) {
        return;
    }
    c.sfx(6, 2);
    if !n_clear(c.rng_range(0x64).wrapping_sub(0x1e)) {
        c.sample(2);
    }
    crate::bonus::death::become_reward(c, slot, 8);
}

/// `touch_hurt` (h0+$236ac): the frog is hurt.
pub fn touch_hurt(c: &mut LevelCtx, slot: u32, _d7: u16) {
    crate::level::death::frog_hurt(c, slot);
}

/// Whether the frog is falling or standing (`frog_vy` not negative).
pub fn frog_not_rising(b: &Bus) -> bool {
    n_clear(b.w(FROG + f::VY))
}
