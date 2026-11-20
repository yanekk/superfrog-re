//! `fd-conform run <case.toml> [--engine hand|translated] [--mount level-1]`: one case, prints the
//! verdict and its report; exit 0 only when the case passes. Format and how to run it:
//! `engine/conform/README.md`. `--engine translated` runs a level case on the translated program
//! judged on the frame bar (`fd_conform::translated_engine::run_translated`, translator T29); `hand`
//! (the default) is fd-core's engine. `--mount level-1` runs the translated program with level 1's
//! hand engine mounted through the seam (translator T36); it needs `--engine translated`.
//!
//! `fd-conform run-all [--engine hand|translated] [--mount level-1] [--cases <glob>] [--report FILE]`: every case
//! under `engine/conform/cases/` whose stem matches the glob, in parallel, one line each
//! (`<case>: <verdict's first line>`), then a summary; `--report` also writes it as Markdown,
//! grouped by the task that works from it and sorted by first difference (`fd_conform::reading`).
//! A case the engine cannot run (a title case on `translated`) is listed as not run. Exit 0 only
//! when every case ran and passed.
//!
//! `fd-conform bug replay <zip> [--out DIR] [--engine hand|translated]`: a bug zip replayed
//! headlessly (DESIGN §2.8, `fd_conform::bug`) on the engine that played it, or the one named (a
//! power-on recording replays on the translated program only): `frames N`, `start`, `engine`,
//! `stopped-at`, `replay-at` (`-` out of level 1), `stop` (the recording's), on the translated
//! program `replay-stop <stop> at frame K` or `replay-stop -` (translator T39), and `screenshot
//! equal` or `screenshot differs (N pixels)`, exit 0 either way; the last frame's picture as `last.png` in
//! `--out` when given. A zip another build made, or that cannot be read, is refused naming why,
//! exit 1 (DESIGN §2.12).
//!
//! `fd-conform bug compare <zip> [--out DIR]`: the recording played on the original and compared
//! (`fd_conform::bug::compare`, T15): the report, its first line `same`, `differs at frame F
//! (engine step S): ...` or `same up to frame F (engine step S), not judged after it`; the
//! pictures at Stop as `original-stop.png` and `engine-stop.png`, and the original's inputs as a
//! case script, `original-inputs.txt`, and the case that runs it, `bug-case.toml`, in `--out`, else the game's `build/bugs/<zip stem>/`. Exit 0 only for `same`. Procedure: `games/forest-dumb/docs/BUG-FIX.md`.
//!
//! `fd-conform code-writes [--cases <glob>] [--max-frames N] [--all]`: every level case under
//! `engine/conform/cases/` whose stem matches the glob (default all) run on the original from the
//! level sync point, reporting each change to the bytes of an instruction it executed
//! (`fd_conform::code_writes`, translator trial DESIGN §2.6). One JSON line per case,
//! `{"case","frames","executed","executed_bytes","changed":[{"at","frame","from","to","since"}]}`,
//! then `{"total":{"cases","frames","changed","misses"}}`. A case that does not start at level 1
//! is skipped with a line on stderr. `--all` adds every other path the translator covers (translator
//! DESIGN §2.5, T15): the power-on run through the title into level 1 (`power-on`, 7 000 frames) and
//! each world 2 to 6 start (`world-2` .. `world-6`), as `code-seen` runs them, after the cases;
//! `--max-frames` bounds only the cases. Exit 0 when every run ran, changes or not; 1 for a
//! missing input; 2 for a wrong command line.
//!
//! `fd-conform code-seen [--cases DIR] [--power-on-frames N] [--worlds 2,3,4,5,6] [--out FILE]`:
//! every instruction the original executes over the level cases in DIR (default
//! `engine/conform/cases/`), the power-on run and the world starts, reduced to entries
//! (`fd_conform::code_seen`, translator DESIGN §2.3), written to FILE (default
//! `games/forest-dumb/re/entries.toml`). One JSON line per run, `{"run","frames","executed"}`, then
//! `{"total":{"runs","frames","instructions","entries","by_source","foreign","funcs_executed","funcs"}}`.
//! `--worlds none` runs no world start. Exit 0 when every run ran; 1 otherwise; 2 for a wrong
//! command line.
//!
//! `fd-conform os-surface [--frames N] [--worlds 1,2] [--out FILE]`: where the program leans on the
//! OS, measured on the original from power-on through the title, Start, level 1's load and `N`
//! frames of play (default 300), and each other world's load asked for (`fd_conform::os_surface`,
//! plans/forest-dumb-translator T01). Writes `games/forest-dumb/re/os-calls.toml` (or `--out`)
//! through a temporary file, sorted and stable, and prints its path. Exit 0 when written; 1 for a
//! missing input or a world whose load never ran; 2 for a wrong command line.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fd_conform::Case;
use fd_conform::reading::{EngineKind, run_mounted};

