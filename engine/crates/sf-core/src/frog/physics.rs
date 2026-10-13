//! The frog's motion: velocity, gravity, running and jumping, and the camera that follows it.

use super::{FROG, f};
use crate::level::LevelCtx;

/// h0+$db58: the frog moves by its velocity. A coordinate that goes negative is put back to 0 and
/// that velocity stopped.
pub fn frog_move(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(FROG + f::FROZEN) != 0 {
        return;
    }
    for (pos, vel) in [(f::X, f::VX), (f::Y, f::VY)] {
        let p = b.w(FROG + pos).wrapping_add(b.w(FROG + vel));
        b.set_w(FROG + pos, p);
        if (p as i16) < 0 {
            b.set_w(FROG + pos, 0);
            b.set_w(FROG + vel, 0);
        }
    }
}

use crate::frog::anim::{
    DEMO_JOY, frog_set_anim, frog_set_anim_at, frog_set_anim_free, set_anim,
};
use crate::layout::h0;
use crate::level::camera::SHOOTER_MODE;
use crate::level::collision::{LEFT_ANIMS, bounce, n_clear_l};
use crate::level::{ATTRACT_MODE, n_clear};
use crate::mem::Bus;

/// `sprites_hidden` (h0+$9e2).
const SPRITES_HIDDEN: u32 = h0(0x9e2);

/// `frog_walk_input` (h0+$113be): the joystick walks, runs, crouches and jumps the frog; friction
/// slows it when nothing is held; and the walking animation follows its speed and the slope.
pub fn frog_walk_input(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(SPRITES_HIDDEN) != 0 {
        return;
    }
    if n_clear(b.w(FROG + f::SPEED_UP)) {
        let t = b.w(FROG + f::SPEED_UP).wrapping_sub(1);
        b.set_w(FROG + f::SPEED_UP, t);
        if !n_clear(t) {
            let m = b.w(FROG + f::MAX_SPEED).wrapping_sub(0xc);
            b.set_w(FROG + f::MAX_SPEED, m);
            if !n_clear(m.wrapping_sub(6)) {
                b.set_w(FROG + f::MAX_SPEED, b.w(FROG + f::BASE_SPEED));
            }
        }
    }
    if b.w(FROG + f::BLINK) != 0 {
        b.set_w(FROG + f::BLINK, b.w(FROG + f::BLINK).wrapping_sub(1));
    }
    if b.w(FROG + f::FROZEN) != 0 {
        return;
    }
    let dx = b.w(FROG + f::X).wrapping_sub(b.w(FROG + f::PREV_X));
    if dx != 0 {
        b.set_w(FROG + f::FACING, if n_clear(dx) { 0 } else { 0xffff });
    }
    let dy = b.w(FROG + f::Y).wrapping_sub(b.w(FROG + f::PREV_Y));
    if dy != 0 {
        b.set_w(FROG + f::CLIMB, if n_clear(dy) { 0xffff } else { 0 });
    }
    // Every fourth frame the speed limit grows a step towards the ground's (or the air's) limit:
    // one more uphill, one less downhill.
    let mut limit = if b.w(FROG + f::ON_GROUND) != 0 {
        b.w(FROG + f::MAX_SPEED)
    } else {
        b.w(FROG + f::BASE_SPEED)
    };
    let tick = b.w(FROG + f::SPEED_TICK).wrapping_add(1);
    b.set_w(FROG + f::SPEED_TICK, tick);
    if n_clear(tick.wrapping_sub(4)) {
        b.set_w(FROG + f::SPEED_TICK, 0);
        let level = b.w(FROG + f::Y) & 0xfffe == b.w(FROG + f::PREV_Y) & 0xfffe;
        if !level && b.w(FROG + f::AIR) == 0 {
            limit = if b.w(FROG + f::CLIMB) != 0 {
                limit.wrapping_add(1)
            } else {
                limit.wrapping_sub(1)
            };
        }
        let speed = b.w(FROG + f::SPEED).wrapping_add(1);
        b.set_w(FROG + f::SPEED, speed);
        if !n_clear(limit.wrapping_sub(speed)) {
            b.set_w(FROG + f::SPEED, limit);
        }
    }
    b.set_l(FROG + f::PREV_X, b.l(FROG + f::X));
    if b.w(FROG + f::CROUCH_TIME) != 0 {
        b.set_w(FROG + f::CROUCH_TIME, b.w(FROG + f::CROUCH_TIME).wrapping_sub(1));
    }
    if b.w(FROG + f::CROUCH) != 0 {
        b.set_w(FROG + f::CROUCH_TIME, 5);
    }
    b.set_w(FROG + f::CROUCH, 0);
    let mut joy = c.hw.joy1dat;
    let b = &mut c.bus;
    if b.l(FROG + f::HURT) != 0 {
        joy = 0;
    } else if b.w(FROG + f::LAUNCHED) != 0 {
        b.set_w(FROG + f::LAUNCHED, b.w(FROG + f::LAUNCHED).wrapping_sub(1));
        joy = 0;
    }
    if b.l(ATTRACT_MODE) != 0 {
        joy = b.w(DEMO_JOY);
    }
    let joy = joy & 0x303;
    if b.w(SHOOTER_MODE) != 0 {
        return shooter_input(b, joy);
    }
    let (wall_r, wall_l) = (b.w(FROG + f::WALL_RIGHT) != 0, b.w(FROG + f::WALL_LEFT) != 0);
    match joy {
        0x100 => return up(c),
        0x103 => return up_side(c, false),
        3 if !wall_r => return walk(c, false),
        2 if !wall_r => b.set_w(FROG + f::CROUCH, 1),
        1 => b.set_w(FROG + f::CROUCH, 1),
        0x200 => return up_side(c, true),
        0x300 if !wall_l => return walk(c, true),
        0x301 if !wall_l => b.set_w(FROG + f::CROUCH, 1),
        _ => {}
    }
    no_walk(c);
}

