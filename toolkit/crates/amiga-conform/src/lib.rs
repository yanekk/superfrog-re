//! Shell, test tooling: the game-free half of a conformance harness, which judges an engine against
//! the original run on the oracle (`amiga-oracle`).
//!
//! - [`compare`]: a frame's picture and Paula writes against the original's, and the owner's
//!   faithful bar over a case ([`Mode`], [`Tolerance`], [`compare::PictureJudge`],
//!   [`compare::PassMatch`]).
//! - [`sound_events`]: Paula writes reduced to what a listener hears, and matched within the
//!   tolerance.
//! - [`channels`]: each channel's output reduced to a level per frame, and judged.
//! - [`feed`]: how far a frame of the original's mix swings.
//! - [`report`]: the diff picture beside a divergence.
//! - [`script`]: input scripts, with a hook for a game's own tokens.
//!
//! A game keeps its cases, its memory map, its levels and milestones, and the run loop that drives
//! both sides; it passes in what these read (excluded rows, file headers, an output folder).

pub mod channels;
pub mod compare;
pub mod feed;
pub mod report;
pub mod script;
pub mod sound_events;

pub use compare::{Mode, Tolerance};
pub use sound_events::{SoundEvent, sound_events};