const USAGE: &str = "usage: fd-conform run <case.toml> [--engine hand|translated] [--mount level-1] | run-all [--engine hand|translated] [--mount level-1] [--cases <glob>] [--report FILE] | bug replay <zip> [--out DIR] [--engine hand|translated] | bug compare <zip> [--out DIR] | code-writes [--cases <glob>] [--max-frames N] [--all] | code-seen [--cases DIR] [--power-on-frames N] [--worlds 2,3,4,5,6] [--out FILE] | os-surface [--frames N] [--worlds 1,2] [--out FILE]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match &args[..] {
        [cmd, path, rest @ ..] if cmd == "run" && !path.starts_with("--") => {
            match engine_arg(rest) {
                Some(engine) => run_case(Path::new(path), engine),
                None => {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                }
            }
        }
        [cmd, rest @ ..] if cmd == "run-all" => match run_all_args(rest) {
            Some((engine, glob, report)) => run_all(engine, &glob, report.as_deref()),
            None => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        },
        [cmd, rest @ ..] if cmd == "code-writes" => match code_writes_args(rest) {
            Some((glob, max, all)) => code_writes(&glob, max, all),
            None => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        },
        [cmd, rest @ ..] if cmd == "code-seen" => match code_seen_args(rest) {
            Some((opts, out)) => code_seen(&opts, &out),
            None => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        },
        [cmd, rest @ ..] if cmd == "os-surface" => match os_surface_args(rest) {
            Some((opts, out)) => os_surface(&opts, out),
            None => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        },
        [bug, replay, zip, rest @ ..] if bug == "bug" && replay == "replay" => {
            bug_replay(Path::new(zip), rest)
        }
        [bug, compare, zip, rest @ ..] if bug == "bug" && compare == "compare" => {
            bug_compare(Path::new(zip), rest)
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match outcome {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("fd-conform: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_case(path: &Path, (engine, mount): (EngineKind, bool)) -> Result<bool, String> {
    let case = Case::load(path)?;
    let verdict = run_mounted(&case, engine, mount)?;
    println!("{}: {verdict}", case.name);
    Ok(verdict.passed())
}

fn engine_arg(rest: &[String]) -> Option<(EngineKind, bool)> {
    let (mut engine, mut mount) = (EngineKind::Hand, false);
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--engine" => engine = EngineKind::parse(it.next()?)?,
            "--mount" => mount = mount_arg(it.next()?)?,
            _ => return None,
        }
    }
    (!mount || engine == EngineKind::Translated).then_some((engine, mount))
}

/// `--mount`'s value: `level-1`, the one mount there is.
fn mount_arg(s: &str) -> Option<bool> {
    (s == "level-1").then_some(true)
}

fn run_all_args(rest: &[String]) -> Option<((EngineKind, bool), String, Option<PathBuf>)> {
    let (mut engine, mut mount, mut glob, mut report) =
        (EngineKind::Hand, false, "*".to_string(), None);
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--engine" => engine = EngineKind::parse(it.next()?)?,
            "--mount" => mount = mount_arg(it.next()?)?,
            "--cases" => glob = it.next()?.clone(),
            "--report" => report = Some(PathBuf::from(it.next()?)),
            _ => return None,
        }
    }
    if mount && engine != EngineKind::Translated {
        return None;
    }
    Some(((engine, mount), glob, report))
}

