//! The frog's two hardware sprites: where they show (h0+$19bbe) and the copper words that put
//! them there (h0+$1b23e).

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::camera::{FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT, SHOOTER_MODE};
use crate::level::collision::{LEFT_ANIMS, n_clear_l};
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

/// `frog_anim_lock` is the frog's `ANIM_LOCK` field (h0+$86a), which the routines name by address.
fn locked(b: &Bus) -> bool {
    b.w(FROG + f::ANIM_LOCK) != 0
}

/// `frog_set_anim` (h0+$102e6): start animation `a1` from its first step, unless locked or
/// already playing.
pub fn frog_set_anim(c: &mut LevelCtx, a1: u32) {
    if locked(&c.bus) {
        return;
    }
    set_anim(&mut c.bus, a1);
}

/// h0+$102f0, the body of [`frog_set_anim`] without the lock.
pub fn set_anim(b: &mut Bus, a1: u32) {
    if b.l(FROG + f::ANIM) == a1 {
        return;
    }
    b.set_l(FROG + f::ANIM, a1);
    b.set_l(FROG + f::ANIM_STEP, a1);
    b.set_w(FROG + f::ANIM_TIME, 0xffff);
}

/// `frog_set_anim_free` (h0+$1030e): [`frog_set_anim`], unless a special animation plays or the
/// frog is hurt or dying.
pub fn frog_set_anim_free(c: &mut LevelCtx, a1: u32) {
    let b = &mut c.bus;
    if locked(b) || b.w(FROG + f::SPECIAL_ANIM) != 0 || b.l(FROG + f::HURT) != 0 {
        return;
    }
    set_anim(b, a1);
}

/// `frog_set_anim_at` (h0+$1034c): start animation `a1` at step `a2`, unless locked, hurt or
/// already playing.
pub fn frog_set_anim_at(c: &mut LevelCtx, a1: u32, a2: u32) {
    let b = &mut c.bus;
    if locked(b) || b.l(FROG + f::HURT) != 0 || b.l(FROG + f::ANIM) == a1 {
        return;
    }
    b.set_l(FROG + f::ANIM, a1);
    b.set_l(FROG + f::ANIM_STEP, a2);
    b.set_w(FROG + f::ANIM_TIME, 0xffff);
}

/// `frog_exit_sprites` (h0+$9f8): non-zero, the frog shows in the second pair of sprite records
/// (the level-end sequence).
pub const EXIT_SPRITES: u32 = h0(0x9f8);
/// The second pair (h0+$1b02c, h0+$1b048).
pub const EXIT_SPRITE_LEFT: u32 = h0(0x1b02c);
pub const EXIT_SPRITE_RIGHT: u32 = h0(0x1b048);

/// `frog_anim_tick` (h0+$10222): the animation's step runs out, the next one (the list starts
/// again at its end) gives the frame: its sprite offset, its height, and the data pointers the
/// copper list's sprite moves get, the two halves of each attached pair one sprite apart.
pub fn frog_anim_tick(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let time = b.w(FROG + f::ANIM_TIME);
    if time != 0xffff {
        let time = time.wrapping_sub(1);
        b.set_w(FROG + f::ANIM_TIME, time);
        if n_clear(time) {
            return;
        }
        b.set_l(FROG + f::ANIM_STEP, b.l(FROG + f::ANIM_STEP).wrapping_add(8));
    }
    let mut step = b.l(FROG + f::ANIM_STEP);
    if (b.l(step) as i32) < 0 {
        step = b.l(FROG + f::ANIM);
        b.set_l(FROG + f::ANIM_STEP, step);
    }
    b.set_w(FROG + f::ANIM_TIME, b.w(step.wrapping_add(6)));
    let frame = b.l(step);
    b.set_l(FROG + f::SPRITE_DX, b.l(frame.wrapping_add(4)));
    let (a5, a6) = if b.w(EXIT_SPRITES) != 0 {
        (EXIT_SPRITE_LEFT, EXIT_SPRITE_RIGHT)
    } else {
        (FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT)
    };
    let height = b.w(frame.wrapping_add(8));
    b.set_w(a5 + spr::HEIGHT, height);
    b.set_w(a6 + spr::HEIGHT, height);
    // The data pointer of each sprite: (height + 1) longs apart (control words and data).
    let size = (height as i16 as i32 as u32) & 0xffff_0000
        | (height.wrapping_add(1).wrapping_shl(2)) as u32;
    let mut data = b.l(frame);
    b.set_l(a5 + 0xc, data);
    for s in [a5, a6] {
        if s == a6 {
            b.set_l(a6 + 0xc, data);
        }
        let cop = b.l(s + spr::COPPER);
        for half in [0u32, 0x10] {
            b.set_w(cop + half + 6, data as u16);
            b.set_w(cop + half + 2, (data >> 16) as u16);
            data = data.wrapping_add(size);
        }
    }
}

