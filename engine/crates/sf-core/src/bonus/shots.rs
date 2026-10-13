//! The frog's shots in shooter mode (h0+$7e3e…$80f0). Five shots live for the whole bonus level:
//! three in a cycle the fire button sends off one at a time from the frog, and two extra ones,
//! sent together once the power (`shot_power`, raised by the bonus level's rewards) is on. A shot
//! not in flight waits, parked off the right of the screen. The spud is not thrown here: in
//! shooter mode `spud_update` runs [`shooter_spud`] instead.

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::camera::{CAMERA_X, CAMERA_Y, SPRITES_HIDDEN};
use crate::level::objects::{bob_anim, bob_follow, nothing, obj_spawn_at_xy, overlapping};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// h0+$7f0a, h0+$7f0e: where [`spawn_shots`] put the shots, as longs.
pub const SHOT_SPAWN_X: u32 = h0(0x7f0a);
pub const SHOT_SPAWN_Y: u32 = h0(0x7f0e);
/// `shot_power` (h0+$7f12): non-zero once the extra two shots fire; halved, it is the damage a
/// shot does past the first point (`touch_bonus_enemy`).
pub const SHOT_POWER: u32 = h0(0x7f12);
/// `shot_delay` (h0+$80c6): passes until the fire button sends the next shot.
pub const SHOT_DELAY: u32 = h0(0x80c6);
/// `shot_cycle` (h0+$80c8): the entry of `shot_cycle_table` (h0+$80cc: the first three shots'
/// places in [`SHOTS`], then 0) the next shot comes from.
pub const SHOT_CYCLE: u32 = h0(0x80c8);
const SHOT_CYCLE_TABLE: u32 = h0(0x80cc);
/// `shots` (h0+$80dc): the five shots' slots; the last two are the extra shots.
pub const SHOTS: u32 = h0(0x80dc);
const EXTRA_SHOTS: [u32; 2] = [SHOTS + 0xc, SHOTS + 0x10];

/// The shots' update (h0+$8070), shared by the three kinds.
const SHOT_UPDATE: u32 = h0(0x8070);
/// The reward's update in shooter mode: a shot passes through it.
const REWARD_UPDATE: u32 = super::death::REWARD_FLOAT;

/// Template 110: a shot that flies straight (the cycle's three).
pub static SHOT: ObjectKind = ObjectKind {
    name: "shot",
    routines: &[
        (h0(0x802e), shot_init),
        (SHOT_UPDATE, shot_update),
        (h0(0x7d4), nothing),
    ],
};

/// Template 111: the extra shot that climbs (`+$e` -12).
pub static SHOT_UP: ObjectKind = ObjectKind {
    name: "shot, climbing",
    routines: &[
        (h0(0x8044), shot_up_init),
        (SHOT_UPDATE, shot_update),
        (h0(0x7d4), nothing),
    ],
};

/// Template 112: the extra shot that drops (`+$e` 12).
pub static SHOT_DOWN: ObjectKind = ObjectKind {
    name: "shot, dropping",
    routines: &[
        (h0(0x805a), shot_down_init),
        (SHOT_UPDATE, shot_update),
        (h0(0x7d4), nothing),
    ],
};

/// h0+$802e: the straight shot's look.
fn shot_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32e8c));
}

/// h0+$8044: the climbing shot's look.
fn shot_up_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32e98));
}

/// h0+$805a: the dropping shot's look.
fn shot_down_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32ea4));
}

/// h0+$7e3e (the end of `level_init_lb`): the five shots, side by side `$160` right of the
/// camera. Their y is that x plus `$130`, the original's slip (it adds to d0, not d1), which puts
/// them well below the screen, where [`shot_update`] parks them at once.
pub fn spawn_shots(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let x = b.w(CAMERA_X + 2).wrapping_add(0x160);
    let y = x.wrapping_add(0x130);
    b.set_l(SHOT_SPAWN_X, x as i16 as u32);
    b.set_l(SHOT_SPAWN_Y, y as i16 as u32);
    for (k, t) in [110u32, 110, 110, 111, 112].into_iter().enumerate() {
        let template = crate::items::template(t);
        let s = obj_spawn_at_xy(c, template, x.wrapping_add(0x10 * k as u16), y);
        c.bus.set_l(SHOTS + 4 * k as u32, s.a0);
    }
}

