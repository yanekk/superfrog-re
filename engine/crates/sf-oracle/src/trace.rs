//! Superfrog's scripted oracle runs: the toolkit's trace cache (`amiga_oracle::cached_trace_in`)
//! under `~/.cache/superfrog-re/traces/`, with the keys this crate wrote before the move.

use std::path::Path;

use amiga_oracle::{CacheOutcome, InputScript, OracleError, SyncPoint, Trace};

use crate::config::OracleConfig;

/// [`cached_trace_in`] on [`crate::trace_cache`].
pub fn cached_trace(
    cfg: &OracleConfig,
    start: &SyncPoint,
    script: &InputScript,
    frames: u32,
) -> Result<Trace, OracleError> {
    amiga_oracle::cached_trace(&cfg.generic()?, start, script, frames)
}

/// Run `frames` frames of `script` from `start`, or read the same run from `dir`.
pub fn cached_trace_in(
    dir: &Path,
    cfg: &OracleConfig,
    start: &SyncPoint,
    script: &InputScript,
    frames: u32,
) -> Result<(Trace, CacheOutcome), OracleError> {
    amiga_oracle::cached_trace_in(dir, &cfg.generic()?, start, script, frames)
}
