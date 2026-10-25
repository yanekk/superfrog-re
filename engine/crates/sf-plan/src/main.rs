//! `sf-plan`: the one command an agent runs to route a level (plans/smart-planner DESIGN §2.1, T11).
//!
//! ```text
//! sf-plan <level> [--snap leg.snap] [--guide route-or-waypoints.txt] [--follow old.txt] [--seed N]
//!         [--out route.txt] [--budget-min 30] [--frames 30000] [--max-mem-gb 4] [--threads N]
//!         [--compass walk|straight|map] [--shorten-share 0.2] [--no-cache] [--engine-only | --app]
//!         [--no-pace] [--lag-limit N]
//!         [--via c,r]... [--avoid c0,r0,c1,r1]...
//! ```
//!
//! Exit 0: a route to the exit is written. Exit 1: the budget or the frames ran out; the best
//! partial route is written and a one-line reason goes to stderr (§2.9). Exit 2: bad arguments, an
//! invalid start, or a guide that does not parse or never leaves its start. The last stderr line is
//! always `PLAN_STATS …`, which `sf-bench` reads.
//!
//! The route's frame 0 is the start's first frame: the level's `AtLevel` sync point, or the leg
//! snapshot's frame. Original mode (the default) plans on the engine and confirms on the original
//! by stretch and repair (§2.6); `--engine-only` plans from the original's state transplanted at
//! the same start and confirms nothing (for tuning). `--app` (set C, T12) starts the level as
//! Game ▸ Start Level… does under a script with power-on seed `N N` (`--seed N`), and plans and
//! confirms on the engine from there: the route is written `# timing: app` (DESIGN §2.6).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime};

use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::FrameInput;
use sf_assets::World;
use sf_core::{Game, LevelId, Start};
use sf_oracle::Amiga;
use sf_plan::app;
use sf_plan::cache::{self, StartKey};
use sf_plan::core::repair::{TracePoint, follow_options, points_from_trace};
use sf_plan::core::{Cell, Compass, Explorer, Facts, GuidePoint, Options, Reason, Route, default_moves};
use sf_plan::guide_file::parse_waypoints;
use sf_plan::oracle::{self, Stats};
use sf_plan::run::{RunOptions, SHORTEN_MIN_ROUNDS, peak_rss_bytes, plan_engine};
use sf_plan::threads::build_threaded;
use sf_plan::trace::{script_inputs, timing, trace_original};

const USAGE: &str = "usage: sf-plan <level> [--snap leg.snap] [--guide route-or-waypoints.txt] \
[--follow old.txt] [--seed N] [--out route.txt] [--budget-min 30] [--frames 30000] [--max-mem-gb 4] \
[--threads N] [--compass walk|straight|map] [--shorten-share 0.2] [--no-cache] [--engine-only | --app] \
[--no-pace] [--lag-limit N] \
[--via c,r]... [--avoid c0,r0,c1,r1]...";

/// Oracle frames beyond `--frames` a start is given room for: the settle and the lag tail.
const ORACLE_SLACK: u32 = 1_000;
/// Idle frames after the exit frame before fire takes the tally's points, and how long it is held:
/// the tail every committed `complete-*` script carries (DESIGN §2.1).
const TALLY_WAIT: usize = 69;
const TALLY_FIRE: usize = 4;

#[derive(Debug, Clone, PartialEq)]
enum CompassArg {
    Straight,
    Map,
    /// D25: needs no map, nothing on disk.
    Walk,
}

#[derive(Debug, Clone)]
struct Args {
    level: LevelId,
    snap: Option<PathBuf>,
    guide: Option<PathBuf>,
    follow: Option<PathBuf>,
    seed: u64,
    out: Option<PathBuf>,
    budget: Duration,
    frames: u32,
    max_mem_gb: f64,
    threads: usize,
    compass: CompassArg,
    shorten_share: f32,
    no_cache: bool,
    engine_only: bool,
    /// Set C: the app's Start Level start, planned and confirmed on the engine (T12).
    app: bool,
    /// `--no-pace`: a played-route guide without its pace (for comparisons, T14).
    no_pace: bool,
    /// `--lag-limit N`: overrides the faithful check's lag bar looked up for the level (T14;
    /// sf-bench never sets it).
    lag_limit: Option<u32>,
    via: Vec<Cell>,
    avoid: Vec<(Cell, Cell)>,
}

