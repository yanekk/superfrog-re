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
    /// Non-zero: the frog does not move ([`super::physics::frog_move`]).
    pub const FROZEN: u32 = 0x88;
}
