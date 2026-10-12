//! The frog (docs/09 "The frog"): its record, and the routines the vertical-blank interrupt runs
//! for it every frame while a level plays (docs/08 "Main loop and interrupt").
//!
//! The record is `frog` (h0+$7d6), one [`crate::LAYOUT`] entry per field. The routines are ported
//! one for one from the original and keep its names and order; each is proven against the
//! original on the routine oracle (`sf-conform/tests/frog_routines.rs`).

pub mod anim;
pub mod physics;

use crate::layout::h0;
use crate::mem::Bus;

/// The frog's record.
pub const FROG: u32 = h0(0x7d6);

/// Field offsets in the frog's record.
pub mod f {
    /// Level x and y of the frog's top-left, in pixels.
    pub const X: u32 = 0x00;
    pub const Y: u32 = 0x02;
    /// Velocity, pixels per frame.
    pub const VX: u32 = 0x04;
    pub const VY: u32 = 0x06;
    /// The current walking speed limit (it grows a step at a time to `MAX_SPEED`).
    pub const SPEED: u32 = 0x08;
    /// Frames in the air since leaving the ground, counted from 1 by the ground probe; 0 on the
    /// ground (inferred).
    pub const AIR: u32 = 0x0a;
    /// Set for a frame when the frog's head hits a ceiling (h0+$e4fc); `frog_tiles` clears it.
    pub const HEAD_BUMP: u32 = 0x1e;
    /// Counts frames for gravity: vy grows by one each time it passes [`GRAVITY_DELAY`] (or 3).
    pub const GRAVITY_TICK: u32 = 0x0c;
    /// The animation playing (a pointer to its list of steps: 8 bytes each, a frame pointer
    /// (negative ends the list, which starts again) and frames to show it at `+6`).
    pub const ANIM: u32 = 0x2c;
    /// The step showing.
    pub const ANIM_STEP: u32 = 0x30;
    /// Frames left on the step; `$ffff` starts the animation at its first step.
    pub const ANIM_TIME: u32 = 0x34;
    /// Set by a wall on the right (h0+$f2da) or a `$3d` edge: the frog stopped there.
    pub const WALL_RIGHT: u32 = 0x0e;
    /// Set by a wall on the left (h0+$f334) or a `$3d` edge.
    pub const WALL_LEFT: u32 = 0x10;
    /// Facing: negative while the frog faces left (inferred from the camera's use).
    pub const FACING: u32 = 0x12;
    /// -1 while the frog moves down (its y grew since the last frame), 0 while it moves up.
    pub const CLIMB: u32 = 0x14;
    /// The frog's position the frame before (x, y; written as a long).
    pub const PREV_X: u32 = 0x18;
    pub const PREV_Y: u32 = 0x1a;
    /// Frames in the air between walking speed steps (h0+$1175e).
    pub const AIR_STEP: u32 = 0x1c;
    /// Counts to 4: the walking speed limit rises or falls a step each time.
    pub const SPEED_TICK: u32 = 0x22;
    /// Frames a spring or launcher holds the frog's animation (set to `$18`/`$1e`).
    pub const LAUNCHED: u32 = 0x2a;
    /// The current frame's sprite offset from the frog: x, y (words; written as a long).
    pub const SPRITE_DX: u32 = 0x36;
    pub const SPRITE_DY: u32 = 0x38;
    /// Hurt: non-zero while the frog is hurt or dying and may not be hurt again (also read as a
    /// long with `DYING`).
    pub const HURT: u32 = 0x3a;
    /// Set once a hurt or dying frog has landed (its fall stopped).
    pub const LANDED: u32 = 0x3c;
    /// Dying.
    pub const DYING: u32 = 0x3e;
    /// Toggles: friction acts every second frame; the walking speed rises every second frame.
    pub const FRICTION_TOGGLE: u32 = 0x40;
    pub const RIGHT_TOGGLE: u32 = 0x42;
    pub const LEFT_TOGGLE: u32 = 0x44;
    /// Frames between friction steps while hurt.
    pub const HURT_FRICTION: u32 = 0x46;
    /// Non-zero while the frog stands on the ground.
    pub const ON_GROUND: u32 = 0x24;
    /// Which side the slope probe last stepped to: 1 left, 2 right, 0 neither.
    pub const SLOPE_SIDE: u32 = 0x28;
    /// Non-zero: the camera stays where it is.
    pub const CAMERA_HELD: u32 = 0x48;
    /// The ground under the frog: the steepness its tile's heights give (h0+$de2a).
    pub const SLOPE: u32 = 0x4a;
    /// The animation the slope picks (a pointer into the frog's animation tables).
    pub const SLOPE_ANIM: u32 = 0x4c;
    /// The map cell the frog stands on (a pointer into `level_map`).
    pub const GROUND_CELL: u32 = 0x50;
    /// Non-zero while the frog rides a platform.
    pub const ON_PLATFORM: u32 = 0x54;
    /// Lives.
    pub const LIVES: u32 = 0x58;
    /// Coins still needed before the exit opens (`coins_left_for_exit`).
    pub const COINS_LEFT: u32 = 0x5c;
    /// Set while the frog is on a slippery or pushing surface this frame (h0+$eaae, $eb64).
    pub const SLIDING: u32 = 0x6a;
    /// Frames on the slippery surface, wrapping.
    pub const SLIDE_TICK: u32 = 0x6c;
    /// Set while on a pushing surface this frame.
    pub const PUSHED: u32 = 0x6e;
    /// Set while fire was held in the air (the jump sound played).
    pub const FIRE_HELD: u32 = 0x64;
    /// The spud may be thrown (docs/09).
    pub const SPUD_ON: u32 = 0x7c;
    /// The walking speed limit on the ground; it falls back to `BASE_SPEED` when a speed-up
    /// runs out (`SPEED_UP`).
    pub const MAX_SPEED: u32 = 0x82;
    /// Frames the frog's sitting sound repeats after (h0+$11056).
    pub const SIT_TIME: u32 = 0x86;
    /// Set while a spring carries the frog: the frog's sprites drop below the play field.
    pub const SPRING: u32 = 0x84;
    /// Set while the frog crouches (down held); `CROUCH_TIME` counts 5 frames after.
    pub const CROUCH: u32 = 0x66;
    pub const CROUCH_TIME: u32 = 0x70;
    /// A special animation by index into h0+$11036 / h0+$11046 (0 none).
    pub const SPECIAL_ANIM: u32 = 0x80;
    /// Set while fire may start a flight (h0+$11056).
    pub const FLIGHT: u32 = 0x90;
    /// Non-zero: the frog does not move ([`super::physics::frog_move`]).
    pub const FROZEN: u32 = 0x88;
    /// Set while the frog goes through a secret passage (marker 38, h0+$ea78).
    pub const IN_PASSAGE: u32 = 0x8a;
    /// Frames of blinking left after a hit; bit 0 hides the sprites.
    pub const BLINK: u32 = 0x8c;
    /// Frames a launcher holds the frog (marker 7).
    pub const LAUNCH_HOLD: u32 = 0x8e;
    /// A shield: non-zero, hazards do not hurt.
    pub const SHIELD: u32 = 0x92;
    /// Frames left with the animation locked ([`super::anim::frog_set_anim`] does nothing).
    pub const ANIM_LOCK: u32 = 0x94;
    /// Frames a sound of the frog sitting on a pushing surface waits (h0+$11056).
    pub const IDLE_SOUND: u32 = 0x96;
    /// The speed a bounce pad gives (vy, h0+$11626).
    pub const BOUNCE_VY: u32 = 0x98;
    /// Frames between gravity steps in the air.
    pub const GRAVITY_DELAY: u32 = 0x9a;
    /// Frames between walking speed steps in the air.
    pub const AIR_STEP_DELAY: u32 = 0x9c;
    /// Extra walking speed in the air.
    pub const AIR_SPEED: u32 = 0x9e;
    /// Jump presses counted while in the air (h0+$11056).
    pub const JUMP_PRESSES: u32 = 0xa6;
    /// Frames a speed-up lasts; negative when none.
    pub const SPEED_UP: u32 = 0xa8;
    /// The base walking speed limit: in the air, and what `MAX_SPEED` returns to.
    pub const BASE_SPEED: u32 = 0xb0;
    /// Toggles every frame of a held jump ([`super::anim::frog_jump_float`]).
    pub const FLOAT_TOGGLE: u32 = 0xa4;
}