/// Whether fire (port 2) is held: CIA-A PRA bit 7 low.
fn fire(c: &LevelCtx) -> bool {
    c.hw.fire
}

/// h0+$11614: up jumps, unless the spud is off and fire is held (then nothing is walked).
pub(crate) fn up(c: &mut LevelCtx) {
    if c.bus.w(FROG + f::SPUD_ON) == 0 && fire(c) {
        return no_walk(c);
    }
    bounce(c);
}

/// h0+$11676 / h0+$116ea: up and to one side jumps that way at the walking speed; in the air it
/// walks that way instead.
fn up_side(c: &mut LevelCtx, left: bool) {
    if c.bus.w(FROG + f::SPUD_ON) == 0 && fire(c) {
        return no_walk(c);
    }
    let b = &mut c.bus;
    let wall = if left { f::WALL_LEFT } else { f::WALL_RIGHT };
    if b.w(FROG + wall) != 0 {
        return up(c);
    }
    if b.w(FROG + f::AIR) != 0 {
        return walk(c, left);
    }
    if b.w(FROG + f::HEAD_BUMP) != 0 {
        return up(c);
    }
    if b.w(FROG + f::ON_PLATFORM) == 0 && b.w(FROG + f::ON_GROUND) == 0 {
        return;
    }
    b.set_w(FROG + f::AIR, 1);
    let speed = b.w(FROG + f::SPEED);
    b.set_w(FROG + f::VX, if left { speed.wrapping_neg() } else { speed });
    b.set_w(FROG + f::GRAVITY_TICK, 3);
    b.set_w(FROG + f::VY, b.w(FROG + f::BOUNCE_VY));
    b.set_w(FROG + f::HEAD_BUMP, 1);
}