impl Args {
    fn hints(&self) -> usize {
        self.via.len() + self.avoid.len()
    }
}

fn cells(s: &str, n: usize, what: &str) -> Result<Vec<u16>, String> {
    let v: Vec<u16> = s
        .split(',')
        .map(|x| x.trim().parse::<u16>())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("{what} {s}: want {} numbers", n))?;
    if v.len() != n {
        return Err(format!("{what} {s}: want {n} numbers"));
    }
    Ok(v)
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut it = argv.iter();
    let mut level = None;
    let mut a = Args {
        level: "W1-1".parse().unwrap(),
        snap: None,
        guide: None,
        follow: None,
        seed: 1,
        out: None,
        budget: Duration::from_secs(30 * 60),
        frames: 30_000,
        max_mem_gb: 4.0,
        threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
        // The walk compass, kept by T28's trial: as many completions, fewer engine frames.
        compass: CompassArg::Walk,
        shorten_share: sf_plan::run::SHORTEN_SHARE,
        no_cache: false,
        engine_only: false,
        app: false,
        no_pace: false,
        lag_limit: None,
        via: Vec::new(),
        avoid: Vec::new(),
    };
    while let Some(k) = it.next() {
        let mut val = || it.next().cloned().ok_or(format!("{k} wants a value"));
        let num = |v: String| v.parse::<f64>().map_err(|_| format!("{k} {v}: not a number"));
        match k.as_str() {
            "--snap" => a.snap = Some(val()?.into()),
            "--guide" => a.guide = Some(val()?.into()),
            "--follow" => a.follow = Some(val()?.into()),
            "--out" => a.out = Some(val()?.into()),
            "--seed" => a.seed = val()?.parse().map_err(|_| format!("{k}: not a number"))?,
            "--budget-min" => {
                let m = num(val()?)?;
                if !(m >= 0.0 && m.is_finite()) {
                    return Err(format!("{k}: not a number of minutes"));
                }
                a.budget = Duration::from_secs_f64(m * 60.0);
            }
            "--frames" => a.frames = val()?.parse().map_err(|_| format!("{k}: not a number"))?,
            "--max-mem-gb" => {
                a.max_mem_gb = num(val()?)?;
                if !(a.max_mem_gb > 0.0 && a.max_mem_gb.is_finite()) {
                    return Err(format!("{k}: want a positive number"));
                }
            }
            "--threads" => {
                a.threads = val()?.parse().map_err(|_| format!("{k}: not a number"))?;
                if a.threads == 0 {
                    return Err(format!("{k}: at least 1"));
                }
            }
            "--compass" => {
                a.compass = match val()?.as_str() {
                    "straight" => CompassArg::Straight,
                    "map" => CompassArg::Map,
                    "walk" => CompassArg::Walk,
                    v => return Err(format!("{k} {v}: straight, map or walk")),
                }
            }
            "--shorten-share" => {
                let s = num(val()?)?;
                if !(s >= 0.0 && s.is_finite()) {
                    return Err(format!("{k}: want 0 or more"));
                }
                a.shorten_share = s as f32;
            }
            "--no-cache" => a.no_cache = true,
            "--no-pace" => a.no_pace = true,
            "--lag-limit" => {
                a.lag_limit = Some(val()?.parse().map_err(|_| format!("{k}: not a number"))?)
            }
            "--engine-only" => a.engine_only = true,
            "--via" => {
                let v = cells(&val()?, 2, "--via")?;
                a.via.push((v[0], v[1]));
            }
            "--avoid" => {
                let v = cells(&val()?, 4, "--avoid")?;
                a.avoid.push(((v[0], v[1]), (v[2], v[3])));
            }
            "--app" => a.app = true,
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s if level.is_none() => {
                let id: LevelId = s.parse()?;
                if matches!(id.world, World::Bonus | World::Final) {
                    return Err(format!(
                        "{id} is not a normal level: the bonus level and the witch keep their own routes (D1)"
                    ));
                }
                level = Some(id);
            }
            s => return Err(format!("one level only: {s}?")),
        }
    }
    a.level = level.ok_or("no level given")?;
    if a.app {
        if a.snap.is_some() {
            return Err("--app starts the level as Start Level does: no --snap".into());
        }
        if a.engine_only {
            return Err("--app already plans and confirms on the engine: no --engine-only".into());
        }
        if a.seed > u32::MAX as u64 {
            return Err("--seed: the app's power-on seed is a 32-bit number".into());
        }
    }
    Ok(a)
}