/// Every matching case on `engine`, in parallel (each run its own oracle), printed in name order.
fn run_all(
    (engine, mount): (EngineKind, bool),
    glob: &str,
    report: Option<&Path>,
) -> Result<bool, String> {
    use fd_conform::code_writes::glob_match;
    use fd_conform::reading::{Reading, Row};
    let dir = fd_conform::conform_dir().join("cases");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .filter(|p| {
            p.file_stem()
                .is_some_and(|s| glob_match(glob, &s.to_string_lossy()))
        })
        .collect();
    paths.sort();
    let cases = paths
        .iter()
        .map(|p| Case::load(p))
        .collect::<Result<Vec<_>, _>>()?;
    if cases.is_empty() {
        return Err(format!("no case in {} matches `{glob}`", dir.display()));
    }
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<Result<fd_conform::Verdict, String>>>> =
        cases.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..workers.min(cases.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(case) = cases.get(i) else { break };
                    let r = run_mounted(case, engine, mount);
                    let line = match &r {
                        Ok(v) => v.to_string().lines().next().unwrap_or("").to_string(),
                        Err(e) => format!("not run: {e}"),
                    };
                    eprintln!("{}: {line}", case.name);
                    *results[i].lock().unwrap() = Some(r);
                }
            });
        }
    });
    let rows: Vec<Row> = cases
        .iter()
        .zip(results)
        .map(|(c, r)| Row::new(&c.name, r.into_inner().unwrap().expect("every case ran")))
        .collect();
    let reading = Reading {
        engine: if mount {
            "translated, level 1 mounted"
        } else {
            engine.name()
        },
        rows,
    };
    for r in &reading.rows {
        println!("{}: {}", r.case, r.result());
    }
    println!("{}", reading.summary());
    if let Some(path) = report {
        let tmp = path.with_extension("md.tmp");
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&tmp, reading.markdown()).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(reading.all_passed())
}