/// h0+$1175e / h0+$117fe: walking right or left speeds the frog a pixel a frame towards the speed
/// limit (every second frame on the ground; in the air every few frames).
fn walk(c: &mut LevelCtx, left: bool) {
    let b = &mut c.bus;
    let wall = if left { f::WALL_LEFT } else { f::WALL_RIGHT };
    if b.w(FROG + wall) != 0 {
        return;
    }
    if b.w(FROG + f::SLIDING) != 0 && b.w(FROG + f::SLIDE_TICK) != 0 {
        return;
    }
    if b.w(FROG + f::AIR) != 0 {
        let t = b.w(FROG + f::AIR_STEP).wrapping_add(1);
        b.set_w(FROG + f::AIR_STEP, t);
        if b.w(FROG + f::ON_GROUND) != 0 {
            if !n_clear(t.wrapping_sub(2)) {
                return;
            }
        } else if n_clear(b.w(FROG + f::AIR_STEP_DELAY).wrapping_sub(t)) {
            return;
        }
        b.set_w(FROG + f::AIR_STEP, 0);
    }
    let (anim, step) = if left {
        (h0(0x105b6), h0(0x105ae))
    } else {
        (h0(0x10d8e), h0(0x10d86))
    };
    frog_set_anim_at(c, anim, step);
    let b = &mut c.bus;
    if b.w(FROG + f::AIR) == 0 {
        let toggle = if left { f::LEFT_TOGGLE } else { f::RIGHT_TOGGLE };
        let t = !b.w(FROG + toggle);
        b.set_w(FROG + toggle, t);
        if t == 0 {
            return;
        }
    }
    let mut target = b.w(FROG + f::SPEED);
    if b.w(FROG + f::ON_GROUND) == 0 {
        target = target.wrapping_add(b.w(FROG + f::AIR_SPEED));
    }
    let slope = b.w(FROG + f::SLOPE);
    let uphill = if left {
        !n_clear(slope.wrapping_sub(1))
    } else {
        n_clear(slope.wrapping_add(1))
    };
    if uphill && b.w(FROG + f::PUSHED) != 0 {
        target = target.wrapping_sub(4);
    }
    let vx = b.w(FROG + f::VX);
    if left {
        let d = target.wrapping_neg().wrapping_sub(vx);
        if !n_clear(d) {
            b.set_w(FROG + f::VX, vx.wrapping_sub(1));
        } else if d != 0 {
            b.set_w(FROG + f::VX, vx.wrapping_add(1));
        }
    } else if (target as i16) > (vx as i16) {
        b.set_w(FROG + f::VX, vx.wrapping_add(1));
    } else if target != vx {
        b.set_w(FROG + f::VX, vx.wrapping_sub(1));
    }
}

/// h0+$1154c: nothing walked: friction every second frame (every sixth while hurt), then the
/// animation for the frog's speed on the slope.
fn no_walk(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(FROG + f::SLIDE_TICK) == 0 {
        let brake = if b.l(FROG + f::HURT) != 0 {
            let t = b.w(FROG + f::HURT_FRICTION).wrapping_add(1);
            b.set_w(FROG + f::HURT_FRICTION, t);
            let due = n_clear(t.wrapping_sub(6));
            if due {
                b.set_w(FROG + f::HURT_FRICTION, 0);
            }
            due
        } else {
            let t = !b.w(FROG + f::FRICTION_TOGGLE);
            b.set_w(FROG + f::FRICTION_TOGGLE, t);
            t != 0
        };
        let vx = b.w(FROG + f::VX);
        if brake && vx != 0 {
            b.set_w(
                FROG + f::VX,
                if n_clear(vx) { vx.wrapping_sub(1) } else { vx.wrapping_add(1) },
            );
        }
    }
    // h0+$11584: a steep slope (4 or more) or a slow frog takes the slope's animation.
    let slope = b.w(FROG + f::SLOPE);
    let steep = n_clear((if n_clear(slope) { slope } else { slope.wrapping_neg() }).wrapping_sub(4));
    let vx = b.w(FROG + f::VX);
    let speed = if n_clear(vx) { vx } else { vx.wrapping_neg() };
    let anim = b.l(FROG + f::ANIM);
    if !steep && n_clear(speed.wrapping_sub(5)) {
        // h0+$115ec: fast: the running animation.
        let a1 = if anim > LEFT_ANIMS { h0(0x10cd6) } else { h0(0x104fe) };
        return frog_set_anim_free(c, a1);
    }
    if !steep && anim == h0(0x10cd6) {
        return;
    }
    let a1 = b.l(FROG + f::SLOPE_ANIM);
    if !steep && n_clear_l(anim.wrapping_sub(LEFT_ANIMS)) {
        return frog_set_anim_free(c, a1);
    }
    if !steep && anim == h0(0x104fe) {
        return;
    }
    frog_set_anim_free(c, a1);
}

