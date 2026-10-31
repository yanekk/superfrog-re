//! `fd-conform run <case.toml>`: one case, prints the verdict; exit 0 only for the same on every
//! pass. Format and how to run it: `engine/conform/README.md`.

use std::path::Path;
use std::process::ExitCode;

use fd_conform::{Case, Verdict, run};

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
        Ok(Verdict::Same { .. }) => ExitCode::SUCCESS,
        Ok(Verdict::Differs { .. }) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("fd-conform: {e}");
            ExitCode::FAILURE
        }
    }
}