/// Why a run ended without a route on disk worth confirming.
enum Fail {
    /// Exit 2: bad arguments, invalid start, bad guide.
    Usage(String),
}

/// A guide or follow file read before anything is started: a script, or (guide only) waypoints.
enum GuideFile {
    Script(GuideScript),
    Waypoints(Vec<GuidePoint>),
}

/// A route script and the timing it is played in at its own start (DESIGN §2.6, §2.11).
struct GuideScript {
    script: InputScript,
    /// `# timing: app`: played on the engine from the app start with this seed (its `# seed:`
    /// line, else the run's); `None`: on the original.
    app_seed: Option<u32>,
}

/// A file whose first line is a timing line, or which parses as a script, is a script; otherwise
/// it is a waypoint file. A file that is neither names the bad line of the form it looks most like
/// (a `:` on a line is a script's).
fn read_guide(path: &Path, waypoints_ok: bool, run_seed: u32) -> Result<GuideFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let looks_script = timing(&text).is_some()
        || text.lines().any(|l| l.split('#').next().unwrap_or("").contains(':'));
    let app_seed = match timing(&text) {
        None | Some("original") => None,
        Some("app") => Some(app::route_seed(&text).unwrap_or(run_seed)),
        Some(t) => return Err(format!("{}: timing `{t}`: original or app", path.display())),
    };
    if looks_script || !waypoints_ok {
        return InputScript::parse(&text)
            .map(|script| GuideFile::Script(GuideScript { script, app_seed }))
            .map_err(|e| format!("{}: {e}", path.display()));
    }
    parse_waypoints(&text)
        .map(GuideFile::Waypoints)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The oracle at the run's start (frame 0 of the route), before the level-start freeze.
fn oracle_at(a: &Args, cap: u32) -> Result<Amiga, String> {
    match &a.snap {
        Some(p) => sync::at_snapshot_file(p, cap),
        None => Ok(sync::reach(&SyncPoint::AtLevel(a.level), &InputScript::idle(), cap)?.amiga),
    }
}

/// A script played at its own start in its own timing, sampled for guide points (§2.11): on the
/// original from the run's start, or for an app-timed one on the engine from its app start. One
/// that never leaves its first cell is refused (§2.9).
fn trace_script(a: &Args, facts: &Facts, path: &Path, g: &GuideScript) -> Result<Vec<TracePoint>, String> {
    let script = &g.script;
    let n = script.lines.last().map_or(0, |l| l.0);
    let t = match g.app_seed {
        Some(seed) => app::trace_app(a.level, seed, facts, script),
        None => {
            let mut amiga = oracle_at(a, n + 100)?;
            trace_original(&mut amiga, facts, script, n)?
        }
    };
    match t.first() {
        Some(p0) if t.iter().any(|p| p.0 != p0.0) => Ok(t),
        _ => Err(format!("{}: played from the start, it never leaves its start cell", path.display())),
    }
}

/// The route as a script (DESIGN §2.1): timing line, a comment, one line per change, and for a
/// route that reaches the exit the committed routes' tally tail (idle, then fire).
fn script_text(a: &Args, inputs: &[FrameInput], exit: bool) -> String {
    let mut s = if a.app {
        // The seed is read back by `app::route_seed`: an app route replays only from its own start.
        format!("# timing: app\n# seed: {}\n", a.seed)
    } else {
        String::from("# timing: original\n")
    };
    let how = if a.app {
        "planned and confirmed on the engine (--app)"
    } else if a.engine_only {
        "planned on the engine only (--engine-only), not confirmed on the original"
    } else {
        "planned on the engine, confirmed on the original"
    };
    let start = match &a.snap {
        Some(p) => format!(
            "the leg snapshot {}",
            p.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned())
        ),
        None if a.app => format!(
            "{}'s first frame from Game ▸ Start Level… (the dialog's defaults, power-on seed {} {})",
            a.level, a.seed, a.seed
        ),
        None => format!("{}'s AtLevel sync point", a.level),
    };
    let _ = writeln!(s, "# {} by sf-plan, seed {}: {how}; frame 0 is {start}.", a.level, a.seed);
    if exit {
        let _ = writeln!(
            s,
            "# The level ends at frame {}, then fire in the tally's wait takes the points.",
            inputs.len()
        );
    } else {
        let _ = writeln!(s, "# A partial route: it does not reach the exit.");
    }
    let mut all = inputs.to_vec();
    if exit {
        all.extend(std::iter::repeat_n(FrameInput::NONE, TALLY_WAIT));
        all.extend(std::iter::repeat_n(FrameInput::joy(sf_core::input::Joy::FIRE), TALLY_FIRE));
    }
    let mut last = None;
    for (k, x) in all.iter().enumerate() {
        if Some(*x) != last {
            let _ = writeln!(s, "{k}: {}", InputScript::format_input(*x));
            last = Some(*x);
        }
    }
    let _ = writeln!(s, "{}: -", all.len());
    s
}

