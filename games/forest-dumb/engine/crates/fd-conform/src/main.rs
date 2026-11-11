//! `fd-conform run <case.toml>`: one case, prints the verdict and its report; exit 0 only when the
//! case passes. Format and how to run it: `engine/conform/README.md`.
//!
//! `fd-conform bug replay <zip> [--out DIR]`: a bug zip replayed headlessly (DESIGN §2.8,
//! `fd_conform::bug`): `frames N`, `stopped-at`, `replay-at` and `screenshot equal` or
//! `screenshot differs (N pixels)`, exit 0 either way; the last frame's picture as `last.png` in
//! `--out` when given. A zip another build made, or that cannot be read, is refused naming why,
//! exit 1 (DESIGN §2.12).
//!
//! `fd-conform bug compare <zip> [--out DIR]`: the recording played on the original and compared
//! (`fd_conform::bug::compare`, T15): the report, its first line `same`, `differs at frame F
//! (engine step S): ...` or `same up to frame F (engine step S), not judged after it`; the
//! pictures at Stop as `original-stop.png` and `engine-stop.png`, and the original's inputs as a
//! case script, `original-inputs.txt`, and the case that runs it, `bug-case.toml`, in `--out`, else the game's `build/bugs/<zip stem>/`. Exit 0 only for `same`. Procedure: `games/forest-dumb/docs/BUG-FIX.md`.
//!
//! `fd-conform code-writes [--cases <glob>] [--max-frames N]`: every level case under
//! `engine/conform/cases/` whose stem matches the glob (default all) run on the original from the
//! level sync point, reporting each change to the bytes of an instruction it executed
//! (`fd_conform::code_writes`, translator trial DESIGN §2.6). One JSON line per case,
//! `{"case","frames","executed","executed_bytes","changed":[{"at","frame","from","to","since"}]}`,
//! then `{"total":{"cases","frames","changed","misses"}}`. A case that does not start at level 1
//! is skipped with a line on stderr. Exit 0 when every case ran, changes or not; 1 for a missing
//! input; 2 for a wrong command line.
//!
//! `fd-conform code-seen [--cases DIR] [--power-on-frames N] [--worlds 2,3,4,5,6] [--out FILE]`:
//! every instruction the original executes over the level cases in DIR (default
//! `engine/conform/cases/`), the power-on run and the world starts, reduced to entries
//! (`fd_conform::code_seen`, translator DESIGN §2.3), written to FILE (default
//! `games/forest-dumb/re/entries.toml`). One JSON line per run, `{"run","frames","executed"}`, then
//! `{"total":{"runs","frames","instructions","entries","by_source","foreign","funcs_executed","funcs"}}`.
//! `--worlds none` runs no world start. Exit 0 when every run ran; 1 otherwise; 2 for a wrong
//! command line.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fd_conform::{Case, run};

const USAGE: &str = "usage: fd-conform run <case.toml> | bug replay <zip> [--out DIR] | bug compare <zip> [--out DIR] | code-writes [--cases <glob>] [--max-frames N] | code-seen [--cases DIR] [--power-on-frames N] [--worlds 2,3,4,5,6] [--out FILE]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match &args[..] {
        [cmd, path] if cmd == "run" => run_case(Path::new(path)),
        [cmd, rest @ ..] if cmd == "code-writes" => match code_writes_args(rest) {
            Some((glob, max)) => code_writes(&glob, max),
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

fn run_case(path: &Path) -> Result<bool, String> {
    let case = Case::load(path)?;
    let verdict = run(&case)?;
    println!("{}: {verdict}", case.name);
    Ok(verdict.passed())
}

fn bug_replay(zip: &Path, rest: &[String]) -> Result<bool, String> {
    use fd_conform::bug;
    let out = match rest {
        [] => None,
        [flag, dir] if flag == "--out" => Some(PathBuf::from(dir)),
        _ => return Err(USAGE.to_string()),
    };
    let bytes = std::fs::read(zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let (rec, screenshot) = bug::open_zip(&bytes, fd_tools::ENGINE_BUILD)
        .map_err(|e| format!("{}: {e}", zip.display()))?;
    let r = bug::replay(fd_conform::game_data()?, &rec, |_, _, _| {});
    println!("frames {}", rec.inputs.len());
    match rec.stopped_at {
        Some(at) => println!("stopped-at {at}"),
        None => println!("stopped-at -"),
    }
    println!("replay-at {}", r.location);
    match bug::screenshot_differs(&screenshot, &r.last)? {
        0 => println!("screenshot equal"),
        n => println!("screenshot differs ({n} pixels)"),
    }
    if let Some(dir) = out {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join("last.png");
        std::fs::write(&path, bug::picture_png(&r.last))
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

fn code_writes_args(rest: &[String]) -> Option<(String, Option<u32>)> {
    let (mut glob, mut max) = ("*".to_string(), None);
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--cases" => glob = it.next()?.clone(),
            "--max-frames" => max = Some(it.next()?.parse().ok()?),
            _ => return None,
        }
    }
    Some((glob, max))
}

/// The level cases matching `glob`, in parallel (each case its own oracle), printed in name order.
fn code_writes(glob: &str, max: Option<u32>) -> Result<bool, String> {
    use fd_conform::code_writes::{glob_match, run_case, total_json};
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
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<Result<_, String>>>> =
        cases.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..workers.min(cases.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(c) = cases.get(i) else { break };
                    let r = run_case(c, max, |_, _| {});
                    *results[i].lock().unwrap() = Some(r);
                }
            });
        }
    });
    let mut reports = Vec::new();
    for (c, r) in cases.iter().zip(results) {
        let r = r
            .into_inner()
            .unwrap()
            .unwrap()
            .map_err(|e| format!("{}: {e}", c.name))?;
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