fn bug_replay(zip: &Path, rest: &[String]) -> Result<bool, String> {
    use fd_conform::bug;
    use fd_core::recording::{EngineKind as Played, StartPoint};
    let (mut out, mut engine) = (None, None);
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match (flag.as_str(), it.next().map(String::as_str)) {
            ("--out", Some(dir)) => out = Some(PathBuf::from(dir)),
            ("--engine", Some("hand")) => engine = Some(Played::Hand),
            ("--engine", Some("translated")) => engine = Some(Played::Translated),
            _ => return Err(USAGE.to_string()),
        }
    }
    let bytes = std::fs::read(zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let (rec, screenshot) = bug::open_zip(&bytes, fd_tools::ENGINE_BUILD)
        .map_err(|e| format!("{}: {e}", zip.display()))?;
    // The engine that played it, unless asked for the other (a level-1 start plays on either).
    let engine = engine.unwrap_or(rec.engine);
    if (engine, rec.start) == (Played::Hand, StartPoint::PowerOn) {
        return Err(format!(
            "{}: a power-on recording replays on the translated program only (--engine translated)",
            zip.display()
        ));
    }
    let place =
        |l: Option<fd_core::recording::Location>| l.map_or("-".to_string(), |l| l.to_string());
    let (location, last, stop) = match engine {
        Played::Hand => {
            let r = bug::replay(fd_conform::game_data()?, &rec, |_, _, _| {});
            (Some(r.location), r.last, None)
        }
        Played::Translated => {
            let disks = fd_conform::disks::load()?;
            let r = bug::replay_translated(&disks, fd_conform::game_data()?, &rec)?;
            (r.location, r.last, Some(r.stop))
        }
    };
    println!("frames {}", rec.inputs.len());
    println!("start {}", rec.start.name());
    println!("engine {}", engine.name());
    println!("stopped-at {}", place(rec.stopped_at));
    println!("replay-at {}", place(location));
    println!("stop {}", rec.stop.as_deref().unwrap_or("-"));
    match stop {
        None => {}
        Some(None) => println!("replay-stop -"),
        Some(Some((frame, s))) => println!("replay-stop {s} at frame {frame}"),
    }
    match bug::screenshot_differs(&screenshot, &last)? {
        0 => println!("screenshot equal"),
        n => println!("screenshot differs ({n} pixels)"),
    }
    if let Some(dir) = out {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join("last.png");
        std::fs::write(&path, bug::picture_png(&last))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(true)
}

fn bug_compare(zip: &Path, rest: &[String]) -> Result<bool, String> {
    use fd_conform::bug;
    let out = match rest {
        [] => bug::default_out(zip)?,
        [flag, dir] if flag == "--out" => PathBuf::from(dir),
        _ => return Err(USAGE.to_string()),
    };
    let bytes = std::fs::read(zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let (rec, _) = bug::open_zip(&bytes, fd_tools::ENGINE_BUILD)
        .map_err(|e| format!("{}: {e}", zip.display()))?;
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    if rec.start == fd_core::recording::StartPoint::PowerOn {
        return Err(format!(
            "{}: a power-on recording starts before the level sync point bug compare starts at; \
             replay it with `bug replay`",
            zip.display()
        ));
    }
    let c = bug::compare(fd_conform::game_data()?, &rec, Some(out.clone()))?;
    print!("{}", c.report());
    let write = |name: &str, bytes: Vec<u8>| {
        let path = out.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{name} {}", path.display());
        Ok::<_, String>(())
    };
    write("original-stop.png", bug::frame_png(&c.retimed.stop))?;
    write("engine-stop.png", bug::picture_png(&c.engine_stop))?;
    // What the original was fed, from the sync point: a case's `input` script, so a fix is
    // checked by a case once the zip no longer replays on the fixed build (BUG-FIX.md).
    let note = rec.note.split_whitespace().collect::<Vec<_>>().join(" ");
    write(
        "original-inputs.txt",
        format!(
            "# {}: the bug zip's inputs on the original's frames from the level sync point \
             (`fd-conform bug compare`), {} frames judged.\n{}",
            if note.is_empty() {
                "(no note)"
            } else {
                note.as_str()
            },
            c.retimed.frames,
            c.retimed.script.to_text()
        )
        .into_bytes(),
    )?;
    let note = if note.is_empty() { "(no note)" } else { &note };
    write("bug-case.toml", c.case_text("bug-NAME", note).into_bytes())?;
    Ok(c.same())
}

fn os_surface_args(rest: &[String]) -> Option<(fd_conform::os_surface::Options, Option<PathBuf>)> {
    let mut opts = fd_conform::os_surface::Options::default();
    let mut out = None;
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--frames" => opts.frames = it.next()?.parse().ok()?,
            "--worlds" => {
                opts.worlds = it
                    .next()?
                    .split(',')
                    .map(|w| w.trim().parse().ok())
                    .collect::<Option<Vec<u32>>>()?;
            }
            "--out" => out = Some(PathBuf::from(it.next()?)),
            _ => return None,
        }
    }
    Some((opts, out))
}

/// The survey, written through a temporary file and a rename.
fn os_surface(
    opts: &fd_conform::os_surface::Options,
    out: Option<PathBuf>,
) -> Result<bool, String> {
    let r = fd_conform::os_surface::run(opts)?;
    let path = out.unwrap_or_else(fd_conform::os_surface::committed_path);
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, r.toml(opts.frames, &opts.worlds))
        .map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{}", path.display());
    Ok(true)
}

fn code_writes_args(rest: &[String]) -> Option<(String, Option<u32>, bool)> {
    let (mut glob, mut max, mut all) = ("*".to_string(), None, false);
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--cases" => glob = it.next()?.clone(),
            "--max-frames" => max = Some(it.next()?.parse().ok()?),
            "--all" => all = true,
            _ => return None,
        }
    }
    Some((glob, max, all))
}