/// The hint log (DESIGN §2.10): `$SF_PLAN_HINTS_LOG`, else `engine/conform/plan-hints.log`.
fn hints_log() -> PathBuf {
    std::env::var_os("SF_PLAN_HINTS_LOG")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conform/plan-hints.log"))
}

/// Today's date, UTC, as `YYYY-MM-DD` (civil-from-days, no date crate in the workspace).
fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// One line per hinted run: date, level, start, the hints, whether it completed.
fn log_hints(a: &Args, completed: bool) {
    if a.hints() == 0 {
        return;
    }
    let start = match &a.snap {
        Some(p) => format!("snap:{}", p.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned())),
        None if a.app => format!("app:{}", a.seed),
        None => "at-level".into(),
    };
    let mut hints = Vec::new();
    for (c, r) in &a.via {
        hints.push(format!("--via {c},{r}"));
    }
    for ((c0, r0), (c1, r1)) in &a.avoid {
        hints.push(format!("--avoid {c0},{r0},{c1},{r1}"));
    }
    let line = format!(
        "{} {} {} {} completed={}\n",
        today(),
        a.level,
        start,
        hints.join(" "),
        if completed { "yes" } else { "no" }
    );
    let path = hints_log();
    let r = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    if let Err(e) = r {
        eprintln!("sf-plan: the hint log {} was not written: {e}", path.display());
    }
}

/// What a run ends with, for the files and the stats line.
struct Outcome {
    inputs: Vec<FrameInput>,
    exit: bool,
    reason: Option<String>,
    route: Route,
    stats: Stats,
}

fn reason_line(r: &Reason, why: &str, confirmed: Option<u32>) -> String {
    let mut s = format!(
        "sf-plan: no route: {why}; coins still needed {}, least exit distance with the coins met {}, cells archived {}",
        r.coins_left,
        r.exit_distance_with_coins_met
            .map_or("none (the coins were never met)".into(), |d| format!("{d} cells")),
        r.cells
    );
    if let Some(f) = confirmed {
        let _ = write!(s, ", the original confirmed to frame {f}");
    }
    s
}

/// The guide and follow files read before anything is started (§2.11, §2.7).
struct Files {
    guide: Option<GuideFile>,
    follow: Option<GuideScript>,
}

fn read_files(a: &Args) -> Result<Files, Fail> {
    let seed = a.seed as u32;
    let guide = a.guide.as_deref().map(|p| read_guide(p, true, seed)).transpose().map_err(Fail::Usage)?;
    let follow = match a.follow.as_deref().map(|p| read_guide(p, false, seed)).transpose().map_err(Fail::Usage)? {
        Some(GuideFile::Script(s)) => Some(s),
        Some(GuideFile::Waypoints(_)) => unreachable!("a follow file is read as a script"),
        None => None,
    };
    Ok(Files { guide, follow })
}

/// What the guide and follow scripts cost: wall-clock on the original and frames played.
#[derive(Default)]
struct GuideCost {
    oracle_time: Duration,
    oracle_frames: u64,
}

