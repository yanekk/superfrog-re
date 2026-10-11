//! Shell, test tooling: the vAmiga full-system reference (DESIGN §3.2). Never a dependency of
//! `sf-app`: the shipped game may not contain an emulator (enforced by sf-conform's purity test).
//!
//! [`Amiga`] runs the original game headless, one PAL frame per [`Amiga::step`], with the same
//! [`FrameInput`] the engine takes, and returns what the frame drew and wrote to Paula
//! ([`FrameCapture`]). [`cached_trace`] runs a scripted stretch and memoises it outside git.
//!
//! The core is built by `tools/setup/oracle.sh`; its pinned commit is `VAMIGA_COMMIT` and its
//! patches are in `patches/` (FINDINGS 2026-10-11 has why each is needed).

mod amiga;
mod config;
mod ffi;
mod trace;

pub use amiga::{Amiga, AmigaMemory, AudioTap, FrameCapture, MIX_RATE, Rgb12Frame, Snapshot};
pub use config::{DEFAULT_MAX_FRAMES, OracleConfig, OracleError};
pub use sf_core::input::{FrameInput, Joy, KeySet};
pub use sf_disk::DiskSlot;
pub use sf_paula::PaulaWrite;
pub use trace::{
    CacheOutcome, InputScript, SyncPoint, Trace, TraceFrame, cached_trace, cached_trace_in,
    trace_key,
};

/// The vAmiga build this crate links: `<commit>-<patch-set hash>`. Part of every trace cache key.
pub const BUILD: &str = env!("SF_ORACLE_BUILD");
