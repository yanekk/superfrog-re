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
//! `--translated` judges the translated tree built in from `FD_TRANSLATED` (translator-trial T04,
//! `judge::translated`) entered at the routine, instead of `FD_PORT`'s port; one build serves every
//! routine of the tree. The line gains the stops, one per failed state, by kind, and the first:
//!
//! `..,"first_failure":"..","self_modifying":n,"untranslated":n,"write_into_code":n,"unsupported":n,"stopped_hangs":n,"first_stop":".."}`
//!
//! `self_modifying` counts the states on which the original changed a byte of the tree's `CODE`
//! (translator DESIGN §2.5): not judged, and out of `passed` and `failures`. `outside` counts states
//! stopped on an access to memory the A500 lacks (only the machine makes one), `traps` those ended
//! by an exception.
//!
//! `--translated --machine` (translator T27) runs the tree on `pcode-machine` instead of the judge's
//! copy of the oracle's hardware mode (`judge::translated::machine`): blits, the beam and register
//! reads are the machine's models, the case's hardware reads answered by its devices. The line
//! gains `"hardware":"machine"`.
//!
//! `--cover` judges the generator, not a port (T61, `tools/generate.py`): the original alone,
//! traced ([`Rig::coverage`]), then each state's pokes and hardware reads against the bytes the
//! original read on some state, off the rig's stack:
//!
//! `{"routine":..,"gen":..,"cases":n,"passed":n,"hangs":n,"faults":n,"unmodelled_blits":n,"base":bool,"branches":[["hN+$off",taken,not_taken],..],"outside":n,"first_outside":".."}`
//!
//! `outside` counts the poked (or hardware-read) bytes the original never read, `first_outside`
//! names the first. A `base` is reported, not judged: the reviewer reads it.
//!
//! `--insn <encoding> --at 'hN+$off'` (translator T06, `judge::insn`) checks one instruction of the
//! module built in from `FD_INSN` (`pcode_rust.py --insn`) against Moira's step on `--states N`
//! generated states (500 unless set), comparing `--columns` (default `regs,ccr,mem,cycles`);
//! `--insn-all [--shard K/N]` checks every site of the module, or every Nth from the Kth. `--seed S`
//! changes the states. One line a site:
//!
//! `{"encoding":..,"at":..,"states":n,"pass":n,"fail":n,"skipped":n,"columns":{"regs":n,"ccr":n,"mem":n,"cycles":n|"not-counted","stop":n},"first":{"state","column","what","moira","translated"}|null,"first_skip":".."}`
//!
//! `--os-run power-on` (translator T26, `os_run`) runs the translated tree on `pcode-machine` from
//! the program's entry, the OS answered and disk 2 in df0, until the title's first fire poll:
//!
//! `{"run":"power-on","stop":"..","at_title":bool,"blocks":n,"cycles":n,"opened":["name",..],"missing":["name",..]}`
//!
//! `--os-run load-napis` runs translated `load_file` alone on the machine with `_NAPIS` asked for,
//! and the original to the frame its `load_name` moves on, and compares the 18 720 bytes at `h3+$0`
//! (`differ` counts the bytes that differ, `first_differ` is the first one's offset or -1):
//!
//! `{"run":"load-napis","stop":"..","equal":bool,"differ":n,"first_differ":n,"nonzero":n,"frame":n,"opened":[..],"missing":[..]}`
//!
//! `nonzero` counts the original's bytes there that are not 0, so an empty match shows.
//!
//! `--generators` (translator T16) prints every registered generator's name, in registry order, as
//! one line `{"generators":["name",..]}`: the whole-tree judge's work list, read from the build
//! rather than from `re/translate.toml`, which predates the later generator modules. Needs no disks.

use std::process::ExitCode;

use fd_conform::generators;
use fd_conform::judge::insn::{self, Columns, InsnRig, Site};
use fd_conform::judge::translated::{self, Hardware, Stops, tree};
use fd_conform::judge::{self, port};
use fd_conform::routine::{Coverage, Gen, Generator, Rig, RigReport, STACK_AREA, h, seed};

const USAGE: &str = "usage: fd-judge 'hN+$off' --gen <name> [--cases N] [--survey | --cover | --translated [--machine]]";

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
    translated: bool,
    machine: bool,
}

