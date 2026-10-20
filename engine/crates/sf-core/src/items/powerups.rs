//! The power-ups' own routines. Wings, speed and the shield run in the frog's routines (T14) once
//! a pickup ([`super::pickups`]) has set them; the spud, which the frog throws, is its own
//! routine in the 25 Hz pass (`spud_update`, h0+$80f0).

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::camera::{CAMERA_X, CAMERA_Y, SHOOTER_MODE, SPRITES_HIDDEN};
use crate::level::objects::SPUD_BOB;
use crate::level::{LevelCtx, n_clear};
use crate::mem::Bus;

/// The spud's fields in the frog's record: in flight, x, y, vx, vy, and the passes before it
/// starts testing walls.
const IN_FLIGHT: u32 = 0x72;
const SPUD_X: u32 = 0x74;
const SPUD_Y: u32 = 0x76;
const SPUD_VX: u32 = 0x78;
const SPUD_VY: u32 = 0x7a;
const SPUD_WAIT: u32 = 0xae;
/// The spud's parked frame (`bob_records+$162c`).
const PARKED: u32 = h0(0x2aefe);
/// The spud's splat against a wall (h0+$2d0b0), and the frame its splat ends on (h0+$2af0a).
const SPLAT: u32 = h0(0x2d0b0);
const SPLAT_END: u32 = h0(0x2af0a);
/// `map_rows` (h0+$17334).
const MAP_ROWS: u32 = h0(0x17334);

/// `spud_update` (h0+$80f0): with the spud (`+$7c`) fire on the ground or a platform throws it, in
/// the way the joystick points (up, up and forward, forward); in flight it steers back to the frog
/// three pixels a pass, splats on a wall, and is gone off the screen or once the frog catches it.
/// Fire in the air (or no spud) puts it away. In shooter mode the frog fires shots instead
/// ([`crate::bonus::shots::shooter_spud`]).
pub fn spud_update(c: &mut LevelCtx) {
    if c.bus.w(SHOOTER_MODE) != 0 {
        return crate::bonus::shots::shooter_spud(c);
    }
    let b = &mut c.bus;
    if b.w(FROG + f::SPUD_ON) == 0 || b.w(SPRITES_HIDDEN) != 0 {
        return park(b);
    }
    let fire = c.hw.fire;
    if fire {
        if b.w(FROG + f::ON_PLATFORM) == 0 && b.w(FROG + f::ON_GROUND) == 0 {
            return park(b);
        }
        b.set_w(FROG + f::LAUNCHED, 8);
    }
    if b.w(FROG + IN_FLIGHT) != 0 {
        return fly(c);
    }
    if !fire || (b.w(FROG + f::ON_GROUND) == 0 && b.w(FROG + f::ON_PLATFORM) == 0) {
        return park(b);
    }
    throw(c);
}

/// h0+$8154: the throw, from the frog's place.
fn throw(c: &mut LevelCtx) {
    #[derive(Clone, Copy)]
    enum Way {
        Up,
        UpRight,
        Right,
        UpLeft,
        Left,
    }
    let b = &mut c.bus;
    b.set_l(FROG + SPUD_X, b.l(FROG));
    let joy = c.hw.joy1dat & 0x303;
    let facing = b.w(FROG + f::FACING);
    let way = if joy == 0x100 {
        Way::Up
    } else if facing == 0 && joy == 0x103 {
        Way::UpRight
    } else if facing == 0 && joy == 3 {
        Way::Right
    } else if facing != 0 && joy == 0x200 {
        Way::UpLeft
    } else if facing != 0 && joy == 0x300 {
        Way::Left
    } else if n_clear(facing) {
        Way::UpRight
    } else {
        Way::UpLeft
    };
    let (dx, dy, anim, vy, vx, throw_anim): (u16, u16, u32, u16, u16, u16) = match way {
        Way::Up => (0xc, 0x10, h0(0x2cf7c), 0xffe8, 0, 1),
        Way::UpRight => (8, 0x20, h0(0x2cfa8), 0xffe8, 0x18, 2),
        Way::Right => (8, 0x20, h0(0x2cfd4), 0, 0x18, 3),
        Way::UpLeft => (0x10, 0x20, h0(0x2d084), 0xffe8, 0xffe8, 2),
        Way::Left => (0x10, 0x20, h0(0x2d058), 0, 0xffe8, 3),
    };
    b.set_w(FROG + SPUD_X, b.w(FROG + SPUD_X).wrapping_add(dx));
    b.set_w(FROG + SPUD_Y, b.w(FROG + SPUD_Y).wrapping_add(dy));
    b.set_l(SPUD_BOB + 0x28, anim);
    b.set_l(SPUD_BOB + 0x42, anim);
    b.set_w(FROG + SPUD_VY, vy);
    b.set_w(FROG + SPUD_VX, vx);
    b.set_w(FROG + f::SPECIAL_ANIM, throw_anim);
    b.set_w(FROG + IN_FLIGHT, 1);
    b.set_w(FROG + SPUD_WAIT, 2);
    let effect = c.rng_range(1).wrapping_add(0xf);
    c.sfx(effect as u8, 3);
}

/// h0+$82d4: the spud out of play, far off at x `$639c`.
pub fn park(b: &mut Bus) {
    b.set_w(FROG + IN_FLIGHT, 0);
    b.set_w(FROG + SPUD_X, 0x639c);
    b.set_l(SPUD_BOB - 4, b.l(FROG + SPUD_X));
    b.set_l(SPUD_BOB + 0x28, PARKED);
    b.set_l(SPUD_BOB + 0x42, PARKED);
}

