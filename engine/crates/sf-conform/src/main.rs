//! `sf-conform run <case.toml>` — one case; exit 1 on a divergence.
//! `sf-conform all` — every case in `engine/conform/cases/`.
//! `sf-conform record <case.toml> [--watch NAME=ADDR:W]... [--every] [--only] [--trace] [--png DIR]`
//! — the original's state per frame of a case, for writing scripts and porting: every `LAYOUT`
//! variable (none with `--only`) and each watched address (`h0+9d8:W`, `0xc17074:L`, width B, W
//! or L), printed on frames where something changed. `--trace` adds where the main program was at
//! the frame's end (its PC and the return addresses on its stack, as `h0+$x`), `--png` writes the
//! original's picture of every printed frame to `DIR/frame<N>.png`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sf_conform::{Case, InputScript, cases_dir, sync};
use sf_core::layout::{LAYOUT, Ram, VarDef, VarId, Width, h0};

const USAGE: &str = "usage: sf-conform run <case.toml> | all | record <case.toml> [--watch NAME=ADDR:W]... [--every] [--only] [--trace] [--png DIR]";

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
            Ok(o) => {
                print!("{}: ok, {} frames", case.name, o.frames);
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
    let mut trace = false;
    let mut png_dir: Option<PathBuf> = None;
    let mut shown: Vec<VarDef> = LAYOUT.iter().flat_map(|d| d.pieces()).collect();
    let layout_pieces = shown.len();
    let mut only = false;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--every" => every = true,
            "--only" => only = true,
            "--trace" => trace = true,
            "--png" => png_dir = Some(PathBuf::from(it.next().ok_or(USAGE)?)),
            "--watch" => shown.push(watch(it.next().ok_or(USAGE)?)?),
            _ => return Err(USAGE.to_string()),
        }
    }
    if only {
        shown.drain(..layout_pieces);
    }
    if let Some(d) = &png_dir {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let synced = sync::reach_frozen(
        &case.sync,
        &case.script,
        case.frames,
        case.excludes(sf_conform::Excluded::Objects),
    )?;
    let mut amiga = synced.amiga;
    // As `run` does: from the end sequence on, every disk the original asks for goes in.
    let mut disks =
        (case.sync == sync::SyncPoint::EndSequence).then(|| sync::Disks::at(&synced.memory));
    println!(
        "# {}: oracle from {:?}, frames counted from there",
        case.name, case.sync
    );
    let mut prev: Option<Vec<u32>> = None;
    let frozen = case.excludes(sf_conform::Excluded::Objects);
    let display = sf_conform::display_data();
    for f in 0..case.frames {
        let input = case.script.input_at(f);
        if frozen {
            sf_conform::freeze::freeze_objects(&mut amiga, &display);
        }
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
                std::fs::write(
                    &path,
                    sf_conform::report::png(p.width as u32, p.height as u32, &rgb),
                )
                .map_err(|e| format!("{}: {e}", path.display()))?;
            }
            let cols: Vec<String> = shown
                .iter()
                .zip(&vals)
                .map(|(d, v)| format!("{}@{:x}={v:x}", d.name, d.addr))
                .collect();
            println!(
                "{f}: {}  | {:?} | {}{at}",
                InputScript::format_input(input),
                sync::pacing(&amiga.cpu(), &mem),
                cols.join(" ")
            );
        }
        prev = Some(vals);
    }
    Ok(true)
}
