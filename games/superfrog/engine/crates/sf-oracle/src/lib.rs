//! Shell, test tooling: the vAmiga full-system reference for Superfrog (DESIGN §3.2). Never a
//! dependency of `sf-app`: the shipped game may not contain an emulator (enforced by sf-conform's
//! purity test).
//!
//! The emulator, the routine runner and the trace cache are the toolkit's `amiga-oracle`
//! (toolkit-extract T05), re-exported here. What stays is Superfrog's: [`OracleConfig::locate`]
//! finds the owner's Kickstart and four disks, [`Amiga`] names the disks by [`DiskSlot`], and
//! traces stay under `~/.cache/superfrog-re/` with the keys they always had.
//!
//! The core is built by `toolkit/tools/setup/oracle.sh` (which runs `toolkit/tools/setup/oracle.sh`).

mod amiga;
mod config;
mod trace;

pub use amiga::Amiga;
pub use amiga_oracle::{
    AmigaMemory, AudioTap, BUILD, BlitEvent, BlitEventKind, CacheOutcome, CallError, CallResult,
    Cpu68k, CpuRegs, CpuWrite, DEFAULT_MAX_FRAMES, FrameCapture, FrameInput, Hunk, HunkError,
    HunkKind, HunkMap, InputScript, Irq, Joy, KeySet, LineSample, MEM_SIZE, MIX_RATE, OracleError,
    PaulaWrite, RETURN_SENTINEL, Regs, Rgb12Frame, Snapshot, SyncPoint, Trace, TraceFrame,
    hash_memory, trace_key,
};
pub use config::{OracleConfig, trace_cache};
pub use sf_disk::DiskSlot;
pub use trace::{cached_trace, cached_trace_in};
