//! The game-free parts of a desktop app that plays an Amiga game: sound to the speakers and the
//! sample clock that paces the game ([`audio`], [`pacing`]), gamepads ([`pad`]), the Amiga key
//! table and which host key feeds which input ([`keys`]), the picture on the GPU with its CRT
//! filters ([`video`]), what a run captures for its tests ([`capture`]), files written whole
//! ([`atomic_write`]) and the single-copy lock ([`lock`]).
//!
//! A shell crate: it reads devices, the clock and files. The run loop, its script, menus and
//! windows are the game's own; a game calls these from them.

pub mod atomic_write;
pub mod audio;
pub mod capture;
pub mod keys;
pub mod lock;
pub mod pacing;
pub mod pad;
pub mod video;
