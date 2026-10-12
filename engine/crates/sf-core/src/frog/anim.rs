//! The frog's two hardware sprites: where they show (h0+$19bbe) and the copper words that put
//! them there (h0+$1b23e).

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::camera::{FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT, SHOOTER_MODE};
use crate::level::{LevelCtx, n_clear};
use crate::mem::Bus;

/// Fields of a hardware-sprite record (`$1c` bytes; docs/09 "Hardware sprites").
pub mod spr {
    /// Screen x and y.
    pub const X: u32 = 0x00;
    pub const Y: u32 = 0x02;
    /// Lines tall.
    pub const HEIGHT: u32 = 0x04;
    /// Non-zero for an attached pair: the copper words of its partner follow.
    pub const ATTACHED: u32 = 0x06;
    /// The record's copper words in the copper list (pointer).
    pub const COPPER: u32 = 0x08;
    /// Its animation: the list (0 for none) and the current entry, and frames left on it.
    pub const ANIM: u32 = 0x10;
    pub const ANIM_AT: u32 = 0x14;
    pub const ANIM_LEFT: u32 = 0x18;
}

/// Where a sprite is put to hide it: past the right of the screen.
const OFF_SCREEN: u16 = 0x150;

/// h0+$19bbe: both frog sprites move by the frame's offset (frog `+$36`, `+$38`) and are hidden
/// when off screen, below the play field or blinking; then their copper words are written.
pub fn place_frog_sprites(c: &mut LevelCtx) {
    for s in [FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT] {
        let b = &mut c.bus;
        let (dx, dy) = (b.w(FROG + f::SPRITE_DX), b.w(FROG + f::SPRITE_DY));
        b.set_w(s + spr::X, b.w(s + spr::X).wrapping_add(dx));
        b.set_w(s + spr::Y, b.w(s + spr::Y).wrapping_add(dy));
        if b.w(FROG + f::BLINK) & 1 != 0 || n_clear(b.w(s + spr::X).wrapping_sub(OFF_SCREEN)) {
            b.set_w(s + spr::X, OFF_SCREEN);
        }
        if !n_clear(b.w(s + spr::X).wrapping_add(0x20)) {
            b.set_w(s + spr::X, OFF_SCREEN);
        }
        if !n_clear(b.w(s + spr::Y).wrapping_add(0x20)) {
            b.set_w(s + spr::Y, 0xffe0);
        }
        if b.w(SHOOTER_MODE) == 0 && n_clear(b.w(s + spr::Y).wrapping_sub(0xff)) {
            b.set_w(s + spr::X, OFF_SCREEN);
        }
        if b.w(FROG + f::SPRING) != 0 {
            b.set_w(s + spr::Y, 0x104);
        }
        sprite_to_copper(b, s);
    }
}

/// h0+$1b23e: a sprite record's position into its copper words (SPRxPOS, SPRxCTL: the moves'
/// data at `+$a` and `+$e`), and for an attached pair the partner's too, with its data pointer
/// one sprite on. A record with an animation steps it first.
pub fn sprite_to_copper(b: &mut Bus, s: u32) {
    let cop = b.l(s + spr::COPPER);
    if b.l(s + spr::ANIM) != 0 {
        animate(b, s, cop);
    }
    b.set_w(cop + 0xe, b.w(cop + 0xe) & 0x80);
    let mut x = b.w(s + spr::X).wrapping_add(0x80);
    if x & 1 != 0 {
        b.set_w(cop + 0xe, b.w(cop + 0xe) | 1);
    }
    x >>= 1;
    b.set_b(cop + 0xb, x as u8);
    let mut top = b.w(s + spr::Y).wrapping_add(0x2c);
    let mut bottom = top.wrapping_add(b.w(s + spr::HEIGHT));
    if n_clear(bottom.wrapping_sub(0x100)) {
        bottom = bottom.wrapping_sub(0xff);
        b.set_b(cop + 0xf, b.b(cop + 0xf) | 2);
    }
    b.set_b(cop + 0xe, bottom as u8);
    if n_clear(top.wrapping_sub(0x100)) {
        top = top.wrapping_sub(0xff);
        b.set_b(cop + 0xf, b.b(cop + 0xf) | 4);
    }
    b.set_b(cop + 0xa, top as u8);
    if b.w(s + spr::ATTACHED) == 0 {
        return;
    }
    b.set_w(cop + 0x1a, b.w(cop + 0xa));
    b.set_w(cop + 0x1e, b.w(cop + 0xe));
    let data = (b.w(cop + 2) as u32) << 16 | b.w(cop + 6) as u32;
    let next = data.wrapping_add((b.w(s + spr::HEIGHT) as i16 as i32 as u32) * 4 + 4);
    b.set_w(cop + 0x16, next as u16);
    b.set_w(cop + 0x12, (next >> 16) as u16);
}