/// `opt_lives` (h0+$9e4), `opt_game_mode` (h0+$9e6): the options screen's choices.
const OPT_LIVES: u32 = h0(0x9e4);
const OPT_GAME_MODE: u32 = h0(0x9e6);
/// `coins_collected` (h0+$445e): coins taken since the last tally (`touch_coin` counts them).
const COINS_COLLECTED: u32 = h0(0x445e);

/// `frog_new_game` (h0+$8f90): the frog's record for a new game, as `title_screen` leaves it
/// (h0+$11ec): all `$200` bytes cleared, the lives the options give, the score and the coins
/// cleared, the game mode's tuning, then [`frog_reset`].
pub fn frog_new_game(b: &mut Bus) {
    for k in 0..0x80 {
        b.set_l(FROG + 4 * k, 0);
    }
    let lives = match b.w(OPT_LIVES) {
        0 => 2,
        1 => 4,
        _ => 6,
    };
    b.set_w(FROG + 0x58, lives);
    b.set_w(FROG + 0x7e, 0);
    b.set_w(FROG + 0x90, 0);
    b.set_w(FROG + f::SPUD_ON, 0);
    b.set_l(FROG + 0xa0, 0);
    b.set_l(FROG + 0x5e, 0);
    b.set_l(FROG + 0xaa, 200_000);
    b.set_l(COINS_COLLECTED, 0);
    let (tuning, base) = if b.w(OPT_GAME_MODE) == 0 {
        ([0xfffb, 6, 2, 0xfffe], 5)
    } else {
        ([0xfffa, 4, 1, 0xfffe], 6)
    };
    for (k, w) in tuning.into_iter().enumerate() {
        b.set_w(FROG + f::BOUNCE_VY + 2 * k as u32, w);
    }
    b.set_w(FROG + f::BASE_SPEED, base);
    frog_reset(b);
}