/// h0+$7f14, `spud_update` in shooter mode: the spud stays put away; fire sends the extra shots
/// (both, once both have left the screen and the power is on) and, every fourth pass while it is
/// held, the cycle's next shot if that one has left the screen. A shot starts at the frog: x its
/// own, y `$28` below (the cycle's `$2a`).
pub fn shooter_spud(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(SPRITES_HIDDEN) != 0 || b.w(FROG + f::DYING) != 0 {
        return;
    }
    b.set_w(FROG + 0x72, 0);
    b.set_w(FROG + 0x74, 0x639c);
    b.set_l(h0(0x297b2), b.l(FROG + 0x74));
    let edge = b.w(CAMERA_X + 2).wrapping_add(0x150);
    let x = b.w(FROG + f::X);
    let y = b.w(FROG + f::Y).wrapping_add(0x28);
    // Has the shot in this place left the screen (past the camera by `$150`)?
    let gone = |b: &crate::mem::Bus, place: u32| !n_clear(edge.wrapping_sub(b.w(b.l(place) + 8)));
    let fire = c.hw.fire;
    if fire && b.w(SHOT_POWER) != 0 && EXTRA_SHOTS.iter().all(|&p| gone(b, p)) {
        for p in EXTRA_SHOTS {
            let s = b.l(p);
            b.set_w(s + 8, x);
            b.set_w(s + 0xa, y);
        }
    }
    let delay = b.w(SHOT_DELAY).wrapping_sub(1);
    b.set_w(SHOT_DELAY, delay);
    if n_clear(delay) || !fire {
        return;
    }
    c.sfx(0, 2);
    let b = &mut c.bus;
    b.set_w(SHOT_DELAY, 3);
    let mut at = b.l(SHOT_CYCLE);
    if b.l(at) == 0 {
        at = SHOT_CYCLE_TABLE;
        b.set_l(SHOT_CYCLE, at);
    }
    let place = b.l(at);
    if !gone(b, place) {
        return;
    }
    b.set_l(SHOT_CYCLE, at.wrapping_add(4));
    let s = b.l(place);
    b.set_w(s + 8, x);
    b.set_w(s + 0xa, y.wrapping_add(2));
}

/// h0+$8070: a shot moves by `+$c`, `+$e` while it is left of the camera's `$200`. Off the screen
/// up or down (y − camera y outside `-$20..$120`) it is parked at the camera's `$200` instead.
pub fn shot_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let far = b.w(CAMERA_X + 2).wrapping_add(0x200);
    let mut x = b.w(slot + 8);
    if n_clear(far.wrapping_sub(x)) {
        x = x.wrapping_add(b.w(slot + 0xc));
    }
    b.set_w(slot + 8, x);
    let dy = b.w(slot + 0xa).wrapping_sub(b.w(CAMERA_Y + 2));
    if !n_clear(dy.wrapping_sub(0xffe0)) || n_clear(dy.wrapping_sub(0x120)) {
        b.set_w(slot + 8, far);
    } else {
        b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    }
    bob_follow(b, slot);
}

/// h0+$8008: every shot one faster (`+$c`), but a shot already at `$1e`.
pub fn shots_faster(c: &mut LevelCtx) {
    let b = &mut c.bus;
    for k in 0..5 {
        let s = b.l(SHOTS + 4 * k);
        let v = b.w(s + 0xc);
        if v != 0x1e {
            b.set_w(s + 0xc, v.wrapping_add(1));
        }
    }
}

/// h0+$d1d0, `collide_all` in shooter mode: each shot's box (`$20` wide; the cycle's `$10` high,
/// the extra shots `$20`) against every object's. A hit runs the object's touch handler as the
/// frog's touch (d7 = 0) and throws the shot `$280` right, out of play; a reward is passed through.
/// The shot's box stays where it was for the rest of the list.
pub fn shot_touches(c: &mut LevelCtx) {
    if c.bus.w(SPRITES_HIDDEN) != 0 {
        return;
    }
    for k in 0..5 {
        let b = &c.bus;
        let shot = b.l(SHOTS + 4 * k);
        let h = if k < 3 { 0x10 } else { 0x20 };
        let (x, y) = (b.w(shot + 8), b.w(shot + 0xa));
        for slot in overlapping(b, x, x.wrapping_add(0x20), y, y.wrapping_add(h)) {
            if c.bus.l(slot + 4) == REWARD_UPDATE {
                continue;
            }
            let touch = c.bus.l(slot + 0x10);
            crate::level::call(c, touch, slot, 0);
            let b = &mut c.bus;
            b.set_w(shot + 8, b.w(shot + 8).wrapping_add(0x280));
        }
    }
}
