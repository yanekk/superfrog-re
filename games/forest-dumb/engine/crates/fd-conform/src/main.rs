//! `fd-conform run <case.toml>`: one case, prints the verdict and its report; exit 0 only when the
//! case passes. Format and how to run it: `engine/conform/README.md`.
//!
//! `fd-conform bug replay <zip> [--out DIR]`: a bug zip replayed headlessly (DESIGN §2.8,
//! `fd_conform::bug`): `frames N`, `stopped-at`, `replay-at` and `screenshot equal` or
//! `screenshot differs (N pixels)`, exit 0 either way; the last frame's picture as `last.png` in
//! `--out` when given. A zip another build made, or that cannot be read, is refused naming why,
//! exit 1 (DESIGN §2.12).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fd_conform::{Case, run};

const USAGE: &str = "usage: fd-conform run <case.toml> | bug replay <zip> [--out DIR]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match &args[..] {
        [cmd, path] if cmd == "run" => run_case(Path::new(path)),
        [bug, replay, zip, rest @ ..] if bug == "bug" && replay == "replay" => {
            bug_replay(Path::new(zip), rest)
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
