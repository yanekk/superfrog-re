//! `fd-conform run <case.toml>`: one case, prints the verdict and its report; exit 0 only when the
//! case passes. Format and how to run it: `engine/conform/README.md`.

use std::path::Path;
use std::process::ExitCode;

use fd_conform::{Case, run};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [cmd, path] = &args[..] else {
        eprintln!("usage: fd-conform run <case.toml>");
        return ExitCode::from(2);
    };
    if cmd != "run" {
        eprintln!("usage: fd-conform run <case.toml>");
        return ExitCode::from(2);
    }
    let outcome = Case::load(Path::new(path)).and_then(|case| {
        let verdict = run(&case)?;
        println!("{}: {verdict}", case.name);
        Ok(verdict)
    });
    match outcome {
        Ok(v) if v.passed() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("fd-conform: {e}");
            ExitCode::FAILURE
        }
    }
}