/// `opt` with the guide and the follow seed applied (§2.7, §2.11), each script played at its own
/// start in its own timing. `prefix` is the frames the run's start settled for: the explorer starts
/// past them, so a follow script's seed inputs do too.
fn apply_guides(a: &Args, files: &Files, facts: &Facts, prefix: usize, mut opt: Options) -> Result<(Options, GuideCost), Fail> {
    let mut cost = GuideCost::default();
    let mut trace = |p: &Path, g: &GuideScript| -> Result<Vec<TracePoint>, Fail> {
        let t = Instant::now();
        let tr = trace_script(a, facts, p, g).map_err(Fail::Usage)?;
        if g.app_seed.is_none() {
            cost.oracle_time += t.elapsed();
            cost.oracle_frames += g.script.lines.last().map_or(0, |l| l.0) as u64;
        }
        Ok(tr)
    };
    let guide_points = match (&files.guide, a.guide.as_deref()) {
        (Some(GuideFile::Script(s)), Some(p)) => Some((points_from_trace(&trace(p, s)?), !a.no_pace)),
        (Some(GuideFile::Waypoints(w)), _) => Some((w.clone(), false)),
        _ => None,
    };
    if let (Some(s), Some(p)) = (&files.follow, a.follow.as_deref()) {
        let tr = trace(p, s)?;
        // The script's frame 0 is the start's; the explorer starts past the settle, so its seed
        // inputs start there too.
        let mut inputs = script_inputs(&s.script);
        inputs.drain(..prefix.min(inputs.len()));
        opt = follow_options(opt, inputs, &tr);
    }
    if let Some((g, pace)) = guide_points {
        // With both, the guide steers and `--follow` only seeds (§2.7). A played route's points
        // carry its pace; waypoints none (T14).
        opt.guide = g;
        opt.guide_pace = pace;
    }
    if a.app {
        // No pace in app mode (owner, 2026-10-23): on W1-1 it made app routes 15–56% longer on
        // three seeds, hearts lost the same, and cost T12's M8 test its no-heart route.
        opt.guide_pace = false;
    }
    Ok((opt, cost))
}

/// The compass (§2.2): the map from the cache under the start's key; walk (D25) is built by the
/// explorer from its start.
fn map_compass(a: &Args, game: &Game, facts: &Facts, opt: &mut Options) -> Result<(), Fail> {
    match a.compass {
        CompassArg::Straight => return Ok(()),
        CompassArg::Walk => {
            opt.compass = Compass::Walk;
            return Ok(());
        }
        CompassArg::Map => {}
    }
    let wings = game.state().w(FROG + f::FLIGHT) != 0;
    let sk = match &a.snap {
        Some(p) => StartKey::snap(&std::fs::read(p).map_err(|e| Fail::Usage(format!("--snap: {e}")))?),
        None if a.app => StartKey::App { seed: a.seed as u32 },
        None => StartKey::AtLevel,
    };
    let key = cache::key(a.level, sk, wings);
    let map = cache::load_or_build(&key, a.no_cache, || {
        build_threaded(game, facts, &default_moves(wings), a.threads).0
    });
    opt.compass = Compass::Map(map);
    Ok(())
}

fn base_options(a: &Args) -> Options {
    Options {
        seed: a.seed,
        max_mem_bytes: (a.max_mem_gb * (1u64 << 30) as f64) as u64,
        via: a.via.clone(),
        avoid: a.avoid.clone(),
        ..Options::default()
    }
}

fn run_options(a: &Args, t0: Instant) -> RunOptions {
    RunOptions {
        threads: a.threads,
        budget: a.budget.saturating_sub(t0.elapsed()),
        shorten_share: a.shorten_share,
        shorten_min_rounds: if a.shorten_share > 0.0 { SHORTEN_MIN_ROUNDS } else { 0 },
        shorten_min_over: None,
        lag_limit: a
            .lag_limit
            .unwrap_or_else(|| sf_plan::oracle::lag_limit_for(a.level, a.snap.is_some())),
        max_rounds: None,
        dead_end: false,
    }
}

/// The run's share of wall-clock on the original (DESIGN §2.6).
fn share(t0: Instant, on_oracle: Duration) -> f32 {
    (on_oracle.as_secs_f64() / t0.elapsed().as_secs_f64().max(1e-9)).clamp(0.0, 1.0) as f32
}

