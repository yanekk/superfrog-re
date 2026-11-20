//! `fd-routeview`: the map page, one web page opened from disk.
//!
//! ```text
//! fd-routeview --all [--out DIR]                  # six levels, map only, live mode on
//! fd-routeview --level W2-1 [--out DIR]           # one level, map only, live mode on
//! fd-routeview --script FILE [--out DIR] [--all]  # level 1's map with a scripted run's path
//! ```
//!
//! `--all` and `--level` write `fd_map::default_out()` (`games/forest-dumb/build/map` of the main
//! checkout), the page the app's live mode writes beside. `--script` alone writes
//! `games/forest-dumb/build/route-viz` as before; with `--all` its route takes W1-1's place in the
//! six-level page. The script is an input script (`conform/scripts/*.txt`) played on the engine
//! from W1-1's start. The page holds the disks' graphics: keep it under `games/forest-dumb/build/`.

use std::path::PathBuf;

const USAGE: &str = "fd-routeview --all [--out DIR]\n\
                     fd-routeview --level ID [--out DIR]\n\
                     fd-routeview --script FILE [--out DIR] [--all]";

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2)
}

fn bad(e: &str) -> ! {
    eprintln!("fd-routeview: {e}");
    std::process::exit(2)
}

fn fail(e: &str) -> ! {
    eprintln!("fd-routeview: {e}");
    std::process::exit(1)
}

fn main() {
    let mut script: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut all = false;
    let mut level: Option<String> = None;
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| usage());
        match k.as_str() {
            "--script" => script = Some(PathBuf::from(val())),
            "--out" => out = Some(PathBuf::from(val())),
            "--level" => level = Some(val()),
            "--all" => all = true,
            _ => usage(),
        }
    }
    let worlds: Vec<u8> = match (&level, all) {
        (Some(_), true) => bad("--level and --all: pick one"),
        (Some(id), false) => vec![fd_routeview::world_of(id).unwrap_or_else(|e| bad(&e))],
        (None, true) => fd_map::WORLDS.to_vec(),
        (None, false) => vec![],
    };
    if worlds.is_empty() && script.is_none() {
        usage()
    }
    // Routes are replayed from W1-1's start only; plan forest-dumb-level-2 adds W2-1's.
    if let (Some(&w), Some(_), Some(id)) = (worlds.first(), &script, &level)
        && w != 1
    {
        bad(&format!(
            "--script replays from W1-1's start only; {id} has no route replay yet"
        ))
    }
    // The script is read before the disks, so a wrong path is named at once.
    if let Some(s) = &script
        && let Err(e) = std::fs::metadata(s)
    {
        bad(&format!("script {}: {e}", s.display()))
    }

    if worlds.is_empty() {
        // `--script` alone: W1-1's route page, where it always was.
        let out = out.unwrap_or_else(fd_routeview::default_out);
        let script = script.expect("checked above");
        let data = fd_conform::game_data().unwrap_or_else(|e| fail(&e));
        match fd_routeview::write_page(&data, &script, &out) {
            Ok((page, d)) => println!(
                "{}: {} frames, ends at {}, {} events",
                page.display(),
                d.track.len(),
                d.end_cell(),
                d.events.len()
            ),
            Err(e) => fail(&e),
        }
        return;
    }

    let out = out.unwrap_or_else(fd_map::default_out);
    let route = script.as_ref().map(|s| {
        let data = fd_conform::game_data().unwrap_or_else(|e| fail(&e));
        fd_routeview::play(&data, s).unwrap_or_else(|e| fail(&e))
    });

    let r = route.as_ref().map(|(dump, entry)| fd_routeview::Route {
        dump,
        entry: entry.clone(),
    });
    let dyn_route = r.as_ref().map(|r| r as &dyn amiga_routeview::LevelDump);
    match fd_routeview::write_map(&out, &worlds, dyn_route) {
        Ok(page) => {
            let n = worlds.len();
            print!(
                "{}: {n} level{}",
                page.display(),
                if n == 1 { "" } else { "s" }
            );
            match &route {
                Some((d, _)) => println!(
                    ", W1-1's route {} frames, ends at {}",
                    d.track.len(),
                    d.end_cell()
                ),
                None => println!(),
            }
        }
        Err(e) => fail(&e),
    }
}