/// `fall_history` (h0+$10182..$10190): a hurt frog's y on each of the last eight frames, newest
/// first.
const FALL_HISTORY: [u32; 8] = [
    0x10182, 0x10184, 0x10186, 0x10188, 0x1018a, 0x1018c, 0x1018e, 0x10190,
];

/// `frog_input_anim` (h0+$fe34): the animation the joystick and the frog's motion call for, and
/// the hurt and dying animations, which end in the blink or the death.
pub fn frog_input_anim(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.l(FROG + f::HURT) != 0 {
        return hurt_anim(c);
    }
    let d7 = c.hw.joy1dat & 0x303;
    let b = &mut c.bus;
    let facing = b.w(FROG + f::FACING) != 0;
    if b.w(FROG + f::AIR) != 0 {
        let mut d0 = b.w(FROG + f::VY);
        if n_clear(d0.wrapping_sub(9)) {
            d0 = 8;
        }
        if !n_clear(d0.wrapping_add(8)) {
            d0 = 0xfff8;
        }
        let d0 = d0.wrapping_neg();
        if n_clear(d0) {
            let table = match (b.w(FROG + f::VX) != 0, facing) {
                (true, false) => h0(0x101da),
                (true, true) => h0(0x101fe),
                (false, false) => h0(0x101b6),
                (false, true) => h0(0x10192),
            };
            let a1 = b.l(table + 4 * d0 as u32);
            return frog_set_anim_free(c, a1);
        }
    } else {
        if !n_clear(b.w(FROG + f::VY).wrapping_sub(3)) || b.w(FROG + f::ON_GROUND) != 0 {
            return;
        }
        if (b.w(FROG + f::SLIDING) != 0 || b.w(FROG + f::PUSHED) != 0) && b.w(FROG + f::CLIMB) == 0
        {
            return;
        }
    }
    // h0+$feda.
    let a1 = if d7 == 0 && b.w(FROG + f::VX) == 0 {
        if facing { h0(0x10542) } else { h0(0x10d0e) }
    } else if n_clear(b.w(FROG + f::VY).wrapping_sub(6)) && {
        let cell = crate::level::collision::cell_at(
            b,
            b.w(FROG + f::X).wrapping_add(8),
            b.w(FROG + f::Y),
        )
        .wrapping_add(0x982);
        crate::level::collision::marker(b, cell) >= 0x3a
            || crate::level::collision::marker(b, cell.wrapping_add(0x260)) >= 0x3a
    } {
        if facing { h0(0x10566) } else { h0(0x10d32) }
    } else if facing {
        h0(0x10572)
    } else {
        h0(0x10d3e)
    };
    frog_set_anim_free(c, a1);
}

/// h0+$ff7c: hurt and dying. When the animation reaches its end mark (`ANIM_TIME` `$4e20`) the
/// frog blinks, or is dead; until then a frog that has stopped falling plays its landing sound.
fn hurt_anim(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let low = !n_clear_l(b.l(FROG + f::ANIM).wrapping_sub(LEFT_ANIMS));
    let a1 = if b.w(FROG + f::DYING) != 0 {
        if b.w(SHOOTER_MODE) != 0 {
            if low { h0(0x10646) } else { h0(0x10e1e) }
        } else if low {
            h0(0x1063e)
        } else {
            h0(0x10e16)
        }
    } else if low {
        h0(0x1077a)
    } else {
        h0(0x10f52)
    };
    frog_set_anim(c, a1);
    let b = &mut c.bus;
    if n_clear(b.w(FROG + f::ANIM_TIME).wrapping_sub(0x4e20)) {
        b.set_w(FROG + f::HURT, 0);
        b.set_w(FROG + f::LANDED, 0);
        b.set_w(FROG + f::BLINK, 0x4b);
        if b.w(OPT_GAME_MODE) == 0 {
            b.set_w(FROG + f::BLINK, 0x96);
        }
        if b.w(FROG + f::DYING) != 0 {
            b.set_w(PLAYER_DIED, 1);
        }
        let a1 = if n_clear_l(b.l(FROG + f::ANIM).wrapping_sub(LEFT_ANIMS)) {
            h0(0x10492)
        } else {
            h0(0x10c9a)
        };
        b.set_l(FROG + f::ANIM, a1);
        return;
    }
    let falls = [h0(0x1063e), h0(0x10e16), h0(0x1077a), h0(0x10f52)];
    let (step, anim) = (b.l(FROG + f::ANIM_STEP), b.l(FROG + f::ANIM));
    let falling = [
        (step, 0),
        (step, 1),
        (anim, 0),
        (anim, 1),
        (step, 2),
        (step, 3),
        (anim, 2),
        (anim, 3),
    ]
    .iter()
    .any(|&(v, k)| v == falls[k]);
    let y = b.w(FROG + f::Y);
    if falling
        && n_clear(b.w(FROG + f::ANIM_TIME).wrapping_sub(0x11))
        && FALL_HISTORY.iter().all(|&a| b.w(h0(a)) == y)
    {
        return landed(c);
    }
    // h0+$100c0: the history moves on.
    for k in (1..8).rev() {
        b.set_w(h0(FALL_HISTORY[k]), b.w(h0(FALL_HISTORY[k - 1])));
    }
    b.set_w(h0(FALL_HISTORY[0]), y);
    if b.w(FROG + f::DYING) == 0 && b.w(FROG + f::LANDED) != 0 {
        return;
    }
    if !n_clear(b.w(FROG + f::VY)) {
        return;
    }
    let cell = crate::level::collision::cell_at(b, b.w(FROG + f::X).wrapping_add(8), y)
        .wrapping_add(0x982);
    if crate::level::collision::marker(b, cell) < 0x3a {
        return;
    }
    landed(c);
}