/// The level cases matching `glob`, and with `all` the power-on run and the world starts, in
/// parallel (each run its own oracle), printed cases in name order first.
fn code_writes(glob: &str, max: Option<u32>, all: bool) -> Result<bool, String> {
    use fd_conform::code_seen::POWER_ON_FRAMES;
    use fd_conform::code_writes::{
        glob_match, run_case, run_power_on_in, run_world_in, total_json,
    };
    let dir = fd_conform::conform_dir().join("cases");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .filter(|p| {
            p.file_stem()
                .is_some_and(|s| glob_match(glob, &s.to_string_lossy()))
        })
        .collect();
    paths.sort();
    let mut cases = Vec::new();
    for p in &paths {
        let c = Case::load(p)?;
        if c.is_level() {
            cases.push(c);
        } else {
            eprintln!("{}: skipped, it does not start at level 1", c.name);
        }
    }
    if cases.is_empty() {
        return Err(format!(
            "no level case in {} matches `{glob}`",
            dir.display()
        ));
    }
    enum Job<'c> {
        Case(&'c Case),
        PowerOn,
        World(u32),
    }
    let mut jobs: Vec<Job> = cases.iter().map(Job::Case).collect();
    if all {
        jobs.push(Job::PowerOn);
        jobs.extend((2..=6).map(Job::World));
    }
    let name = |j: &Job| match j {
        Job::Case(c) => c.name.clone(),
        Job::PowerOn => "power-on".to_string(),
        Job::World(w) => format!("world-{w}"),
    };
    let (kickstart, disks) = if all {
        let k = amiga_oracle::kickstart_path().map_err(|e| e.to_string())?;
        (k, fd_tools::disk_dir()?)
    } else {
        Default::default()
    };
    let cache = fd_conform::sync::cache_dir();
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<Result<_, String>>>> =
        jobs.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..workers.min(jobs.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(job) = jobs.get(i) else { break };
                    let r = match job {
                        Job::Case(c) => run_case(c, max, |_, _| {}),
                        Job::PowerOn => run_power_on_in(&kickstart, &disks, POWER_ON_FRAMES),
                        Job::World(w) => run_world_in(&kickstart, &disks, &cache, *w),
                    };
                    *results[i].lock().unwrap() = Some(r);
                }
            });
        }
    });
    let mut reports = Vec::new();
    for (j, r) in jobs.iter().zip(results) {
        let r = r
            .into_inner()
            .unwrap()
            .unwrap()
            .map_err(|e| format!("{}: {e}", name(j)))?;
        println!("{}", r.json());
        reports.push(r);
    }
    println!("{}", total_json(&reports));
    Ok(true)
}

fn code_seen_args(rest: &[String]) -> Option<(fd_conform::code_seen::Options, PathBuf)> {
    use fd_conform::code_seen::{Options, re_dir};
    let mut opts = Options::default();
    let mut out = re_dir().join("entries.toml");
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--cases" => opts.cases = PathBuf::from(it.next()?),
            "--power-on-frames" => opts.power_on_frames = it.next()?.parse().ok()?,
            "--worlds" => {
                let v = it.next()?;
                opts.worlds = if v == "none" {
                    vec![]
                } else {
                    v.split(',')
                        .map(|w| w.trim().parse().ok().filter(|w| (2..=6).contains(w)))
                        .collect::<Option<Vec<u32>>>()?
                };
            }
            "--out" => out = PathBuf::from(it.next()?),
            _ => return None,
        }
    }
    Some((opts, out))
}

/// Every run, its entries written to `out`.
fn code_seen(opts: &fd_conform::code_seen::Options, out: &Path) -> Result<bool, String> {
    use fd_conform::code_seen::{
        collect_in, entries, funcs, parse_symbols, re_dir, render, summary_json, write_atomic,
    };
    let kickstart = amiga_oracle::kickstart_path().map_err(|e| e.to_string())?;
    let disks = fd_tools::disk_dir()?;
    let mut skipped = Vec::new();
    let (seen, hunks) = collect_in(
        &kickstart,
        &disks,
        &fd_conform::sync::cache_dir(),
        opts,
        &mut skipped,
    )?;
    for s in &skipped {
        eprintln!("{s}: skipped, it does not start at level 1");
    }
    let path = re_dir().join("symbols.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let funcs = funcs(&parse_symbols(&text)?);
    let e = entries(&seen.insns, &funcs, &seen.vectors, hunks[0].0);
    write_atomic(out, &render(&e, &hunks))?;
    for (run, frames, executed) in &seen.runs {
        println!(r#"{{"run":"{run}","frames":{frames},"executed":{executed}}}"#);
    }
    println!("{}", summary_json(&seen, &e, &funcs));
    Ok(true)
}
