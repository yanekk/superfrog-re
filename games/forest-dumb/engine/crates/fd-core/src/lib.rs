//! Forest Dumb Forever's engine core, so far its smallest real port: the game's measured timing and
//! a placeholder scene for the idle title screen, whose one modelled variable the first conformance
//! case compares with the original (plans/forest-dumb-start DESIGN §2.8, §3.3).
//!
//! Pure: everything arrives per frame as a parameter. No clock, file or network, and the only
//! dependencies are `amiga-clock` and `amiga-input`.

pub mod ported;
pub mod timing;
pub mod title;

pub use ported::{PORTED, Ported};
pub use timing::FdTiming;
pub use title::{LAYOUT, LAYOUT_HUNK, TitleScene};