/// `--app` (set C, DESIGN §2.6): the level from the app's Start Level start, settled on the
/// engine, planned there, and accepted only when the whole route, settle included, replays from
/// that start to the exit with no life lost. Only an original-timed guide touches the original,
/// and it gives cells, not inputs, so no frame of it is written under the app's timing.
fn run_app(a: &Args, t0: Instant, files: &Files) -> Result<Outcome, Fail> {
    let seed = a.seed as u32;
    let mut game = app::app_start(a.level, seed);
    let mut inputs = app::settle(&mut game);
    let prefix = inputs.len();
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let facts = sf_plan::core::facts(&game, &data.levels[&a.level], display.rom());
    let (mut opt, cost) = apply_guides(a, files, &facts, prefix, base_options(a))?;
    map_compass(a, &game, &facts, &mut opt)?;
    let mut stats = Stats {
        oracle_frames: cost.oracle_frames,
        ..Stats::default()
    };
    if prefix >= a.frames as usize {
        let ex = Explorer::new(game, facts, Options { max_frames: 0, ..opt });
        let (mut route, why) = ex.best_partial();
        route.inputs = inputs[..a.frames as usize].to_vec();
        stats.confirm_share = share(t0, cost.oracle_time);
        return Ok(Outcome {
            inputs: route.inputs.clone(),
            exit: false,
            reason: Some(reason_line(&why, "frames exhausted (--frames)", None)),
            route,
            stats,
        });
    }
    opt.max_frames = a.frames - prefix as u32;
    let (route, mut reason) = match plan_engine(game, facts, opt, &run_options(a, t0)) {
        Ok(route) => (route, None),
        Err((route, why)) => {
            let w = if t0.elapsed() >= a.budget { "budget spent" } else { "frames or budget exhausted" };
            (route, Some(reason_line(&why, w, None)))
        }
    };
    inputs.extend_from_slice(&route.inputs);
    if reason.is_none() {
        let r = app::replay(a.level, seed, &inputs);
        if !r.completed() || r.exit != Some(inputs.len() as u32) {
            reason = Some(format!(
                "sf-plan: no route: the plan did not replay from the app start (exit at {:?} of {} frames, lives lost {})",
                r.exit,
                inputs.len(),
                r.lives_lost
            ));
        }
    }
    stats.engine_frames = route.engine_frames;
    stats.confirm_share = share(t0, cost.oracle_time);
    Ok(Outcome {
        exit: reason.is_none(),
        inputs,
        reason,
        route,
        stats,
    })
}

fn run(a: &Args, t0: Instant) -> Result<Outcome, Fail> {
    // Files first: a bad guide costs no oracle time.
    let files = read_files(a)?;
    if a.app {
        return run_app(a, t0, &files);
    }
    if let Some(p) = &a.snap {
        sync::load_snapshot_file(p).map_err(|e| Fail::Usage(format!("--snap: {e}")))?;
    }
    let cap = a.frames.saturating_add(ORACLE_SLACK);
    // The start's oracle time (reaching it, then the settle) is `run.confirm_time`.
    let t_start = Instant::now();
    let start = oracle_at(a, cap).map_err(|e| Fail::Usage(format!("the start: {e}")))?;
    let run = oracle::settle(start, t_start).map_err(|e| Fail::Usage(format!("the start: {e}")))?;
    let facts = oracle::facts_at(&run, a.level);
    let prefix = run.inputs.len();

    // Guide scripts are played from the run's own start (DESIGN §4.2).
    let (mut opt, cost) = apply_guides(a, &files, &facts, prefix, base_options(a))?;
    let (oracle_extra, guide_frames) = (cost.oracle_time, cost.oracle_frames);

    let game = Game::with_display(
        sf_conform::display_data(),
        Start::Transplant(sync::core_memory(run.amiga.memory())),
    );
    map_compass(a, &game, &facts, &mut opt)?;

    let ro = run_options(a, t0);
    let mut stats = Stats {
        oracle_frames: run.oracle_frames + guide_frames,
        confirmed_frames: if a.engine_only { 0 } else { prefix as u32 },
        ..Stats::default()
    };
    // The share of wall-clock on the original (DESIGN §2.6): the start, the guide's replays and
    // the confirmation, over the whole run.
    let start_time = run.confirm_time;
    let share = |on_oracle: Duration| share(t0, on_oracle);

    if prefix >= a.frames as usize {
        // No frame left to explore: the start alone is the partial route.
        let ex = Explorer::new(game, facts, Options { max_frames: 0, ..opt });
        let (mut route, why) = ex.best_partial();
        route.inputs = run.inputs[..a.frames as usize].to_vec();
        stats.confirm_share = share(start_time + oracle_extra);
        let confirmed = (!a.engine_only).then_some(a.frames);
        return Ok(Outcome {
            inputs: route.inputs.clone(),
            exit: false,
            reason: Some(reason_line(&why, "frames exhausted (--frames)", confirmed)),
            route,
            stats,
        });
    }

    if a.engine_only {
        opt.max_frames = a.frames - prefix as u32;
        let r = plan_engine(game, facts, opt, &ro);
        let (route, reason) = match r {
            Ok(route) => (route, None),
            Err((route, why)) => {
                let w = if t0.elapsed() >= a.budget { "budget spent" } else { "frames or budget exhausted" };
                (route, Some(reason_line(&why, w, None)))
            }
        };
        stats.engine_frames = route.engine_frames;
        stats.confirm_share = share(start_time + oracle_extra);
        let mut inputs = run.inputs.clone();
        inputs.extend_from_slice(&route.inputs);
        return Ok(Outcome {
            exit: reason.is_none(),
            inputs,
            reason,
            route,
            stats,
        });
    }

    opt.max_frames = a.frames;
    // `plan_original`'s share is of its own wall-clock, the start's time included.
    let t_plan = Instant::now();
    let on_oracle = |s: &Stats| {
        oracle_extra + (start_time + t_plan.elapsed()).mul_f64(s.confirm_share as f64)
    };
    match oracle::plan_original(run, facts, opt, &ro) {
        Ok((route, mut s)) => {
            s.oracle_frames += guide_frames;
            s.confirm_share = share(on_oracle(&s));
            Ok(Outcome {
                inputs: route.inputs.clone(),
                exit: true,
                reason: None,
                route,
                stats: s,
            })
        }
        Err((route, mut s, why)) => {
            s.oracle_frames += guide_frames;
            s.confirm_share = share(on_oracle(&s));
            let w = if t0.elapsed() >= a.budget { "budget spent" } else { "frames or budget exhausted" };
            Ok(Outcome {
                inputs: route.inputs.clone(),
                exit: false,
                reason: Some(reason_line(&why, w, Some(s.confirmed_frames))),
                route,
                stats: s,
            })
        }
    }
}

