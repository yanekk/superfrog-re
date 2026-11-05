//! The character (plans/forest-dumb-level-1 T08, T51): its state and the original's routines that
//! change it, ported from `game_frame`'s character calls (docs/09-play-measured.md "The character's
//! update").
//!
//! T51 ports the walk animation's frame pickers ([`anim`]) and the joystick read ([`input`]); T08
//! builds `Character` on them and appends here.

pub mod anim;
pub mod input;

pub use anim::{Anim, AnimTables};
pub use input::{InputLog, JoyFlags, joy1dat};
