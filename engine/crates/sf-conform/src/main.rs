//! `sf-conform run <case.toml>...` — cases; exit 1 on a divergence.
//! `sf-conform all` — every case in `engine/conform/cases/`.
//! `sf-conform scan <case.toml>` — one case run to its end past each divergence: one line per
//! frame that differs (what differs first in it), for finding what is left.
//! `run`, `all` and `scan` judge faithfully (DESIGN §2.1 checks 1–4) and print per case the
//! picture frames and sound shifts they tolerated, the re-syncs and the milestones the engine met
//! run alone (check 4: every case that steps the engine); `--strict` judges frame-exact instead,
//! for diagnosis, without check 4. `run --alone` judges by check 4 only: checks 1–3 end a run at
//! their first failure, so a case red there never reaches it.
//! `sf-conform feed <case.toml> [--frames N]` — the pacing feed of a case from power-on (T211):
//! what check 4's engine run alone is fed, and its frame hashes, written into the trace cache for
//! the app to be fed the same (`sf_conform::feed`); prints the two files. `--frames N` cuts the
//! case to its first N frames.
//! `sf-conform channels <case.toml>...` — each of Paula's channels' output against the
//! original's over whole cases from power-on (T213, `sf_conform::channels`): the original's as
//! recorded with the case's pacing feed, the engine's from the feed replayed through the app's
//! mixer; exit 1 where a channel differs longer than the tolerance allows.
//! `sf-conform record <case.toml> [--watch NAME=ADDR:W]... [--every] [--only] [--all-vars] [--max-mb N] [--trace] [--paula] [--png DIR]`
//! — the original's state per frame of a case, for writing scripts and porting: every `LAYOUT`
//! variable up to 1 KB long (every one with `--all-vars`, none with `--only`) and each watched
//! address (`h0+9d8:W`, `0xc17074:L`, width B, W or L), printed on frames where something changed. `--trace` adds where the main program was at
//! the frame's end (its PC and the return addresses on its stack, as `h0+$x`), `--png` writes the
//! original's picture of every printed frame to `DIR/frame<N>.png`, `--paula` its Paula writes
//! (register=value@line). It stops with an error once it has written 200 MB (`--max-mb N`).
//! `sf-conform bug replay <zip> [--out DIR] [--frames A..B] [--every N]` — a bug zip replayed
//! headlessly (bug-recorder DESIGN §2.7, `sf_conform::bug`): the frames asked (A up to B, B left
//! out; default the last 250) written as `frame-NNNNNN.png` to DIR (default
//! `~/.cache/superfrog-re/bugs/<zip stem>/`), then `frames N`, `stopped-at`, `replay-at` and
//! whether the last picture equals the zip's screenshot. Exit 0 whatever the comparison says.
//! `sf-conform [--strict] bug compare <zip> [--out DIR]` — the zip's start and inputs played on
//! the original (bug-recorder DESIGN §2.8, §2.9): the inputs retimed onto the original's frames,
//! judged faithfully (frame-exact with `--strict`) in legs of 8 passes; prints `same` or `differs
//! at frame F (engine step S): …`, where the original left the recording if it did, and the
//! legs; writes `original-stop.png` and `engine-stop.png` to DIR (default the zip's folder).
//! Exit 0 for same, 1 for differs.
//! `sf-conform bug case <zip> <name>` — the zip as a permanent case:
//! `engine/conform/cases/<name>.toml` and `engine/conform/scripts/<name>.txt`, its inputs in the
//! original's frames; refuses a name either folder has.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sf_conform::{Alone, Case, InputScript, Mode, cases_dir, sync};
use sf_core::layout::{LAYOUT, Ram, VarDef, VarId, Width, h0};

