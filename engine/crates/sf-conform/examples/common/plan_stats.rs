//! The `PLAN_STATS` line `sf-bench` reads off a planner's stderr at its exit (smart-planner T01):
//! engine and oracle frames stepped and the share of the run spent stepping the original, so the
//! benchmark records the same numbers for the old planners as for `sf-plan`.
//!
//! `PLAN_STATS engine_frames=N oracle_frames=N confirm_share=F hints=0` (the old planners take no
//! hint the benchmark passes, so `hints` is always 0 here). The line is also printed every
//! `EVERY`, so a planner killed for its timeout still leaves its counts; the last line counts.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static ENGINE_FRAMES: AtomicU64 = AtomicU64::new(0);
static ORACLE_FRAMES: AtomicU64 = AtomicU64::new(0);
static ORACLE_NS: AtomicU64 = AtomicU64::new(0);

const EVERY: std::time::Duration = std::time::Duration::from_secs(30);

/// The run's clock starts here (the share's denominator).
pub fn start() {
    START.get_or_init(Instant::now);
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(EVERY);
            print();
        }
    });
}

/// One engine frame stepped.
pub fn engine() {
    ENGINE_FRAMES.fetch_add(1, Relaxed);
}

/// `sync::step_inserting`, counted and timed.
pub fn oracle_step(
    a: &mut sf_oracle::Amiga,
    x: sf_core::input::FrameInput,
) -> Result<sf_oracle::FrameCapture, String> {
    let t = Instant::now();
    let r = sf_conform::sync::step_inserting(a, x);
    ORACLE_NS.fetch_add(t.elapsed().as_nanos() as u64, Relaxed);
    ORACLE_FRAMES.fetch_add(1, Relaxed);
    r
}

pub fn print() {
    let wall = START.get_or_init(Instant::now).elapsed().as_nanos().max(1) as f64;
    let share = (ORACLE_NS.load(Relaxed) as f64 / wall).min(1.0);
    eprintln!(
        "PLAN_STATS engine_frames={} oracle_frames={} confirm_share={share:.4} hints=0",
        ENGINE_FRAMES.load(Relaxed),
        ORACLE_FRAMES.load(Relaxed),
    );
}
