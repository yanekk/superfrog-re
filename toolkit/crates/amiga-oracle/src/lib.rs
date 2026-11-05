//! Shell, test tooling: the vAmiga full-system reference. Never a dependency of a shipped game: a
//! game may not contain an emulator (each game's purity test enforces it).
//!
//! [`Amiga`] runs an original game headless, one PAL frame per [`Amiga::step`], with the same
//! [`FrameInput`] an engine takes, and returns what the frame drew and wrote to Paula
//! ([`FrameCapture`]). It takes a Kickstart and any list of disks, and may power on with df0 empty
//! ([`OracleConfig::boot`]). [`cached_trace`] runs a scripted stretch and memoises it outside git.
//! [`Cpu68k`] runs one routine of the original alone on Moira, for routine conformance.
//!
//! The core is built by `toolkit/tools/setup/oracle.sh`; its pinned commit is `VAMIGA_COMMIT` and
//! its patches are in `patches/`. A game keeps its own disks, their names and its cache roots, and
//! passes them in.

mod amiga;
mod config;
mod ffi;
mod routine;
mod trace;

pub use amiga::{
    Amiga, AmigaMemory, AudioTap, BlitEvent, BlitEventKind, CpuRegs, CpuWrite, FrameCapture, Irq,
    LineSample, MIX_RATE, Rgb12Frame, SLOW_BASE, Snapshot, hash_memory,
};
pub use amiga_input::{FrameInput, Joy, KeySet};
pub use amiga_paula::PaulaWrite;
pub use config::{
    DEFAULT_MAX_FRAMES, OracleConfig, OracleError, default_cache, kickstart_path, read_only,
};
pub use routine::{
    BranchWays, CallError, CallResult, CallTrace, Cpu68k, Gen, Hunk, HunkError, HunkKind, HunkMap,
    MEM_SIZE, RETURN_SENTINEL, Regs,
};
pub use trace::{
    CacheOutcome, InputScript, SyncPoint, Trace, TraceFrame, cached_trace, cached_trace_in,
    trace_key,
};

/// The vAmiga build this crate links and how the shim sets it up: `<commit>-<patch-set
/// hash>-<settings>`. Part of every trace and sync cache key. The settings tag names emulation
/// options the shim sets that change what a run computes (`fixedSettings` in `shim/shim.cpp`);
/// change it with them, so a snapshot or trace taken under the old settings is never reused.
/// `clx`: Denise's three collision checks on.
pub const BUILD: &str = concat!(env!("AMIGA_ORACLE_BUILD"), "-clx");
