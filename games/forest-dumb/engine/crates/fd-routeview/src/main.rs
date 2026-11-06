//! `fd-routeview`: level 1's map with the character's path from a scripted run, one web page.
//!
//! ```text
//! fd-routeview --script FILE [--out DIR]
//! ```
//!
//! The script is an input script (`conform/scripts/*.txt`, `0: -`, `10: R U`, …) played on the
//! engine from W1-1's start. Run from `games/forest-dumb/engine/`:
//! `cargo run --release -p fd-routeview -- --script FILE`, then open
//! `games/forest-dumb/build/route-viz/index.html`. The page holds the disks' graphics: keep it under
//! `games/forest-dumb/build/`.

use std::path::PathBuf;

fn usage() -> ! {
    eprintln!("fd-routeview --script FILE [--out DIR]");
    std::process::exit(2)
}

fn main() {
    let mut script: Option<PathBuf> = None;
    let mut out = fd_routeview::default_out();
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| usage());
        match k.as_str() {
            "--script" => script = Some(PathBuf::from(val())),
            "--out" => out = PathBuf::from(val()),
            _ => usage(),
        }
    }
    let Some(script) = script else { usage() };
    // The script is read before the disks, so a wrong path is named at once.
    if let Err(e) = std::fs::metadata(&script) {
        eprintln!("fd-routeview: script {}: {e}", script.display());
        std::process::exit(2)
    }
    let data = fd_conform::game_data().unwrap_or_else(|e| {
        eprintln!("fd-routeview: {e}");
        std::process::exit(1)
    });
    match fd_routeview::write_page(&data, &script, &out) {
        Ok((page, d)) => println!(
            "{}: {} frames, ends at {}, {} events",
            page.display(),
            d.track.len(),
            d.end_cell(),
            d.events.len()
        ),
        Err(e) => {
            eprintln!("fd-routeview: {e}");
            std::process::exit(1)
        }
    }
}