/// h0+$1b2d6: a record's animation: each entry is a frame's data pointer (negative ends the list,
/// which starts again) and how many frames it shows (`+6`).
fn animate(b: &mut Bus, s: u32, cop: u32) {
    let left = b.w(s + spr::ANIM_LEFT).wrapping_sub(1);
    b.set_w(s + spr::ANIM_LEFT, left);
    if n_clear(left) {
        return;
    }
    b.set_l(s + spr::ANIM_AT, b.l(s + spr::ANIM_AT).wrapping_add(8));
    loop {
        let at = b.l(s + spr::ANIM_AT);
        let data = b.l(at);
        if (data as i32) < 0 {
            b.set_l(s + spr::ANIM_AT, b.l(s + spr::ANIM));
            continue;
        }
        b.set_w(s + spr::ANIM_LEFT, b.w(at.wrapping_add(6)));
        b.set_w(cop + 6, data as u16);
        b.set_w(cop + 2, (data >> 16) as u16);
        return;
    }
}

/// `demo_joy` (h0+$11a2a): the attract demo's recorded joystick, which the frog reads instead of
/// `JOY1DAT` while the demo runs.
pub const DEMO_JOY: u32 = h0(0x11a2a);
/// `attract_mode` (h0+$18).
use crate::level::ATTRACT_MODE;

/// The joystick as the frog routines read it: the demo's while the attract demo runs.
pub fn frog_joy(c: &LevelCtx) -> u16 {
    if c.bus.l(ATTRACT_MODE) != 0 {
        c.bus.w(DEMO_JOY)
    } else {
        c.hw.joy1dat
    }
}

/// `frog_jump_float` (h0+$11352): while up is held between 12 and 31 frames into a jump, every
/// second frame takes one more off the frog's rising speed; past 28 frames the jump ends at 32.
pub fn frog_jump_float(c: &mut LevelCtx) {
    let joy = frog_joy(c);
    let b = &mut c.bus;
    let air = b.w(FROG + f::AIR);
    if b.w(FROG + f::ON_GROUND) != 0
        || !n_clear(air.wrapping_sub(0xc))
        || n_clear(air.wrapping_sub(0x20))
    {
        b.set_w(FROG + f::FLOAT_TOGGLE, 0);
        return;
    }
    if n_clear(b.w(FROG + f::VY)) {
        return;
    }
    if !matches!(joy & 0x303, 0x100 | 0x103 | 0x200) {
        return;
    }
    let t = !b.w(FROG + f::FLOAT_TOGGLE);
    b.set_w(FROG + f::FLOAT_TOGGLE, t);
    if t == 0 {
        return;
    }
    b.set_w(FROG + f::VY, b.w(FROG + f::VY).wrapping_sub(1));
    if n_clear(b.w(FROG + f::AIR).wrapping_sub(0x1c)) {
        b.set_w(FROG + f::AIR, 0x20);
    }
}
