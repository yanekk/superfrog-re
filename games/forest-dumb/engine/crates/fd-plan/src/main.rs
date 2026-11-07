//! `fd-plan`: a route through W1-1 on the engine (DESIGN §2.11).
//!
//! ```text
//! fd-plan level W1-1 [--guide FILE] [--budget-mins N] [--out FILE] [--page DIR] [--rounds N]
//!         [--threads N] [--seed N] [--frames N] [--compass walk|straight] [--no-confirm]
//! ```
//!
//! Plans from the level's start, writes the route as an input script (`--out`, by default
//! `games/forest-dumb/build/routes/W1-1.txt`) and the map page with it drawn (`--page`, by default
//! `fd-routeview`'s folder), then plays it on the original and says how far the two agree.
//!
//! Exit 0: a route to the exit. Exit 1: the budget (`--budget-mins`, 30 by default) or the rounds
//! ran out first; the furthest route is written and drawn, and stderr says where it stopped and why
//! (a stall goes to the owner to play: games/superfrog/docs/planner-stall-ask-owner.md). Exit 2: bad
//! arguments, a guide that does not parse, or missing disks. The last stdout line is `PLAN_STATS …`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use amiga_plan::guide_file::parse_waypoints;
use amiga_plan::run::{RunOptions, peak_rss_bytes};
use amiga_plan::{Compass, GuidePoint, Options};
use fd_conform::Outcome;
use fd_plan::{confirm, plan, script_text};

const USAGE: &str = "usage: fd-plan level W1-1 [--guide FILE] [--budget-mins N] [--out FILE] \
[--page DIR] [--rounds N] [--threads N] [--seed N] [--frames N] [--compass walk|straight] [--no-confirm]";

#[derive(Debug)]
struct Args {
    guide: Option<PathBuf>,
    budget: Duration,
    out: PathBuf,
    page: PathBuf,
    rounds: Option<u64>,
    threads: usize,
    seed: u64,
    frames: u32,
    walk: bool,
    confirm: bool,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut it = argv.iter();
    match (it.next().map(String::as_str), it.next().map(String::as_str)) {
        (Some("level"), Some("W1-1")) => {}
        (Some("level"), Some(l)) => return Err(format!("level {l}: only W1-1 is built")),
        _ => return Err("want `level W1-1`".into()),
    }
    let game = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let game = game.canonicalize().unwrap_or(game);
    let mut a = Args {
        guide: None,
        budget: Duration::from_secs(30 * 60),
        out: game.join("build/routes/W1-1.txt"),
        page: fd_routeview::default_out(),
        rounds: None,
        threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
        seed: 1,
        frames: 30_000,
        walk: true,
        confirm: true,
    };
    while let Some(k) = it.next() {
        let mut val = || it.next().cloned().ok_or(format!("{k} wants a value"));
        let int = |v: String| {
            v.parse::<u64>()
                .map_err(|_| format!("{k} {v}: not a whole number"))
        };
        match k.as_str() {
            "--guide" => a.guide = Some(val()?.into()),
            "--budget-mins" => {
                let v = val()?;
                let m: f64 = v.parse().map_err(|_| format!("{k} {v}: not a number"))?;
                if !(m >= 0.0 && m.is_finite()) {
                    return Err(format!("{k} {v}: not a number of minutes"));
                }
                a.budget = Duration::from_secs_f64(m * 60.0);
            }
            "--out" => a.out = val()?.into(),
            "--page" => a.page = val()?.into(),
            "--rounds" => a.rounds = Some(int(val()?)?),
            "--threads" => a.threads = int(val()?)?.max(1) as usize,
            "--seed" => a.seed = int(val()?)?,
            "--frames" => a.frames = int(val()?)?.min(u32::MAX as u64) as u32,
            "--compass" => {
                a.walk = match val()?.as_str() {
                    "walk" => true,
                    "straight" => false,
                    v => return Err(format!("{k} {v}: walk or straight")),
                }
            }
            "--no-confirm" => a.confirm = false,
            s => return Err(format!("unknown argument {s}")),
        }
    }
    Ok(a)
}

fn guide(path: &Path) -> Result<Vec<GuidePoint>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_waypoints(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let a = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("fd-plan: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    // The guide is read before the disks, so a bad one is named at once.
    let guide = match a.guide.as_deref().map(guide).transpose() {
        Ok(g) => g.unwrap_or_default(),
        Err(e) => {
            eprintln!("fd-plan: guide {e}");
            return ExitCode::from(2);
        }
    };
    let data = match fd_conform::game_data() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("fd-plan: {e}");
            return ExitCode::from(2);
        }
    };
    let t0 = Instant::now();
    let opt = Options {
        seed: a.seed,
        max_frames: a.frames,
        compass: if a.walk {
            Compass::Walk
        } else {
            Compass::Straight
        },
        guide,
        ..Options::default()
    };
    let run = RunOptions {
        threads: a.threads,
        budget: a.budget,
        max_rounds: a.rounds,
        ..RunOptions::default()
    };
    let p = plan(&data, opt, &run);
    let wall = t0.elapsed().as_secs_f64();
    let r = &p.route;
    let mut notes = vec![
        format!(
            "fd-plan level W1-1, seed {}, {} rounds, {} engine frames, {wall:.1} s",
            a.seed, r.rounds, r.engine_frames
        ),
        match &p.stalled {
            None => format!("reaches the exit, {} frames", r.inputs.len()),
            Some(_) => format!("stalled: ends at {} after {} frames", p.end, r.inputs.len()),
        },
    ];
    let mut agreed = String::from("not played on the original (--no-confirm)");
    if a.confirm {
        agreed = match confirm::confirm(&data, &r.inputs) {
            Ok(v) => match &v.outcome {
                Outcome::Same => format!("the original agrees on all {} passes", v.passes),
                Outcome::Differs { pass, var, .. } => format!(
                    "the original agrees for {pass} of {} passes, then {var} differs",
                    r.inputs.len()
                ),
                o => format!("on the original: {o:?}"),
            },
            Err(e) => format!("not played on the original: {e}"),
        };
        notes.push(agreed.clone());
    }
    let text = script_text(&notes, &r.inputs);
    if let Some(dir) = a.out.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&a.out, &text) {
        eprintln!("fd-plan: {}: {e}", a.out.display());
        return ExitCode::from(2);
    }
    match fd_routeview::write_page(&data, &a.out, &a.page) {
        Ok((page, _)) => println!("page: {}", page.display()),
        Err(e) => eprintln!("fd-plan: the page: {e}"),
    }
    println!(
        "route: {} ({} frames, ends at {})",
        a.out.display(),
        r.inputs.len(),
        p.end
    );
    println!("{agreed}");
    let code = match &p.stalled {
        None => 0,
        Some(why) => {
            eprintln!(
                "fd-plan: stalled at {} ({} items left, {} cells tried, nearest the exit {}): the page shows where; \
                 a person playing that stretch is the next step (DESIGN §2.11)",
                p.end,
                why.coins_left,
                why.cells,
                why.exit_distance_with_coins_met
                    .map_or("never with the items met".into(), |d| format!("{d} cells")),
            );
            1
        }
    };
    println!(
        "PLAN_STATS frames={} rounds={} engine_frames={} cells={} wall_s={wall:.1} peak_rss_mb={}",
        r.inputs.len(),
        r.rounds,
        r.engine_frames,
        r.cells,
        peak_rss_bytes() >> 20
    );
    ExitCode::from(code)
}
