//! The frog (docs/09 "The frog"): its record, and the routines the vertical-blank interrupt runs
//! for it every frame while a level plays (docs/08 "Main loop and interrupt").
//!
//! The record is `frog` (h0+$7d6), one [`crate::LAYOUT`] entry per field. The routines are ported
//! one for one from the original and keep its names and order; each is proven against the
//! original on the routine oracle (`sf-conform/tests/frog_routines.rs`).

pub mod anim;
pub mod physics;

use crate::layout::h0;

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
    /// Frames in the air since leaving the ground, counted from 1 by the ground probe; 0 on the
    /// ground (inferred).
    pub const AIR: u32 = 0x0a;
    /// Set for a frame when the frog's head hits a ceiling (h0+$e4fc); `frog_tiles` clears it.
    pub const HEAD_BUMP: u32 = 0x1e;
    /// Counts frames for gravity: vy grows by one each time it passes [`GRAVITY_DELAY`] (or 3).
    pub const GRAVITY_TICK: u32 = 0x0c;
    /// The animation playing (a pointer to its list of steps).
    pub const ANIM: u32 = 0x2c;
    /// Set by a wall on the right (h0+$f2da) or a `$3d` edge: the frog stopped there.
    pub const WALL_RIGHT: u32 = 0x0e;
    /// Set by a wall on the left (h0+$f334) or a `$3d` edge.
    pub const WALL_LEFT: u32 = 0x10;
    /// Facing: negative while the frog faces left (inferred from the camera's use).
    pub const FACING: u32 = 0x12;
    /// Negative while the frog climbs (inferred from the camera's use).
    pub const CLIMB: u32 = 0x14;
    /// Frames a spring or launcher holds the frog's animation (set to `$18`/`$1e`).
    pub const LAUNCHED: u32 = 0x2a;
    /// The current frame's sprite offset from the frog: x, y (words; written as a long).
    pub const SPRITE_DX: u32 = 0x36;
    pub const SPRITE_DY: u32 = 0x38;
    /// Hurt: non-zero while the frog is hurt or dying and may not be hurt again (also read as a
    /// long with `DYING`).
    pub const HURT: u32 = 0x3a;
    /// Dying.
    pub const DYING: u32 = 0x3e;
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
    /// The spud may be thrown (docs/09).
    pub const SPUD_ON: u32 = 0x7c;
    /// Set while a spring carries the frog: the frog's sprites drop below the play field.
    pub const SPRING: u32 = 0x84;
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
    /// The speed a bounce pad gives (vy, h0+$11626).
    pub const BOUNCE_VY: u32 = 0x98;
    /// Frames between gravity steps in the air.
    pub const GRAVITY_DELAY: u32 = 0x9a;
    /// Toggles every frame of a held jump ([`super::anim::frog_jump_float`]).
    pub const FLOAT_TOGGLE: u32 = 0xa4;
}

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
