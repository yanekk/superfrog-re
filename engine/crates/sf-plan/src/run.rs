//! Shell side of the explorer (DESIGN §2.4, §3.1): a round's parts run on `std::thread::scope`, the
//! wall-clock budget is checked between rounds, peak resident memory is read from the OS. The core
//! decides everything else, so the route is the same for any thread count (M8).

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use sf_core::Game;
use sf_core::input::FrameInput;

use crate::core::shorten::{self, Shortcut};
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
    /// When set, `shorten_min_rounds` applies only to a first exit longer than this many frames or
    /// losing a heart; otherwise the share alone. A repair stretch passes the frames its old plan
    /// still had: its route is mostly that plan's, already shortened, unless its search went
    /// another way (W1-2's repair took a 7 200-frame, 3-heart exit for a 4 718-frame plan, T14).
    pub shorten_min_over: Option<usize>,
}

impl Default for RunOptions {
    fn default() -> RunOptions {
        RunOptions {
            threads: 1,
            budget: Duration::from_secs(30 * 60),
            shorten_share: 0.0,
            shorten_min_rounds: 0,
            shorten_min_over: None,
        }
    }
}

/// Shortening's defaults (DESIGN §2.12): 20% of the rounds to the first exit, at least
/// `SHORTEN_MIN_ROUNDS`, about 60 s of rounds on this machine (10 threads, 80 bursts a round;
/// measured in T22, FINDINGS). T14 tunes both.
pub const SHORTEN_SHARE: f32 = 0.2;
pub const SHORTEN_MIN_ROUNDS: u64 = 180;

impl RunOptions {
    /// The defaults with shortening on (`sf-plan`'s, T11).
    pub fn shortening() -> RunOptions {
        RunOptions {
            shorten_share: SHORTEN_SHARE,
            shorten_min_rounds: SHORTEN_MIN_ROUNDS,
            ..RunOptions::default()
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
/// route and why (DESIGN §2.9). With shortening on (DESIGN §2.12) the search goes on after the
/// first exit for `shorten::budget` more rounds keeping the shortest exit, then the shortcut pass
/// spends as many rounds again; the wall-clock budget caps both. The route's `rounds` and
/// `engine_frames` then count everything, the pass's explorer runs included.
pub fn plan_engine(
    start: Game,
    facts: Facts,
    opt: Options,
    run: &RunOptions,
) -> Result<Route, (Route, Reason)> {
    let t0 = Instant::now();
    let shorten_on = run.shorten_share > 0.0 || run.shorten_min_rounds > 0;
    // Kept only when shortening needs them: the start's state is ~0.8 MB.
    let keep = shorten_on.then(|| (start.clone(), facts.clone(), opt.clone()));
    let mut ex = Explorer::new(start, facts, opt);
    let mut best = ex.found().cloned();
    while best.is_none() {
        if t0.elapsed() >= run.budget {
            return Err(ex.best_partial());
        }
        if let Progress::Exit(r) = run_round(&mut ex, run.threads) {
            best = Some(r);
        }
    }
    let mut best = best.unwrap();
    let Some((start, facts, opt)) = keep else {
        return Ok(best);
    };
    let min = match run.shorten_min_over {
        Some(n) if best.inputs.len() <= n && best.hearts_lost == 0 => 0,
        _ => run.shorten_min_rounds,
    };
    let extra = shorten::budget(ex.rounds(), run.shorten_share, min);
    let until = ex.rounds() + extra;
    while ex.rounds() < until && t0.elapsed() < run.budget {
        if let Progress::Exit(r) = run_round(&mut ex, run.threads)
            && shorten::better(&r, &best)
        {
            best = r;
        }
    }
    let (mut rounds, mut frames) = (ex.rounds(), ex.engine_frames());
    best.cells = ex.archive().entries().len() as u32;
    drop(ex);
    // The pass's explorer runs know nothing of hints: a stretch from i could skip a `via` cell or
    // cross an `avoid` box, so a hinted route is only shortened by the search above.
    if opt.via.is_empty() && opt.avoid.is_empty() {
        let deadline = t0 + run.budget;
        let p = shortcut_pass(&start, &facts, &opt, best.inputs.clone(), extra, run.threads, deadline);
        rounds += p.rounds;
        frames += p.engine_frames;
        if p.inputs.len() < best.inputs.len() {
            best.inputs = p.inputs;
            best.hearts_lost = p.hearts_lost;
        }
    }
    best.rounds = rounds;
    best.engine_frames = frames;
    Ok(best)
}

/// What the shortcut pass made of a route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassResult {
    pub inputs: Vec<FrameInput>,
    pub hearts_lost: u8,
    pub rounds: u64,
    pub engine_frames: u64,
    pub pairs_tried: u64,
    pub stretches_kept: u64,
}

/// The shortcut pass (DESIGN §2.12) over `route`, an exit from `start`: each pair's explorer runs
/// on `threads` threads with `opt`'s seed, burst count and caps, seeded from `opt.seed` and the
/// pair's index (M8), for at most `rounds` rounds in all; it stops early at `deadline`.
pub fn shortcut_pass(
    start: &Game,
    facts: &Facts,
    opt: &Options,
    route: Vec<FrameInput>,
    rounds: u64,
    threads: usize,
    deadline: Instant,
) -> PassResult {
    let mut sc = Shortcut::new(start, facts, route, rounds);
    let (mut spent, mut frames, mut pair) = (0u64, 0u64, 0u64);
    while Instant::now() < deadline {
        let cap = sc.pair_rounds();
        let Some((g, goal)) = sc.next() else { break };
        pair += 1;
        let o = Options {
            seed: opt.seed ^ pair.wrapping_mul(0x9e37_79b9_7f4a_7c15),
            max_frames: opt.max_frames,
            max_mem_bytes: opt.max_mem_bytes,
            bursts_per_round: opt.bursts_per_round,
            goal,
            ..Options::default()
        };
        let mut pe = Explorer::new(g, facts.clone(), o);
        let mut found = None;
        while found.is_none() && pe.rounds() < cap && Instant::now() < deadline {
            if let Progress::Exit(r) = run_round(&mut pe, threads) {
                found = Some(r.inputs);
            }
        }
        sc.spend(pe.rounds().max(1));
        spent += pe.rounds();
        frames += pe.engine_frames();
        sc.offer(found);
    }
    let (pairs_tried, stretches_kept) = sc.counts();
    PassResult {
        inputs: sc.route().to_vec(),
        hearts_lost: sc.hearts_lost(),
        rounds: spent,
        engine_frames: frames + sc.engine_frames(),
        pairs_tried,
        stretches_kept,
    }
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
