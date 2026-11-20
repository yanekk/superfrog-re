//! Shell side of the explorer: a round's parts run on `std::thread::scope`, the wall-clock budget is
//! checked between rounds, peak resident memory is read from the OS. The core decides everything
//! else, so the route is the same for any thread count.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use amiga_input::FrameInput;

use crate::core::shorten::{self, Shortcut};
use crate::core::{Explorer, Facts, Options, PartResult, PlanGame, Progress, Reason, Route};

/// Parts per thread a round is cut into: bursts vary in length, so small parts taken in turn keep
/// the threads busy to the round's end.
const PARTS_PER_THREAD: usize = 4;

/// The compass's cost over the run, for its log: the longest `Explorer::new` in nanoseconds (the
/// walk compass's open cells and first fill; no clock in `core`), and the most fill bytes one
/// explorer held at its end (outside the archive's count).
pub static COMPASS_BUILD_NS: AtomicU64 = AtomicU64::new(0);
pub static COMPASS_FILL_BYTES: AtomicU64 = AtomicU64::new(0);
pub static COMPASS_FILLS: AtomicU64 = AtomicU64::new(0);

fn note_fills<G: PlanGame>(ex: &Explorer<G>) {
    let (n, bytes) = ex.compass_fills();
    if bytes > COMPASS_FILL_BYTES.fetch_max(bytes, Ordering::Relaxed) {
        COMPASS_FILLS.store(n as u64, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunOptions {
    pub threads: usize,
    /// Wall-clock: the search stops between rounds once it is spent; a stopped run is not held to
    /// be the same on every machine.
    pub budget: Duration,
    /// Shortening after the first exit; the default keeps it off.
    pub shorten_share: f32,
    pub shorten_min_rounds: u64,
    /// When set, `(frames, hearts)`: `shorten_min_rounds` applies only to a first exit more than
    /// `over_slack` longer than `frames` or losing more than `hearts` health; otherwise the share
    /// alone. A repair stretch passes the frames its old plan still had and the health that plan
    /// lost: its route is mostly that plan's, already shortened, unless its search went another
    /// way (one repair took a 7 200-frame exit losing health for a 4 718-frame plan losing none).
    pub shorten_min_over: Option<(usize, u8)>,
    /// When set, the search gives up at this many rounds with no goal found, as at the budget's
    /// end (a repair's reach stretches).
    pub max_rounds: Option<u64>,
    /// A repair's stretches: the search gives up, as at the budget's end, once it is a dead end
    /// (`dead_end`: `DEAD_ROUNDS` rounds with at most `DEAD_CELLS` cells), so the repair can
    /// rewind instead of spending the budget there.
    pub dead_end: bool,
    /// Log the first exit and the shortening budget to stderr.
    pub log: bool,
}

/// A search that has archived at most this many cells after `DEAD_ROUNDS` rounds is a dead end.
pub const DEAD_CELLS: u32 = 8;
pub const DEAD_ROUNDS: u64 = 100;

/// Whether a search `rounds` in with `cells` archived is a dead end: counted in rounds and cells,
/// never time, so the same rounds end it on any machine.
pub fn dead_end(rounds: u64, cells: u32) -> bool {
    rounds >= DEAD_ROUNDS && cells <= DEAD_CELLS
}

impl Default for RunOptions {
    fn default() -> RunOptions {
        RunOptions {
            threads: 1,
            budget: Duration::from_secs(30 * 60),
            shorten_share: 0.0,
            shorten_min_rounds: 0,
            shorten_min_over: None,
            max_rounds: None,
            dead_end: false,
            log: false,
        }
    }
}

/// Shortening's defaults: 20% of the rounds to the first exit, at least `SHORTEN_MIN_ROUNDS`, about
/// 60 s of rounds on a 10-thread machine at 80 bursts a round.
pub const SHORTEN_SHARE: f32 = 0.2;
pub const SHORTEN_MIN_ROUNDS: u64 = 180;

impl RunOptions {
    /// The defaults with shortening on.
    pub fn shortening() -> RunOptions {
        RunOptions {
            shorten_share: SHORTEN_SHARE,
            shorten_min_rounds: SHORTEN_MIN_ROUNDS,
            ..RunOptions::default()
        }
    }
}

/// Plays one round on `threads` threads and merges it.
pub fn run_round<G: PlanGame>(ex: &mut Explorer<G>, threads: usize) -> Progress {
    let threads = threads.max(1);
    let parts = ex.round_parts(threads * PARTS_PER_THREAD);
    let results: Vec<Mutex<Option<PartResult<G>>>> =
        parts.iter().map(|_| Mutex::new(None)).collect();
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

/// How much longer than `shorten_min_over`'s frames a first exit may be without the minimum: 10%
/// and 64 frames. A repair's exit is rarely as short as the rest it replaces; at the bare bound
/// eight repairs of one level each spent ~380 rounds shortening an exit found in ~20.
pub fn over_slack(frames: usize) -> usize {
    frames + frames / 10 + 64
}

/// The explorer from `start` to the goal, within `run`'s budget: the route, or the best partial
/// route and why. With shortening on the search goes on after the
/// first exit for `shorten::budget` more rounds keeping the shortest exit, then the shortcut pass
/// spends as many rounds again; the wall-clock budget caps both. The route's `rounds` and
/// `engine_frames` then count everything, the pass's explorer runs included.
pub fn plan_engine<G: PlanGame>(
    start: G,
    facts: Facts,
    opt: Options,
    run: &RunOptions,
) -> Result<Route, (Route, Reason)> {
    let t0 = Instant::now();
    let shorten_on = run.shorten_share > 0.0 || run.shorten_min_rounds > 0;
    // Kept only when shortening needs them: a start's state may be most of a megabyte.
    let keep = shorten_on.then(|| (start.clone(), facts.clone(), opt.clone()));
    let t_new = Instant::now();
    let mut ex = Explorer::new(start, facts, opt);
    COMPASS_BUILD_NS.fetch_max(t_new.elapsed().as_nanos() as u64, Ordering::Relaxed);
    let mut best = ex.found().cloned();
    while best.is_none() {
        if t0.elapsed() >= run.budget || run.max_rounds.is_some_and(|m| ex.rounds() >= m) {
            note_fills(&ex);
            return Err(ex.best_partial());
        }
        if let Progress::Exit(r) = run_round(&mut ex, run.threads) {
            best = Some(r);
        } else if run.dead_end && dead_end(ex.rounds(), ex.archive().entries().len() as u32) {
            // Counted in rounds and cells, never time: the same rounds end it on any machine.
            return Err(ex.best_partial());
        }
    }
    let mut best = best.unwrap();
    let Some((start, facts, opt)) = keep else {
        note_fills(&ex);
        return Ok(best);
    };
    let min = match run.shorten_min_over {
        Some((n, h)) if best.inputs.len() <= over_slack(n) && best.hearts_lost <= h => 0,
        _ => run.shorten_min_rounds,
    };
    let extra = shorten::budget(ex.rounds(), run.shorten_share, min);
    if run.log {
        eprintln!(
            "  {:6.1}s first exit: {} frames, {} hearts, {} rounds; shortening {} rounds",
            t0.elapsed().as_secs_f64(),
            best.inputs.len(),
            best.hearts_lost,
            ex.rounds(),
            extra
        );
    }
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
    note_fills(&ex);
    drop(ex);
    // The pass's explorer runs know nothing of hints: a stretch from i could skip a `via` cell or
    // cross an `avoid` box, so a hinted route is only shortened by the search above.
    if opt.via.is_empty() && opt.avoid.is_empty() {
        let deadline = t0 + run.budget;
        let p = shortcut_pass(
            &start,
            &facts,
            &opt,
            best.inputs.clone(),
            extra,
            run.threads,
            deadline,
        );
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

/// The shortcut pass over `route`, an exit from `start`: each pair's explorer runs on `threads`
/// threads with `opt`'s seed, burst count and caps, seeded from `opt.seed` and the pair's index,
/// for at most `rounds` rounds in all; it stops early at `deadline`.
pub fn shortcut_pass<G: PlanGame>(
    start: &G,
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

/// The process's peak resident memory in bytes.
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