fn main() -> ExitCode {
    let t0 = Instant::now();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let a = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sf-plan: {e}\n{USAGE}");
            eprintln!("PLAN_STATS error=usage hints=0");
            return ExitCode::from(2);
        }
    };
    let o = match run(&a, t0) {
        Ok(o) => o,
        Err(Fail::Usage(e)) => {
            eprintln!("sf-plan: {e}\n{USAGE}");
            eprintln!("PLAN_STATS error=usage hints={}", a.hints());
            return ExitCode::from(2);
        }
    };
    let text = script_text(&a, &o.inputs, o.exit);
    match &a.out {
        Some(p) => {
            // Temp then rename, so a reader never sees half a route.
            let tmp = p.with_extension(format!("tmp{}", std::process::id()));
            if let Err(e) = std::fs::write(&tmp, &text).and_then(|_| std::fs::rename(&tmp, p)) {
                eprintln!("sf-plan: {}: {e}", p.display());
                return ExitCode::from(2);
            }
        }
        None => print!("{text}"),
    }
    log_hints(&a, o.exit);
    if let Some(r) = &o.reason {
        eprintln!("{r}");
    }
    if a.compass == CompassArg::Walk {
        use std::sync::atomic::Ordering::Relaxed;
        eprintln!(
            "sf-plan: walk compass: explorer built in {:.1} ms at most, fills {} ({:.1} MB) at most",
            sf_plan::run::COMPASS_BUILD_NS.load(Relaxed) as f64 / 1e6,
            sf_plan::run::COMPASS_FILLS.load(Relaxed),
            sf_plan::run::COMPASS_FILL_BYTES.load(Relaxed) as f64 / 1e6,
        );
    }
    eprintln!(
        "PLAN_STATS engine_frames={} oracle_frames={} confirm_share={:.4} route_frames={} hearts_lost={} hints={} \
         guided={} repairs={} rounds={} cells={} peak_mem_mb={}",
        o.stats.engine_frames.max(o.route.engine_frames),
        o.stats.oracle_frames,
        o.stats.confirm_share,
        o.inputs.len(),
        o.route.hearts_lost,
        a.hints(),
        u8::from(a.guide.is_some()),
        o.stats.repairs,
        o.route.rounds,
        o.route.cells,
        peak_rss_bytes() >> 20,
    );
    ExitCode::from(if o.exit { 0 } else { 1 })
}