fn args() -> Result<Args, String> {
    let v: Vec<String> = std::env::args().skip(1).collect();
    let mut it = v.iter();
    let (mut routine, mut gen_name, mut cases, mut survey, mut cover, mut translated) =
        (None, None, 2000usize, false, false, false);
    let mut machine = false;
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
            "--translated" => translated = true,
            "--machine" => machine = true,
            r if routine.is_none() && !r.starts_with("--") => routine = Some(r.to_string()),
            other => return Err(format!("`{other}`: {USAGE}")),
        }
    }
    let routine = routine.ok_or(USAGE)?;
    if [survey, cover, translated].iter().filter(|&&m| m).count() > 1 {
        return Err(format!(
            "--survey, --cover and --translated are three modes: {USAGE}"
        ));
    }
    if machine && !translated {
        return Err(format!("--machine judges a translated tree: {USAGE}"));
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
        translated,
        machine,
    })
}

/// The names that opened (or did not), as a JSON list's items.
fn names(opened: &[(String, bool)], ok: bool) -> String {
    let n: Vec<String> = opened
        .iter()
        .filter(|(_, o)| *o == ok)
        .map(|(n, _)| format!("\"{}\"", esc(n)))
        .collect();
    n.join(",")
}

/// `--os-run <which>`: one line of JSON, or the reason it could not run.
fn os_run_main(which: &str) -> Result<(), String> {
    use fd_conform::os_run;
    let disks = fd_conform::disks::load()?;
    let program = os_run::program(&disks[0])?;
    match which {
        "power-on" => {
            let e = os_run::power_on_to_title(&program, disks[1].clone())?;
            let stop = match &e.result {
                Ok(()) => "returned".to_string(),
                Err(s) => s.to_string(),
            };
            println!(
                "{{\"run\":\"power-on\",\"stop\":\"{}\",\"at_title\":{},\"blocks\":{},\"cycles\":{},\"opened\":[{}],\"missing\":[{}]}}",
                esc(&stop),
                stop.starts_with(&format!("unsupported: {}", os_run::AT_TITLE)),
                e.blocks,
                e.cycles,
                names(&e.opened, true),
                names(&e.opened, false)
            );
            Ok(())
        }
        "load-napis" => {
            let l = os_run::load_napis(&program, disks)?;
            let stop = match &l.result {
                Ok(()) => "returned".to_string(),
                Err(s) => s.to_string(),
            };
            let differ: Vec<usize> = (0..l.machine.len())
                .filter(|&i| l.machine[i] != l.original[i])
                .collect();
            println!(
                "{{\"run\":\"load-napis\",\"stop\":\"{}\",\"equal\":{},\"differ\":{},\"first_differ\":{},\"nonzero\":{},\"frame\":{},\"opened\":[{}],\"missing\":[{}]}}",
                esc(&stop),
                l.result.is_ok() && differ.is_empty() && l.machine.len() == os_run::NAPIS_LEN,
                differ.len(),
                differ.first().map_or(-1, |&i| i as i64),
                l.original.iter().filter(|&&b| b != 0).count(),
                l.frame,
                names(&l.opened, true),
                names(&l.opened, false)
            );
            Ok(())
        }
        other => Err(format!("--os-run {other}: not power-on or load-napis")),
    }
}

