//! Shell side of the explorer (DESIGN §2.4, §3.1): a round's parts run on `std::thread::scope`, the
//! wall-clock budget is checked between rounds, peak resident memory is read from the OS. The core
//! decides everything else, so the route is the same for any thread count (M8).

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use sf_core::Game;

use crate::core::{Explorer, Facts, Options, PartResult, Progress, Reason, Route};

/// Parts per thread a round is cut into: bursts vary in length, so small parts taken in turn keep
/// the threads busy to the round's end.
const PARTS_PER_THREAD: usize = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct RunOptions {
    pub threads: usize,
    /// Wall-clock: the search stops between rounds once it is spent; a stopped run is not held to
    /// M8.
    pub budget: Duration,
    /// Shortening after the first exit (DESIGN §2.12, T22); the default keeps it off.
    pub shorten_share: f32,
    pub shorten_min_rounds: u64,
}

impl Default for RunOptions {
    fn default() -> RunOptions {
        RunOptions {
            threads: 1,
            budget: Duration::from_secs(30 * 60),
            shorten_share: 0.0,
            shorten_min_rounds: 0,
        }
    }
}

/// Plays one round on `threads` threads and merges it.
pub fn run_round(ex: &mut Explorer, threads: usize) -> Progress {
    let threads = threads.max(1);
    let parts = ex.round_parts(threads * PARTS_PER_THREAD);
    let results: Vec<Mutex<Option<PartResult>>> = parts.iter().map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let explorer = &*ex;
    std::thread::scope(|s| {
        for _ in 0..threads.min(parts.len()) {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(part) = parts.get(k) else { break };
                    *results[k].lock().unwrap() = Some(explorer.run_part(part));
                }
            });
        }
    });
    ex.merge(
        results
            .into_iter()
            .map(|m| m.into_inner().unwrap().unwrap())
            .collect(),
    )
}

/// The explorer from `start` to the goal, within `run`'s budget: the route, or the best partial
/// route and why (DESIGN §2.9).
pub fn plan_engine(
    start: Game,
    facts: Facts,
    opt: Options,
    run: &RunOptions,
) -> Result<Route, (Route, Reason)> {
    let t0 = Instant::now();
    let mut ex = Explorer::new(start, facts, opt);
    if let Some(r) = ex.found() {
        return Ok(r.clone());
    }
    while t0.elapsed() < run.budget {
        if let Progress::Exit(r) = run_round(&mut ex, run.threads) {
            return Ok(r);
        }
    }
    Err(ex.best_partial())
}

/// The process's peak resident memory in bytes (DESIGN §2.4's `peak_mem_mb`).
pub fn peak_rss_bytes() -> u64 {
    // SAFETY: `getrusage` only writes the struct it is given.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) } != 0 {
        return 0;
    }
    let max = ru.ru_maxrss as u64;
    // macOS reports bytes, Linux kilobytes.
    if cfg!(target_os = "macos") {
        max
    } else {
        max * 1024
    }
}
