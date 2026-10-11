//! `sf-conform run <case.toml>` — one case; exit 1 on a divergence.
//! `sf-conform all` — every case in `engine/conform/cases/`.
//! `sf-conform record <case.toml> [--watch NAME=ADDR:W]... [--every]` — the original's state per
//! frame of a case, for writing scripts: every `LAYOUT` variable and each watched address
//! (`h0+9d8:W`, `0xc17074:L`, width B, W or L), printed on frames where something changed.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sf_conform::{Case, InputScript, cases_dir, sync};
use sf_core::layout::{LAYOUT, Ram, VarDef, VarId, Width, h0};

const USAGE: &str = "usage: sf-conform run <case.toml> | all | record <case.toml> [--watch NAME=ADDR:W]... [--every]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("run") if args.len() == 2 => run_cases(&[PathBuf::from(&args[1])]),
        Some("all") if args.len() == 1 => all(),
        Some("record") if args.len() >= 2 => record(Path::new(&args[1]), &args[2..]),
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

fn all() -> Result<bool, String> {
    let dir = cases_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    run_cases(&paths)
}

fn run_cases(paths: &[PathBuf]) -> Result<bool, String> {
    let mut green = true;
    for p in paths {
        let case = Case::load(p)?;
        match sf_conform::run(&case) {
            Ok(()) => println!("{}: ok, {} frames", case.name, case.frames),
            Err(d) => {
                green = false;
                print!("{}: {d}", case.name);
            }
        }
    }
    Ok(green)
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

fn record(path: &Path, rest: &[String]) -> Result<bool, String> {
    let case = Case::load(path)?;
    let mut every = false;
    let mut shown: Vec<VarDef> = LAYOUT.iter().flat_map(|d| d.pieces()).collect();
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--every" => every = true,
            "--watch" => shown.push(watch(it.next().ok_or(USAGE)?)?),
            _ => return Err(USAGE.to_string()),
        }
    }
    let synced = sync::reach(&case.sync, &case.script, case.frames)?;
    let mut amiga = synced.amiga;
    println!(
        "# {}: oracle from {:?}, frames counted from there",
        case.name, case.sync
    );
    let mut prev: Option<Vec<u32>> = None;
    for f in 0..case.frames {
        let input = case.script.input_at(f);
        amiga.step(input).map_err(|e| e.to_string())?;
        let mem = sync::core_memory(amiga.memory());
        let vals: Vec<u32> = shown.iter().map(|d| d.read(&mem as &dyn Ram)).collect();
        if every || prev.as_ref() != Some(&vals) {
            let cols: Vec<String> = shown
                .iter()
                .zip(&vals)
                .map(|(d, v)| format!("{}@{:x}={v:x}", d.name, d.addr))
                .collect();
            println!(
                "{f}: {}  | {:?} | {}",
                InputScript::format_input(input),
                sync::pacing(&amiga.cpu(), &mem),
                cols.join(" ")
            );
        }
        prev = Some(vals);
    }
    Ok(true)
}