fn main() -> ExitCode {
    let v: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = v.iter().position(|a| a == "--os-run") {
        let which = v.get(i + 1).map(String::as_str).unwrap_or("");
        return match os_run_main(which) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("fd-judge: {e}");
                ExitCode::from(2)
            }
        };
    }
    if v.len() == 1 && v[0] == "--generators" {
        let names: Vec<String> = generators::all()
            .map(|(n, _)| format!("\"{}\"", esc(n)))
            .collect();
        println!("{{\"generators\":[{}]}}", names.join(","));
        return ExitCode::SUCCESS;
    }
    if v.iter().any(|a| a == "--insn" || a == "--insn-all") {
        return match insn_main(&v) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("fd-judge: {e}");
                ExitCode::from(2)
            }
        };
    }
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
    let mut stops = None::<Stops>;
    let (report, failures, first): (RigReport, usize, String) = if a.survey {
        let r = rig.survey(&a.gen_name, a.cases, a.entry, &generator);
        (r, 0, String::new())
    } else if a.translated {
        // The tree names entries `hN+$off` in lower-case hex, whatever the argument's case.
        let key = rig.name(a.entry);
        let hw = if a.machine {
            Hardware::Machine
        } else {
            Hardware::Mem
        };
        let j = translated::check_on(
            &mut rig,
            &a.gen_name,
            a.cases,
            a.entry,
            &generator,
            tree::entry(&key),
            tree::CODE,
            hw,
        );
        stops = Some(j.stops);
        match j.verdict {
            Ok(r) => (r, 0, String::new()),
            Err(f) => (f.report, f.failures, f.first),
        }
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
    let stops = stops.map_or(String::new(), |s| {
        format!(
            ",\"self_modifying\":{},\"untranslated\":{},\"write_into_code\":{},\"unsupported\":{},\"stopped_hangs\":{},\"outside\":{},\"traps\":{}{},\"first_stop\":\"{}\"",
            report.self_modifying,
            s.untranslated,
            s.write_into_code,
            s.unsupported,
            s.hangs,
            s.outside,
            s.traps,
            if a.machine {
                ",\"hardware\":\"machine\""
            } else {
                ""
            },
            esc(&s.first)
        )
    });
    println!(
        "{{\"routine\":\"{}\",\"gen\":\"{}\",\"cases\":{},\"passed\":{},\"paths\":{},\"hangs\":{},\"faults\":{},\"unmodelled_blits\":{},\"failures\":{failures},\"pass\":{pass},\"first_failure\":\"{}\"{stops}}}",
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

const INSN_USAGE: &str = "usage: fd-judge --insn <encoding> --at 'hN+$off' | --insn-all [--shard K/N] \
                          [--states N] [--columns regs,ccr,mem,cycles] [--seed S]";

/// `--insn` and `--insn-all`: the instruction check over the module `FD_INSN` built in.
fn insn_main(v: &[String]) -> Result<(), String> {
    let (mut enc, mut at, mut all, mut states, mut cols, mut seed, mut shard) = (
        None,
        None,
        false,
        500usize,
        Columns::ALL,
        String::new(),
        (0usize, 1usize),
    );
    let mut it = v.iter();
    while let Some(a) = it.next() {
        let mut val = || {
            it.next()
                .cloned()
                .ok_or(format!("`{a}` takes a value: {INSN_USAGE}"))
        };
        match a.as_str() {
            "--insn" => enc = Some(val()?.to_lowercase()),
            "--at" => at = Some(val()?),
            "--insn-all" => all = true,
            "--states" => {
                states = val()?
                    .parse()
                    .ok()
                    .filter(|&n| n > 0)
                    .ok_or("--states takes a whole number above 0")?
            }
            "--columns" => cols = Columns::parse(&val()?)?,
            "--seed" => seed = val()?,
            "--shard" => {
                let s = val()?;
                shard = s
                    .split_once('/')
                    .and_then(|(k, n)| Some((k.parse().ok()?, n.parse().ok()?)))
                    .filter(|&(k, n): &(usize, usize)| n > 0 && k < n)
                    .ok_or(format!("--shard `{s}` is not K/N with K below N"))?;
            }
            other => return Err(format!("`{other}`: {INSN_USAGE}")),
        }
    }
    if all == enc.is_some() || enc.is_some() != at.is_some() {
        return Err(INSN_USAGE.into());
    }
    let rows: Vec<Site> = match (&enc, &at) {
        (Some(e), Some(at)) => {
            let key = parse_routine(at)
                .ok_or_else(|| format!("`{at}` is not `hN+$off` with N a hunk 0-4"))?;
            let row = insn::tree::SITES
                .iter()
                .find(|r| (r.2 as usize, r.3) == key && r.0 == e)
                .ok_or_else(|| {
                    format!(
                        "{e} at {at} is not in the --insn module built in (FD_INSN names \
                         build/translator/insn.rs, written by pcode_rust.py --insn)"
                    )
                })?;
            vec![Site::of(row)]
        }
        _ => insn::tree::SITES
            .iter()
            .enumerate()
            .filter(|(i, _)| i % shard.1 == shard.0)
            .map(|(_, r)| Site::of(r))
            .collect(),
    };
    let dir = fd_tools::disk_dir()?;
    let exe = fd_tools::game_file(&dir, fd_conform::routine::PROGRAM)?;
    let (mem, spans) = fd_conform::routine::load_program(&exe)?;
    let sizes: Vec<u32> = spans.iter().map(|&(_, n)| n).collect();
    if !insn::tree::HUNK_SIZES.is_empty() && insn::tree::HUNK_SIZES != sizes.as_slice() {
        return Err(format!(
            "the --insn module's hunk sizes {:x?} are not the program's {sizes:x?}: an export of another program",
            insn::tree::HUNK_SIZES
        ));
    }
    for s in &rows {
        let a = (fd_conform::HUNK_BASES[s.hunk as usize] + s.off) as usize;
        let word = format!("{:02x}{:02x}", mem[a], mem[a + 1]);
        if word != s.encoding {
            return Err(format!(
                "{}: the program holds {word} there, not {}: the export is of another program",
                s.at, s.encoding
            ));
        }
    }
    let mut rig = InsnRig::new(mem, &fd_conform::HUNK_BASES, &sizes);
    std::panic::set_hook(Box::new(|_| {}));
    for s in &rows {
        let r = rig.check(s, states, &seed, cols);
        println!("{}", r.json(s, cols));
    }
    Ok(())
}