/// h0+$1015c: a hurt frog has landed: the animation runs on, and a hurt (not dying) frog thuds.
fn landed(c: &mut LevelCtx) {
    let b = &mut c.bus;
    b.set_w(FROG + f::ANIM_TIME, 0);
    b.set_w(FROG + f::LANDED, 1);
    if b.w(FROG + f::DYING) == 0 {
        c.sfx(0xe, 3);
    }
}

/// `opt_game_mode` (h0+$9e6): 0 easy (the blink after a hit lasts twice as long).
const OPT_GAME_MODE: u32 = h0(0x9e6);
/// `player_died` (h0+$82c): the frog's `+$56`.
const PLAYER_DIED: u32 = h0(0x82c);

/// `entry_fall` (h0+$6c92): how fast the frog's sprites drop at a level's start, up to 8.
const ENTRY_FALL: u32 = h0(0x6c92);
/// `sprites_hidden` (h0+$9e2): set while the frog's entry runs (the camera leaves its sprites).
const ENTRY: u32 = h0(0x9e2);

/// `frog_level_entry` (h0+$6bd4): at a level's start the frog drops in from above the screen
/// (h0+$6b22 sets it up): its animation runs, and once it reaches its falling step the sprites
/// fall faster each frame until they reach the frog's place, which ends the entry. In the bonus
/// level it flies in from the left instead.
pub fn frog_level_entry(c: &mut LevelCtx) {
    if c.bus.w(ENTRY) == 0 {
        return;
    }
    frog_anim_tick(c);
    let b = &mut c.bus;
    let (cx, cy) = (
        b.w(crate::level::camera::CAMERA_X + 2),
        b.w(crate::level::camera::CAMERA_Y + 2),
    );
    if b.w(SHOOTER_MODE) != 0 {
        // h0+$6b86.
        b.set_w(FROG + f::X, cx.wrapping_add(0x80));
        b.set_w(FROG + f::Y, cy.wrapping_add(0x5c));
        for s in [FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT] {
            b.set_w(s + spr::X, b.w(s + spr::X).wrapping_add(4));
        }
        place_frog_sprites(c);
        let b = &mut c.bus;
        if n_clear(b.w(FROG_SPRITE_LEFT + spr::X).wrapping_sub(0x80)) {
            b.set_w(FROG + f::BLINK, 0x64);
            b.set_w(ENTRY, 0);
        }
        return;
    }
    if n_clear(b.w(FROG + f::ANIM_TIME).wrapping_sub(0x1f40)) {
        if !n_clear(b.w(ENTRY_FALL).wrapping_sub(8)) {
            b.set_w(ENTRY_FALL, b.w(ENTRY_FALL).wrapping_add(1));
        }
        let fall = b.w(ENTRY_FALL);
        for s in [FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT] {
            b.set_w(s + spr::Y, b.w(s + spr::Y).wrapping_add(fall));
        }
    }
    let mut ground = 0x70u16;
    if b.w(crate::level::camera::FINAL_LEVEL_FLAG) != 0 {
        ground = ground.wrapping_add(0x50);
    }
    if n_clear(ground.wrapping_sub(b.w(FROG_SPRITE_LEFT + spr::Y))) {
        return place_frog_sprites(c);
    }
    // h0+$6c3a: landed.
    b.set_w(FROG + f::ANIM_TIME, 0);
    b.set_w(FROG + f::SPRITE_DY, 0);
    b.set_w(FROG + f::Y, b.w(FROG_SPRITE_LEFT + spr::Y).wrapping_add(cy));
    let stunned = b.w(h0(0x1c)) != 0 || b.w(h0(0x12)) == 0;
    if stunned {
        b.set_l(FROG + f::ANIM, h0(0x1077a));
    }
    b.set_w(FROG + f::FACING, 0xffff);
    if stunned {
        b.set_w(FROG + f::HURT, 1);
    }
    b.set_w(ENTRY, 0);
}

/// `frog_exit_walk` (h0+$11e62): the frog's walk across the end-of-world picture, a state
/// machine through code addresses (h0+$11e92) while `frog_exit_sprites` is set. That screen is
/// the game flow's (T21); in a level the flag is clear and this does nothing.
pub fn frog_exit_walk(c: &mut LevelCtx) {
    debug_assert_eq!(
        c.bus.w(EXIT_SPRITES),
        0,
        "the end-of-world walk is not ported (T21)"
    );
}
