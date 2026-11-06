//! The character (plans/forest-dumb-level-1 T08, T51): its state and the original's routines that
//! change it, ported from `game_frame`'s character calls (docs/09-play-measured.md "The character's
//! update").
//!
//! T51 ports the walk animation's frame pickers ([`anim`]) and the joystick read ([`input`]); T08
//! the movement on them ([`physics`]). T23 the countdown, the keys and the lives ([`death`]).

pub mod anim;
pub mod death;
pub mod input;
pub mod physics;

pub use anim::{Anim, AnimTables};
pub use death::{Keys, Play};
pub use input::{InputLog, JoyFlags, joy1dat};
pub use physics::{Character, Motion};