const USAGE: &str = "usage: sf-conform [--strict | --alone] run <case.toml>... | [--strict] all | [--strict] scan <case.toml> | feed <case.toml> [--frames N] | channels <case.toml>... | record <case.toml> [--watch NAME=ADDR:W]... [--every] [--only] [--all-vars] [--max-mb N] [--trace] [--paula] [--png DIR] | bug replay <zip> [--out DIR] [--frames A..B] [--every N] | [--strict] bug compare <zip> [--out DIR] | bug case <zip> <name>";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let strict =
        args.first().is_some_and(|a| a != "record") && args.iter().any(|a| a == "--strict");
    args.retain(|a| !strict || a != "--strict");
    let mode = if strict { Mode::Strict } else { Mode::Faithful };
    let alone =
        !strict && args.first().is_some_and(|a| a == "run") && args.iter().any(|a| a == "--alone");
    args.retain(|a| !alone || a != "--alone");
    let result = match args.first().map(String::as_str) {
        Some("run") if args.len() >= 2 => run_cases(
            &args[1..].iter().map(PathBuf::from).collect::<Vec<_>>(),
            mode,
            alone,
        ),
        Some("all") if args.len() == 1 => all(mode),
        Some("scan") if args.len() == 2 => scan(Path::new(&args[1]), mode),
        Some("feed") if args.len() == 2 || args.len() == 4 && args[2] == "--frames" => {
            feed(Path::new(&args[1]), args.get(3))
        }
        Some("channels") if args.len() >= 2 => channels(&args[1..]),
        Some("record") if args.len() >= 2 => record(Path::new(&args[1]), &args[2..]),
        Some("bug") if args.len() >= 3 && args[1] == "replay" => {
            bug_replay(Path::new(&args[2]), &args[3..])
        }
        Some("bug") if args.len() >= 3 && args[1] == "compare" => {
            bug_compare(Path::new(&args[2]), &args[3..], mode)
        }
        Some("bug") if args.len() == 4 && args[1] == "case" => {
            bug_case(Path::new(&args[2]), &args[3])
        }
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn bug_replay(zip: &Path, rest: &[String]) -> Result<bool, String> {
    use sf_conform::bug;
    let (mut out, mut range, mut every) = (None, None, None);
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        let slot = match a.as_str() {
            "--out" => &mut out,
            "--frames" => &mut range,
            "--every" => &mut every,
            _ => return Err(USAGE.to_string()),
        };
        *slot = Some(it.next().ok_or(USAGE)?.as_str());
    }
    let bytes = std::fs::read(zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let (rec, screenshot) = bug::read_zip(&bytes).map_err(|e| format!("{}: {e}", zip.display()))?;
    let pick = bug::FramePick::parse(range, every, rec.inputs.len() as u32)?;
    let dir = match out {
        Some(d) => PathBuf::from(d),
        None => bug::default_out(zip)?,
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut failed = None;
    let r = bug::replay(&rec, |f, _, pic| {
        if failed.is_none() && pick.wants(f) {
            let path = dir.join(format!("frame-{f:06}.png"));
            if let Err(e) = std::fs::write(&path, bug::picture_png(pic)) {
                failed = Some(format!("{}: {e}", path.display()));
            }
        }
    });
    if let Some(e) = failed {
        return Err(e);
    }
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
    Ok(true)
}

fn bug_compare(zip: &Path, rest: &[String], mode: Mode) -> Result<bool, String> {
    use sf_conform::bug;
    let out = match rest {
        [] => None,
        [flag, dir] if flag == "--out" => Some(PathBuf::from(dir)),
        _ => return Err(USAGE.to_string()),
    };
    let bytes = std::fs::read(zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let (rec, _) = bug::read_zip(&bytes).map_err(|e| format!("{}: {e}", zip.display()))?;
    // Beside the zip: the pictures are disk-derived, and the zip's folder, `bugs/`, is
    // git-ignored (RE §2.3; bug-recorder DESIGN §2.10 step 3).
    let dir = match out {
        Some(d) => d,
        None => match zip.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        },
    };
    let c = bug::compare(&rec, mode)?;
    print!("{}", c.report());
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let write = |name: &str, png: Vec<u8>| {
        let p = dir.join(name);
        std::fs::write(&p, png).map_err(|e| format!("{}: {e}", p.display()))?;
        println!("{}", p.display());
        Ok::<(), String>(())
    };
    if let Some(stop) = &c.retimed.stop {
        write("original-stop.png", bug::frame_png(stop))?;
    }
    write("engine-stop.png", bug::picture_png(&c.engine_stop))?;
    Ok(c.same())
}

fn bug_case(zip: &Path, name: &str) -> Result<bool, String> {
    use sf_conform::{bug, scripts_dir};
    let bytes = std::fs::read(zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let (rec, _) = bug::read_zip(&bytes).map_err(|e| format!("{}: {e}", zip.display()))?;
    let file = zip
        .file_name()
        .map_or("-".to_string(), |n| n.to_string_lossy().into_owned());
    let (case, script) = bug::write_case(&rec, name, &file, &cases_dir(), &scripts_dir())?;
    println!("{}", case.display());
    println!("{}", script.display());
    Ok(true)
}

fn feed(path: &Path, frames: Option<&String>) -> Result<bool, String> {
    let mut case = Case::load(path)?;
    if let Some(n) = frames {
        case.frames = n.parse().map_err(|_| format!("--frames {n:?}: not a whole number"))?;
    }
    let f = sf_conform::feed::feed(&case)?;
    println!("feed {}", f.feed.display());
    println!("hashes {}", f.hashes.display());
    println!("{} frames, {}", f.frames, if f.cached { "from the cache" } else { "run now" });
    if let Err(d) = &f.alone {
        println!("the engine run alone did not meet the original's milestones: {d}");
    }
    Ok(true)
}

fn channels(paths: &[String]) -> Result<bool, String> {
    let tol = sf_conform::channels::OWNER;
    let mut green = true;
    for p in paths {
        let case = Case::load(Path::new(p))?;
        let v = sf_conform::channels::check(&case, tol)?;
        let ok = v.runs.is_empty();
        green &= ok;
        println!(
            "{} {}: {} frames of play compared ({} with the engine's sound shifted, at most {} frames; {} cut off), differing {:?} per channel; longest: {}",
            if ok { "ok" } else { "FAILED" },
            case.name,
            v.compared,
            v.shifted,
            v.furthest,
            v.cut_off,
            v.differing,
            v.longest
        );
        for r in &v.runs {
            println!("  past the tolerance ({tol:?}): {r}");
        }
        for (r, task) in &v.known {
            println!("  known, in a stretch the case lists ({task}): {r}");
        }
    }
    Ok(green)
}

fn scan(path: &Path, mode: Mode) -> Result<bool, String> {
    let case = Case {
        mode,
        ..Case::load(path)?
    };
    let (found, o) = sf_conform::scan_counting(&case);
    for d in &found {
        let what = if let Some((v, theirs, ours)) = &d.first_var {
            format!(
                "var {} at ${:06x}: original ${theirs:x}, engine ${ours:x}",
                v.name, v.addr
            )
        } else if let Some(p) = &d.paula_diff {
            format!("paula {}", p.lines().next().unwrap_or(""))
        } else if let Some(s) = &d.sound_diff {
            format!("sound {s}")
        } else if let Some((theirs, ours)) = d.passes_apart {
            format!("passes apart: original {theirs}, engine {ours}")
        } else if let Some(m) = &d.milestone {
            format!("plays alone: {m}")
        } else if let Some((from, limit)) = d.lag {
            format!("mid-pass from frame {from}, past the limit of {limit} frames")
        } else if let Some(p) = &d.picture_diff {
            match &d.picture_note {
                Some(why) => format!("picture {:?}: {why}", p.mismatch.first),
                None => format!("picture {:?}", p.mismatch.first),
            }
        } else {
            String::from("unported")
        };
        let of = d
            .oracle_frame
            .map_or(String::new(), |o| format!(" (O {o})"));
        println!("frame {}{of}: {what}", d.frame);
    }
    println!(
        "{}: {} of {} frames differ; {}{}",
        case.name,
        found.len(),
        case.frames,
        counts(&o),
        tolerated(&o).map_or(String::new(), |t| format!("; {t}"))
    );
    Ok(found.is_empty())
}

fn all(mode: Mode) -> Result<bool, String> {
    let dir = cases_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    run_cases(&paths, mode, false)
}

fn run_cases(paths: &[PathBuf], mode: Mode, alone: bool) -> Result<bool, String> {
    let mut green = true;
    for p in paths {
        let mut case = Case {
            mode,
            ..Case::load(p)?
        };
        if alone {
            if case.alone == Alone::Off {
                return Err(format!(
                    "{}: --alone: a render-only case steps no engine",
                    case.name
                ));
            }
            case.alone = Alone::Only;
        }
        match sf_conform::run(&case) {
            Ok(o) => {
                print!("{}: ok, {} frames ({})", case.name, o.frames, counts(&o));
                if case.compare.render {
                    print!(", {} rendered", o.rendered);
                }
                if !o.straddled.is_empty() {
                    print!(
                        "; lag (the original mid-pass) at {} frame(s) {:?}, re-synced at {:?}",
                        o.straddled.len(),
                        o.straddled,
                        o.resynced
                    );
                }
                if !o.raced.is_empty() {
                    print!("; beam races excused at {} frame(s)", o.raced.len());
                }
                if let Some(t) = tolerated(&o) {
                    print!("; {t}");
                }
                println!();
                for (f, why) in &o.skipped {
                    println!("  frame {f} left out: {why}");
                }
            }
            Err(d) => {
                green = false;
                print!("{}: {d}", case.name);
            }
        }
    }
    Ok(green)
}

/// The frames compared, the longest run the original ended mid-pass (the limited one), the
/// longest run left out for lag, and the frames the original's pass was late (DESIGN §2.1).
fn counts(o: &sf_conform::Outcome) -> String {
    let ((mf, mn), (sf, sn)) = (o.longest_mid_pass, o.longest_skip);
    let lag = match sn {
        0 => format!("{} compared, none left out for lag", o.compared),
        _ => format!(
            "{} compared, longest mid-pass run {mn} from frame {mf}, longest left out {sn} from frame {sf}",
            o.compared
        ),
    };
    format!("{lag}, the original late at {} frame(s)", o.late)
}

/// What a faithful run tolerated (DESIGN §2.1: never silent): the picture frames by kind, the
/// sound events matched frames apart or cut off by the case's end, the frames the passes were a
/// frame apart. `None` for a strict run.
fn tolerated(o: &sf_conform::Outcome) -> Option<String> {
    (o.mode == Mode::Faithful).then(|| {
        let mut t = format!(
            "tolerated: picture {} one-step, {} few-pixel, {} short-glitch frame(s); sound {} shift(s)",
            o.picture_colour_step.len(),
            o.picture_few_pixels.len(),
            o.picture_glitches.len(),
            o.sound_shifts
        );
        if o.sound_cut_off > 0 {
            t += &format!(", {} event(s) cut off by the end", o.sound_cut_off);
        }
        if o.sound_trailing > 0 {
            t += &format!(", {} change(s) before a stop", o.sound_trailing);
        }
        if let Some(m) = &o.milestones {
            t += &format!("; alone: {} milestone(s) met", m.len());
            if std::env::var_os("SF_CONFORM_MILESTONES").is_some() {
                for (f, m) in m {
                    t += &format!("\n  {f}: {m}");
                }
            }
        }
        if !o.pass_skews.is_empty() {
            t += &format!(
                "; passes a frame apart at {} frame(s), {} state(s) compared so",
                o.pass_skews.len(),
                o.pass_held
            );
        }
        t
    })
}

/// `NAME=ADDR:W`, `ADDR:W`; ADDR is `h0+<hex>` or `0x<hex>`.
fn watch(spec: &str) -> Result<VarDef, String> {
    let bad = || format!("--watch {spec}: expected NAME=ADDR:W, ADDR h0+<hex> or 0x<hex>, W B|W|L");
    let (name, rest) = spec.split_once('=').unwrap_or(("", spec));
    let (addr, w) = rest.rsplit_once(':').ok_or_else(bad)?;
    let hex = |s: &str| {
        let s = s.trim_start_matches('$');
        s.bytes()
            .all(|b| b.is_ascii_hexdigit())
            .then(|| u32::from_str_radix(s, 16).ok())
            .flatten()
            .ok_or_else(bad)
    };
    let addr = match (addr.strip_prefix("h0+"), addr.strip_prefix("0x")) {
        (Some(off), _) => h0(hex(off)?),
        (_, Some(a)) => hex(a)?,
        _ => return Err(bad()),
    };
    let width = match w {
        "B" => Width::B,
        "W" => Width::W,
        "L" => Width::L,
        _ => return Err(bad()),
    };
    let name: &'static str = if name.is_empty() { spec } else { name }.to_string().leak();
    // Watched addresses are not modelled; the id is only a placeholder for printing.
    Ok(VarDef {
        id: VarId::RngState,
        name,
        addr,
        width,
    })
}

/// `LAYOUT` entries longer than this many bytes (bitmaps, the level map, the object slots) are left out
/// of `record` unless `--all-vars`: each is printed one long at a time on every changed frame, which
/// made lines of ~3.5 MB and a 38 GB recording that filled the disk and crashed the run
/// (2026-10-15, DESIGN §5.2).
const RECORD_BIG_VAR_BYTES: u32 = 0x400;

/// `record` stops once it has written this much (lines and PNGs) unless `--max-mb` says otherwise.
const RECORD_DEFAULT_MAX_MB: u64 = 200;

/// The `LAYOUT` entries `record` prints by default, and the names of those it leaves out.
fn record_layout(all_vars: bool) -> (Vec<VarDef>, Vec<&'static str>) {
    let mut left_out = Vec::new();
    let shown = LAYOUT
        .iter()
        .filter(|d| match d.width {
            Width::Bytes(n) if n > RECORD_BIG_VAR_BYTES && !all_vars => {
                left_out.push(d.name);
                false
            }
            _ => true,
        })
        .flat_map(|d| d.pieces())
        .collect();
    (shown, left_out)
}

/// What `record` has written, against its cap.
struct RecordBudget {
    written: u64,
    max: u64,
}

impl RecordBudget {
    fn new(max_mb: u64) -> RecordBudget {
        RecordBudget {
            written: 0,
            max: max_mb * 1_000_000,
        }
    }

    /// Counts `bytes`; `Err` once the total passes the cap, naming what to do.
    fn spend(&mut self, bytes: usize, frame: u32) -> Result<(), String> {
        self.written += bytes as u64;
        if self.written > self.max {
            return Err(format!(
                "record stopped at frame {frame}: output passed {} MB (--max-mb raises the cap; \
                 --only with --watch, or fewer frames, keeps it small)",
                self.max / 1_000_000
            ));
        }
        Ok(())
    }
}

fn record(path: &Path, rest: &[String]) -> Result<bool, String> {
    let case = Case::load(path)?;
    let mut every = false;
    let mut trace = false;
    let mut paula = false;
    let mut png_dir: Option<PathBuf> = None;
    let mut all_vars = false;
    let mut max_mb = RECORD_DEFAULT_MAX_MB;
    let mut watched: Vec<VarDef> = Vec::new();
    let mut only = false;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--every" => every = true,
            "--only" => only = true,
            "--trace" => trace = true,
            "--paula" => paula = true,
            "--all-vars" => all_vars = true,
            "--max-mb" => {
                max_mb = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|&n| n > 0)
                    .ok_or(USAGE)?
            }
            "--png" => png_dir = Some(PathBuf::from(it.next().ok_or(USAGE)?)),
            "--watch" => watched.push(watch(it.next().ok_or(USAGE)?)?),
            _ => return Err(USAGE.to_string()),
        }
    }
    let (mut shown, left_out) = if only {
        (Vec::new(), Vec::new())
    } else {
        record_layout(all_vars)
    };
    shown.extend(watched);
    let mut budget = RecordBudget::new(max_mb);
    if let Some(d) = &png_dir {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let synced = sync::reach_frozen(&case.sync, &case.script, case.frames, case.freeze())?;
    let mut amiga = synced.amiga;
    // As `run` does: every disk the original asks for goes in.
    let mut disks = Some(sync::Disks::at(&synced.memory));
    println!(
        "# {}: oracle from {:?}, frames counted from there",
        case.name, case.sync
    );
    if !left_out.is_empty() {
        println!(
            "# left out (over {RECORD_BIG_VAR_BYTES} bytes; --all-vars, or --watch one word): {}",
            left_out.join(" ")
        );
    }
    let mut prev: Option<Vec<u32>> = None;
    let frozen = case.freeze();
    let display = sf_conform::display_data();
    for f in 0..case.frames {
        let input = case.script.input_at(f);
        frozen.apply(&mut amiga, &display);
        let capture = amiga.step(input).map_err(|e| e.to_string())?;
        if let Some(d) = disks.as_mut() {
            d.serve(&mut amiga);
        }
        let mem = sync::core_memory(amiga.memory());
        let mut vals: Vec<u32> = shown.iter().map(|d| d.read(&mem as &dyn Ram)).collect();
        let at = if trace {
            let chain = sync::main_chain(&amiga.cpu(), &mem);
            vals.extend(&chain);
            let names: Vec<String> = chain
                .iter()
                .map(|&a| format!("h0+${:x}", a - h0(0)))
                .collect();
            format!(" | at {}", names.join(" < "))
        } else {
            String::new()
        };
        if every || prev.as_ref() != Some(&vals) {
            if let Some(d) = &png_dir {
                let p = &capture.picture;
                let rgb: Vec<u8> = p
                    .pixels
                    .iter()
                    .flat_map(|&c| {
                        [
                            (c >> 8 & 15) as u8 * 17,
                            (c >> 4 & 15) as u8 * 17,
                            (c & 15) as u8 * 17,
                        ]
                    })
                    .collect();
                let path = d.join(format!("frame{f}.png"));
                let bytes = sf_conform::report::png(p.width as u32, p.height as u32, &rgb);
                budget.spend(bytes.len(), f)?;
                std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            let cols: Vec<String> = shown
                .iter()
                .zip(&vals)
                .map(|(d, v)| format!("{}@{:x}={v:x}", d.name, d.addr))
                .collect();
            let line = format!(
                "{f}: {}  | {:?} | {}{at}",
                InputScript::format_input(input),
                sync::pacing(&amiga.cpu(), &mem, &capture.paula),
                cols.join(" ")
            );
            budget.spend(line.len() + 1, f)?;
            println!("{line}");
            if paula {
                let ws: Vec<String> = capture
                    .paula
                    .iter()
                    .map(|w| {
                        format!(
                            "{}={:04x}@{}",
                            sf_conform::compare::reg_name(w.reg),
                            w.value,
                            w.vpos
                        )
                    })
                    .collect();
                let line = format!("    paula: {}", ws.join(" "));
                budget.spend(line.len() + 1, f)?;
                println!("{line}");
            }
        }
        prev = Some(vals);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_leaves_out_the_big_runs_by_default() {
        let (shown, left_out) = record_layout(false);
        assert!(left_out.contains(&"playfield_buffer_0"));
        assert!(left_out.contains(&"bob_sheet"));
        assert!(
            !left_out.contains(&"frog_rest"),
            "the frog's record is small and wanted"
        );
        assert!(shown.iter().any(|d| d.name == "frog_rest"));
        assert!(shown.iter().all(|d| !left_out.contains(&d.name)));
        let (all, none) = record_layout(true);
        assert!(none.is_empty());
        assert!(all.len() > shown.len());
    }

    #[test]
    fn a_default_record_line_stays_small() {
        // ~30 characters per long printed; a line of the default set must stay far below the
        // 3.5 MB lines that filled the disk.
        let (shown, _) = record_layout(false);
        assert!(shown.len() * 30 < 200_000, "{} longs per line", shown.len());
    }

    #[test]
    fn the_budget_stops_past_its_cap_and_says_how_to_go_on() {
        let mut b = RecordBudget::new(1);
        assert!(b.spend(600_000, 0).is_ok());
        assert!(b.spend(400_000, 1).is_ok());
        let e = b.spend(1, 2).unwrap_err();
        assert!(e.contains("frame 2") && e.contains("--max-mb"), "{e}");
    }
}
