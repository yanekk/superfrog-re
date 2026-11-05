//! The game-free parts of a desktop app that plays an Amiga game: sound to the speakers and the
//! sample clock that paces the game ([`audio`], [`pacing`]), gamepads ([`pad`]), the Amiga key
//! table and which host key feeds which input ([`keys`]), the picture on the GPU with its CRT
//! filters ([`video`]), what a run captures for its tests ([`capture`]), files written whole
//! ([`atomic_write`]) and the single-copy lock ([`lock`]).
//!
//! The panels every game's app shares, each an AppKit-free model the tests drive and its native
//! view: `settings.toml` ([`settings`]), the Controls window ([`controls_window`]), Start Level
//! ([`start_level_window`], fields the game names), the bug recorder's bar ([`recording_bar`]) and
//! save window ([`save_window`]); and the game-free half of a `--script` ([`script_core`]).
//!
//! A shell crate: it reads devices, the clock and files. The run loop, the game window, its menus
//! and the glue that opens these panels are the game's own; a game calls these from them.

pub mod atomic_write;
pub mod audio;
pub mod capture;
pub mod controls_window;
pub mod keys;
pub mod lock;
pub mod pacing;
pub mod pad;
pub mod recording_bar;
pub mod save_window;
pub mod script_core;
pub mod settings;
pub mod start_level_window;
pub mod video;