/// `frog_reset` (h0+$902e): the frog's record at a level's (re)start: still, the walking speed
/// limit at 7, the standing animation from its first step, no speed-up.
pub fn frog_reset(b: &mut Bus) {
    const CLEARED: [u32; 57] = [
        0x04, 0x06, 0x0a, 0x0c, 0x0e, 0x10, 0x12, 0x14, 0x16, 0x18, 0x1a, 0x1c, 0x1e, 0x20,
        0x22, 0x24, 0x28, 0x2a, 0x36, 0x38, 0x3a, 0x3c, 0x3e, 0x40, 0x42, 0x44, 0x46, 0x48,
        0x4a, 0x4c, 0x54, 0x56, 0x62, 0x64, 0x66, 0x68, 0x6a, 0x6c, 0x6e, 0x70, 0x72, 0x74,
        0x76, 0x78, 0x7a, 0x80, 0x84, 0x86, 0x88, 0x8a, 0x8c, 0x92, 0x94, 0x96, 0xa4, 0xa6,
        0xae,
    ];
    b.set_w(FROG + f::SPEED, 7);
    b.set_w(FROG + 0x5a, 4);
    for o in CLEARED {
        b.set_w(FROG + o, 0);
    }
    b.set_l(FROG + f::ANIM, STAND_ANIM);
    b.set_l(FROG + f::ANIM_STEP, STAND_ANIM);
    b.set_w(FROG + f::ANIM_TIME, 0xffff);
    b.set_w(FROG + 0x8e, 0xffff);
    b.set_w(FROG + f::SPEED_UP, 0xffff);
    b.set_w(FROG + 0x82, b.w(FROG + f::BASE_SPEED));
}

/// The standing animation `frog_reset` starts (h0+$10d8e).
const STAND_ANIM: u32 = h0(0x10d8e);

/// `frog_life_reset` (h0+$9148): what losing a life resets.
pub fn frog_life_reset(c: &mut crate::level::LevelCtx) {
    let b = &mut c.bus;
    b.set_w(FROG + 0x90, 0);
    b.set_w(FROG + f::SPUD_ON, 0);
    b.set_w(FROG + 0xa8, 0xffff);
    b.set_w(FROG + 0x82, b.w(FROG + 0xb0));
    if b.w(crate::level::camera::FINAL_LEVEL_FLAG) != 0 {
        b.set_w(FROG + f::SPUD_ON, 1);
    }
}