/// h0+$118a0: the bonus level's flying frog: speed decays every second frame and stays within
/// ±3, and the joystick pushes it by 2 in each direction held.
fn shooter_input(b: &mut Bus, joy: u16) {
    let t = !b.w(SHOOTER_DECAY);
    b.set_w(SHOOTER_DECAY, t);
    if t != 0 {
        for v in [f::VX, f::VY] {
            if n_clear(b.w(FROG + v).wrapping_sub(1)) {
                b.set_w(FROG + v, b.w(FROG + v).wrapping_sub(1));
            }
            if !n_clear(b.w(FROG + v)) {
                b.set_w(FROG + v, b.w(FROG + v).wrapping_add(1));
            }
        }
    }
    for v in [f::VX, f::VY] {
        if n_clear(b.w(FROG + v).wrapping_sub(3)) {
            b.set_w(FROG + v, 3);
        }
        if !n_clear(b.w(FROG + v).wrapping_add(2)) {
            b.set_w(FROG + v, 0xfffd);
        }
    }
    let (dx, dy): (i16, i16) = match joy {
        0x100 => (0, -2),
        0x103 => (2, -2),
        3 => (2, 0),
        2 => (2, 2),
        1 => (0, 2),
        0x200 => (-2, -2),
        0x300 => (-2, 0),
        0x301 => (-2, 2),
        _ => return,
    };
    // The original adds y before x where it does both.
    if dy != 0 {
        b.set_w(FROG + f::VY, b.w(FROG + f::VY).wrapping_add(dy as u16));
    }
    if dx != 0 {
        b.set_w(FROG + f::VX, b.w(FROG + f::VX).wrapping_add(dx as u16));
    }
}

/// `shooter_decay` (h0+$1194e): toggles every frame in the bonus level.
const SHOOTER_DECAY: u32 = h0(0x1194e);

