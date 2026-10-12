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
    /// Facing: negative while the frog faces left (inferred from the camera's use).
    pub const FACING: u32 = 0x12;
    /// Negative while the frog climbs (inferred from the camera's use).
    pub const CLIMB: u32 = 0x14;
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
    /// Non-zero: the frog does not move ([`super::physics::frog_move`]).
    pub const FROZEN: u32 = 0x88;
}
