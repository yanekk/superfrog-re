//! `fd-judge 'hN+$off' --gen <name> [--cases N] [--survey]` (plans/forest-dumb-level-1 T33): runs the
//! original routine at `hN+$off` and the port built in (`FD_PORT`, see `build.rs`) from `N` states
//! (2 000 unless set) of the generator registered as `name`, and prints one JSON line:
//!
//! `{"routine":..,"gen":..,"cases":n,"passed":n,"paths":n,"hangs":n,"faults":n,"unmodelled_blits":n,"failures":n,"pass":bool,"first_failure":".."}`
//!
//! `--survey` runs the original alone: `passed` is then the states it returned from, the line
//! `tools/translate.py` reads to answer `no-oracle` before any model call. Exit 0 when the line was
//! printed, whatever the verdict; 2 on bad arguments or missing disks, with the reason on stderr.
//!
//! `--cover` judges the generator, not a port (T61, `tools/generate.py`): the original alone,
//! traced ([`Rig::coverage`]), then each state's pokes and hardware reads against the bytes the
//! original read on some state, off the rig's stack:
//!
//! `{"routine":..,"gen":..,"cases":n,"passed":n,"hangs":n,"faults":n,"unmodelled_blits":n,"base":bool,"branches":[["hN+$off",taken,not_taken],..],"outside":n,"first_outside":".."}`
//!
//! `outside` counts the poked (or hardware-read) bytes the original never read, `first_outside`
//! names the first. A `base` is reported, not judged: the reviewer reads it.

use std::process::ExitCode;

use fd_conform::generators;
use fd_conform::judge::{self, port};
use fd_conform::routine::{Coverage, Gen, Generator, Rig, RigReport, STACK_AREA, h, seed};

const USAGE: &str = "usage: fd-judge 'hN+$off' --gen <name> [--cases N] [--survey | --cover]";

/// `hN+$off` as (hunk, offset).
fn parse_routine(r: &str) -> Option<(usize, u32)> {
    let rest = r.strip_prefix('h')?;
    let (hunk, off) = rest.split_once("+$")?;
    let hunk: usize = hunk.parse().ok()?;
    (hunk < fd_conform::HUNK_BASES.len()).then_some(())?;
    Some((hunk, u32::from_str_radix(off, 16).ok()?))
}

fn esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            '\n' => o += "\\n",
            c if (c as u32) < 0x20 => {}
            c => o.push(c),
        }
    }
    o
}

struct Args {
    routine: String,
    entry: u32,
    gen_name: String,
    cases: usize,
    survey: bool,
    cover: bool,
}

fn args() -> Result<Args, String> {
    let v: Vec<String> = std::env::args().skip(1).collect();
    let mut it = v.iter();
    let (mut routine, mut gen_name, mut cases, mut survey, mut cover) =
        (None, None, 2000usize, false, false);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--gen" => gen_name = it.next().cloned(),
            "--cases" => {
                cases = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|&n| n > 0)
                    .ok_or("--cases takes a whole number above 0")?
            }
            "--survey" => survey = true,
            "--cover" => cover = true,
            r if routine.is_none() && !r.starts_with("--") => routine = Some(r.to_string()),
            other => return Err(format!("`{other}`: {USAGE}")),
        }
    }
    let routine = routine.ok_or(USAGE)?;
    if survey && cover {
        return Err(format!("--survey and --cover are two modes: {USAGE}"));
    }
    let (hunk, off) = parse_routine(&routine)
        .ok_or_else(|| format!("`{routine}` is not `hN+$off` with N a hunk 0-4"))?;
    Ok(Args {
        entry: h(hunk, off),
        routine,
        gen_name: gen_name.ok_or(USAGE)?,
        cases,
        survey,
        cover,
    })
}

fn main() -> ExitCode {
    let a = match args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("fd-judge: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(generator) = generators::generator(&a.gen_name) else {
        eprintln!(
            "fd-judge: no generator `{}` in fd-conform/src/gen/",
            a.gen_name
        );
        return ExitCode::from(2);
    };
    let mut rig = match Rig::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("fd-judge: {e}");
            return ExitCode::from(2);
        }
    };
    // A port's panic (an overflow, say) is caught and reported per state by the rig; the default
    // hook would print it for every one of 2 000 states.
    std::panic::set_hook(Box::new(|_| {}));
    if a.cover {
        let c = rig.coverage(&a.gen_name, a.cases, a.entry, &generator);
        println!("{}", cover_line(&a, &rig, &generator, &c));
        return ExitCode::SUCCESS;
    }
    // Seeded by the generator's name, as an engine port's rig test seeds it, so both see the same states.
    let (report, failures, first): (RigReport, usize, String) = if a.survey {
        let r = rig.survey(&a.gen_name, a.cases, a.entry, &generator);
        (r, 0, String::new())
    } else {
        match judge::check(
            &mut rig,
            &a.gen_name,
            a.cases,
            a.entry,
            &generator,
            port::port,
        ) {
            Ok(r) => (r, 0, String::new()),
            Err(f) => (f.report, f.failures, f.first),
        }
    };
    let pass = !a.survey && failures == 0 && report.passed > 0;
    println!(
        "{{\"routine\":\"{}\",\"gen\":\"{}\",\"cases\":{},\"passed\":{},\"paths\":{},\"hangs\":{},\"faults\":{},\"unmodelled_blits\":{},\"failures\":{failures},\"pass\":{pass},\"first_failure\":\"{}\"}}",
        esc(&a.routine),
        esc(&a.gen_name),
        a.cases,
        report.passed,
        report.paths,
        report.hangs,
        report.faults,
        report.unmodelled_blits,
        esc(&first)
    );
    ExitCode::SUCCESS
}

/// The `--cover` line: the coverage, and the states' pokes against what the original read. The
/// states are the coverage run's own, made again from the same seed.
fn cover_line(a: &Args, rig: &Rig, generator: &Generator, c: &Coverage) -> String {
    let r = &c.report;
    let mut g = Gen::new(seed(&a.gen_name));
    let (mut outside, mut first) = (0usize, String::new());
    let read = |x: u32| STACK_AREA.contains(&x) || c.reads.contains(&x);
    for case in 0..a.cases {
        let s = (generator.case)(&mut g);
        let poked = s
            .pokes
            .iter()
            .flat_map(|(at, b)| (0..b.len() as u32).map(move |i| (at + i, "pokes")));
        let hw = s
            .hw_reads
            .iter()
            .map(|&(at, _)| (at, "sets a hardware read of"));
        for (x, what) in poked.chain(hw) {
            if !read(x) {
                outside += 1;
                if first.is_empty() {
                    first = format!(
                        "state {case} {what} ${x:06x} ({}), which the original reads on no state",
                        rig.name(x)
                    );
                }
            }
        }
    }
    let branches: Vec<String> = c
        .branches
        .iter()
        .map(|b| format!("[\"{}\",{},{}]", esc(&rig.name(b.at)), b.taken, b.not_taken))
        .collect();
    format!(
        "{{\"routine\":\"{}\",\"gen\":\"{}\",\"cases\":{},\"passed\":{},\"hangs\":{},\"faults\":{},\"unmodelled_blits\":{},\"base\":{},\"branches\":[{}],\"outside\":{outside},\"first_outside\":\"{}\"}}",
        esc(&a.routine),
        esc(&a.gen_name),
        a.cases,
        r.passed,
        r.hangs,
        r.faults,
        r.unmodelled_blits,
        generator.base.is_some(),
        branches.join(","),
        esc(&first)
    )
}