/// `frog_air_anim` (h0+$11056): fire in the air (or in a flight) plays the jump sound once and
/// lifts the frog; the shield, launcher and special animations; and the sound of a frog sitting
/// on a pushing surface.
pub fn frog_air_anim(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(FROG + f::ANIM_LOCK) != 0 {
        b.set_w(FROG + f::ANIM_LOCK, b.w(FROG + f::ANIM_LOCK).wrapping_sub(1));
    }
    if b.w(FROG + f::HURT) != 0 {
        return;
    }
    if b.w(FROG + f::ON_GROUND) != 0 {
        b.set_w(FROG + f::JUMP_PRESSES, 0);
    }
    let demo_fire = b.w(DEMO_FIRE) != 0;
    let pressed = demo_fire
        || if b.l(ATTRACT_MODE) == 0 && b.w(FROG + f::FLIGHT) == 0 {
            false
        } else if c.hw.fire {
            true
        } else {
            c.bus.set_w(FROG + f::FIRE_HELD, 0);
            false
        };
    let b = &mut c.bus;
    if pressed && b.w(FROG + f::ON_PLATFORM) == 0 && b.w(FROG + f::ON_GROUND) == 0 {
        let moving = b.w(FROG + f::VX) != 0;
        let a1 = if n_clear_l(b.l(FROG + f::ANIM).wrapping_sub(LEFT_ANIMS)) {
            if moving { h0(0x10dfe) } else { h0(0x10e0a) }
        } else if moving {
            h0(0x10626)
        } else {
            h0(0x10632)
        };
        frog_set_anim(c, a1);
        let b = &mut c.bus;
        if b.w(FROG + f::FIRE_HELD) == 0 {
            c.sfx(0x13, 3);
            let b = &mut c.bus;
            b.set_w(FROG + f::FIRE_HELD, 1);
            if n_clear(b.w(FROG + f::VY).wrapping_sub(1))
                && !n_clear(b.w(FROG + f::JUMP_PRESSES).wrapping_sub(8))
            {
                b.set_w(FROG + f::JUMP_PRESSES, b.w(FROG + f::JUMP_PRESSES).wrapping_add(1));
                let vy = b.w(FROG + f::VY).wrapping_sub(2);
                b.set_w(FROG + f::VY, vy);
                if !n_clear(vy.wrapping_add(2)) {
                    b.set_w(FROG + f::VY, 1);
                }
            }
        }
    }
    // h0+$11120: the shield.
    let b = &mut c.bus;
    if b.w(FROG + f::SHIELD) != 0 && b.w(FROG + f::BLINK) == 0 {
        let s = b.w(FROG + f::SHIELD).wrapping_sub(1);
        b.set_w(FROG + f::SHIELD, s);
        if s == 0 {
            b.set_w(FROG + f::BLINK, 0x28);
        }
        let mut a1 = if b.w(FROG + f::FACING) != 0 { h0(0x103f6) } else { h0(0x10bce) };
        if b.w(FROG + f::Y) != b.w(FROG + f::PREV_Y) {
            a1 += 0xc;
            if b.w(FROG + f::CLIMB) != 0 {
                a1 += 0xc;
            }
        }
        return frog_set_anim(c, a1);
    }
    // h0+$11170: a launcher's hold.
    let hold = b.w(FROG + f::LAUNCH_HOLD);
    if hold != 0 && n_clear(hold) {
        b.set_w(FROG + f::LAUNCH_HOLD, hold.wrapping_sub(1));
        let a1 = if b.w(FROG + f::FACING) == 0 { h0(0x1041a) } else { h0(0x10bf2) };
        return frog_set_anim(c, a1);
    }
    if b.w(SHOOTER_MODE) != 0 {
        return frog_set_anim(c, h0(0x10b7e));
    }
    if b.w(FROG + f::SPECIAL_ANIM) != 0 {
        let table = if b.w(FROG + f::FACING) != 0 { h0(0x11046) } else { h0(0x11036) };
        let k = b.w(FROG + f::SPECIAL_ANIM).wrapping_mul(4);
        let a1 = b.l(table.wrapping_add(k as i16 as i32 as u32));
        frog_set_anim(c, a1);
        let b = &mut c.bus;
        if n_clear(b.w(FROG + f::ANIM_TIME).wrapping_sub(0x4e20)) {
            b.set_w(FROG + f::SPECIAL_ANIM, 0);
        }
    }
    // h0+$111e0: sitting still on a moving surface makes the frog croak now and then.
    let b = &mut c.bus;
    if b.w(FROG + f::SIT_TIME) == 0 {
        b.set_w(FROG + f::IDLE_SOUND, 0);
    }
    let vx = b.w(FROG + f::VX);
    let slope = b.w(FROG + f::SLOPE);
    let fast = n_clear(vx.wrapping_sub(7)) || !n_clear(vx.wrapping_add(6));
    let steep = n_clear(slope.wrapping_sub(1)) || !n_clear(slope.wrapping_add(1));
    let mut sit = false;
    if fast && steep
        && b.w(FROG + f::CLIMB) != 0
        && c.hw.joy1dat & 0x303 == 0
        && (b.w(FROG + f::SLIDING) != 0 || b.w(FROG + f::PUSHED) != 0)
    {
        let t = b.w(FROG + f::IDLE_SOUND).wrapping_add(1);
        b.set_w(FROG + f::IDLE_SOUND, t);
        if !n_clear(t.wrapping_sub(2)) && b.l(WORLD) != 5 && n_clear(c.rng_range(3).wrapping_sub(2))
        {
            c.sample(0);
        }
        c.bus.set_w(FROG + f::SIT_TIME, 4);
        sit = true;
    }
    let b = &mut c.bus;
    if !sit {
        if b.w(FROG + f::SIT_TIME) != 0 {
            b.set_w(FROG + f::SIT_TIME, b.w(FROG + f::SIT_TIME).wrapping_sub(1));
            sit = true;
        } else if b.w(FROG + f::CROUCH) != 0
            && (b.w(FROG + f::ON_PLATFORM) != 0 || b.w(FROG + f::ON_GROUND) != 0)
        {
            sit = true;
        }
    }
    if sit {
        // h0+$1128c: on the ground at rest (or falling fast): the sitting animation.
        let rest = if b.w(FROG + f::ON_GROUND) != 0 && n_clear(b.w(FROG + f::VY).wrapping_sub(7)) {
            true
        } else {
            b.w(FROG + f::VY) == 0
        };
        if rest {
            let a1 = if n_clear_l(b.l(FROG + f::ANIM).wrapping_sub(LEFT_ANIMS)) {
                h0(0x10df2)
            } else {
                h0(0x1061a)
            };
            frog_set_anim(c, a1);
        }
    }
    // h0+$112b6: on a pushing surface under a marker-`$39` cell, now and then a bubble rises
    // (an object, T15–T18); a frog at rest on it is locked in its sitting animation.
    let b = &mut c.bus;
    if b.w(FROG + f::PUSHED) == 0 {
        return;
    }
    let cell = b.l(FROG + f::GROUND_CELL);
    if b.w(cell.wrapping_sub(0x260)) & 0x3f != 0x39 {
        return;
    }
    let t = b.w(IDLE_TIMER).wrapping_add(1);
    b.set_w(IDLE_TIMER, t);
    if n_clear(t.wrapping_sub(0x40)) {
        // The original spawns template +$3a74 above the frog here (obj_spawn_at_xy).
        b.set_w(IDLE_TIMER, 0);
    }
    if b.w(FROG + f::VX) != 0 || b.w(FROG + f::VY) != 9 || c.hw.joy1dat & 0x303 != 0 {
        return;
    }
    let b = &mut c.bus;
    b.set_w(FROG + f::ANIM_LOCK, 2);
    let a1 = if b.w(FROG + f::FACING) != 0 { h0(0x103b2) } else { h0(0x10b8a) };
    set_anim(b, a1);
}