/// h0+$82fc: the spud in flight.
fn fly(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let wait = b.w(FROG + SPUD_WAIT);
    let test_wall = if n_clear(wait) {
        let w = wait.wrapping_sub(1);
        b.set_w(FROG + SPUD_WAIT, w);
        !n_clear(w)
    } else {
        true
    };
    if test_wall {
        let x = b.w(FROG + SPUD_X);
        let mut y = b.w(FROG + SPUD_Y);
        if n_clear(b.w(FROG + SPUD_VY)) {
            y = y.wrapping_sub(8);
        }
        let row = (y >> 4).wrapping_mul(4);
        let cell = b
            .l(MAP_ROWS.wrapping_add(row as i16 as i32 as u32))
            .wrapping_add((x >> 4).wrapping_mul(2) as u32);
        if n_clear((b.w(cell + 2) & 0x3f).wrapping_sub(0x3a)) {
            // A wall: the splat (its sound once).
            if b.l(SPUD_BOB + 8) != SPLAT {
                c.sfx(7, 2);
                let b = &mut c.bus;
                b.set_w(FROG + SPUD_X, b.w(FROG + SPUD_X).wrapping_sub(8));
                b.set_l(SPUD_BOB + 0x28, SPLAT);
                b.set_l(SPUD_BOB + 0x42, SPLAT);
                b.set_l(SPUD_BOB - 4, b.l(FROG + SPUD_X));
                b.set_w(FROG + SPUD_VX, 0);
                b.set_w(FROG + SPUD_VY, 0);
                return;
            }
            let b = &mut c.bus;
            b.set_l(SPUD_BOB - 4, b.l(FROG + SPUD_X));
            b.set_w(FROG + SPUD_VX, 0);
            b.set_w(FROG + SPUD_VY, 0);
            if n_clear(b.w(SPUD_BOB + 0x1a).wrapping_sub(0x4e20)) {
                park(b);
            }
            return;
        }
    }
    let b = &mut c.bus;
    if b.l(SPUD_BOB + 0x42) != SPLAT {
        b.set_w(
            FROG + SPUD_X,
            b.w(FROG + SPUD_X).wrapping_add(b.w(FROG + SPUD_VX)),
        );
        b.set_w(
            FROG + SPUD_Y,
            b.w(FROG + SPUD_Y).wrapping_add(b.w(FROG + SPUD_VY)),
        );
    }
    if b.l(b.l(SPUD_BOB + 0x16)) == SPLAT_END {
        return park(b);
    }
    // Back to the frog, three a pass up to 16.
    let to = (b.w(FROG + f::X).wrapping_add(0x14).wrapping_add(4)) & 0xfff8;
    steer(b, SPUD_VX, b.w(FROG + SPUD_X) & 0xfff8, to);
    let to = b.w(FROG + f::Y).wrapping_add(0x20) & 0xfff0;
    steer(b, SPUD_VY, b.w(FROG + SPUD_Y) & 0xfff0, to);
    let (x, y) = (b.w(FROG + SPUD_X), b.w(FROG + SPUD_Y));
    let (cx, cy) = (b.w(CAMERA_X + 2), b.w(CAMERA_Y + 2));
    if n_clear(cx.wrapping_sub(x))
        || !n_clear(cx.wrapping_add(0x150).wrapping_sub(x))
        || n_clear(cy.wrapping_sub(y))
        || !n_clear(cy.wrapping_add(0x110).wrapping_sub(y))
    {
        return park(b);
    }
    // Caught: the frog's box against the spud's.
    let x0 = b.w(FROG + f::X).wrapping_add(4).wrapping_add(0xe);
    let y0 = b.w(FROG + f::Y).wrapping_add(4).wrapping_add(0x16);
    let (x1, y1) = (x0.wrapping_add(0xa), y0.wrapping_add(0x20));
    let sx0 = x.wrapping_add(0xfffc);
    let sy0 = y.wrapping_add(0xfffc);
    let (sx1, sy1) = (sx0.wrapping_add(0x10), sy0.wrapping_add(0x10));
    let apart = !n_clear(sx1.wrapping_sub(x0))
        || n_clear(sx0.wrapping_sub(x1))
        || !n_clear(sy1.wrapping_sub(y0))
        || n_clear(sy0.wrapping_sub(y1));
    if !apart {
        b.set_w(FROG + IN_FLIGHT, 0);
        b.set_w(FROG + SPUD_X, 0x3e80);
    }
    b.set_l(SPUD_BOB - 4, b.l(FROG + SPUD_X));
}

/// h0+$83d8: one axis of the way back: `at` (the spud's) against `to` (the frog's).
fn steer(b: &mut Bus, field: u32, at: u16, to: u16) {
    let v = b.w(FROG + field);
    if at == to {
        b.set_w(FROG + field, 0);
    } else if n_clear(at.wrapping_sub(to)) {
        if n_clear(v.wrapping_sub(0xfff0)) {
            b.set_w(FROG + field, v.wrapping_sub(3));
        }
    } else if !n_clear(v.wrapping_sub(0x10)) {
        b.set_w(FROG + field, v.wrapping_add(3));
    }
}