/// `demo_fire` (h0+$11a28): the attract demo's fire button (`$f000` held).
const DEMO_FIRE: u32 = h0(0x11a28);
/// `frog_idle_timer` (h0+$11350).
const IDLE_TIMER: u32 = h0(0x11350);
/// `world` (h0+$40da).
const WORLD: u32 = h0(0x40da);
/// `demo_left`, `demo_at` (h0+$11a2c, h0+$11a2e): frames the demo's current input lasts, and the
/// next entry of the recording (three words: start frame, joystick and fire, frames).
const DEMO_LEFT: u32 = h0(0x11a2c);
const DEMO_AT: u32 = h0(0x11a2e);
/// `demo_running` (h0+$11a24): cleared when the demo ends.
const DEMO_RUNNING: u32 = h0(0x11a24);
/// `level_state` (h0+$a06).
const LEVEL_STATE: u32 = h0(0xa06);

/// `demo_play` (h0+$11990): in the attract demo the recorded joystick replaces the player's; fire
/// or the recording's end stops the demo.
pub fn demo_play(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.l(ATTRACT_MODE) == 0 {
        return;
    }
    if !c.hw.fire {
        let b = &mut c.bus;
        let left = b.w(DEMO_LEFT).wrapping_sub(1);
        b.set_w(DEMO_LEFT, left);
        if n_clear(left.wrapping_sub(1)) {
            return;
        }
        let at = b.l(DEMO_AT);
        if b.w(at) != 0xffff {
            b.set_w(DEMO_LEFT, b.w(at.wrapping_add(4)));
            let input = b.w(at.wrapping_add(2));
            b.set_l(DEMO_AT, at.wrapping_add(4));
            b.set_w(DEMO_FIRE, if input & 0xf000 != 0 { 0 } else { 0xf000 });
            b.set_w(DEMO_JOY, input & 0x303);
            return;
        }
    }
    let b = &mut c.bus;
    b.set_l(DEMO_RUNNING, 0);
    b.set_l(LEVEL_STATE, 1);
    b.set_w(FROG + f::LIVES, 0xffff);
}

/// `demo_record` (h0+$11a32): a development recorder of the joystick, on while h0+$1c is set,
/// which nothing in the game sets; not ported beyond its test.
pub fn demo_record(c: &mut LevelCtx) {
    debug_assert_eq!(c.bus.w(h0(0x1c)), 0, "the demo recorder is a development leftover");
}
