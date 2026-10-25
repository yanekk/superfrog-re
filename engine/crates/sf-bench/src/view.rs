//! After a guided set-C run of the new planner, the route page (`sf-routeview --planner`) is
//! rebuilt from the results and routes it just wrote, so each run ends with a picture of every
//! route and its verdict. The viewer runs as a child, like the planners: sf-bench never links it
//! (sf-routeview reads `results.jsonl` through this crate, so linking it would be a cycle). A
//! viewer that fails is reported and never fails the benchmark.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::run::Options;
use crate::sets::Set;

const TARGET: (&str, &str, &str) = ("sf-routeview", "bin", "sf-routeview");

/// Whether a run under `o` gets the page: the viewer draws the new planner's guided set-C routes
/// (an unguided run writes the same `routes/new-C-<level>.txt` files, so it is left alone).
pub fn wanted(o: &Options) -> bool {
    o.set == Set::C && o.planner == "new" && !o.unguided
}

/// The viewer's command for the results file `results` (its routes are in `routes/` beside it).
pub fn command(results: &Path) -> Command {
    let mut c = Command::new(crate::adapters::binary(TARGET));
    c.arg("--planner").arg("--results").arg(results).current_dir(crate::engine_dir());
    c
}

/// Builds and runs the viewer; the page's path, or why there is none.
pub fn run(results: &Path) -> Result<PathBuf, String> {
    crate::adapters::build(&[TARGET])?;
    let out = command(results).output().map_err(|e| format!("sf-routeview: {e}"))?;
    let said = String::from_utf8_lossy(&out.stdout);
    let line = said.lines().last().unwrap_or("").to_string();
    if !out.status.success() {
        return Err(format!("sf-routeview failed ({}): {}", out.status, String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(PathBuf::from(line.split(": ").next().unwrap_or(&line)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn opts(set: Set, planner: &str, unguided: bool) -> Options {
        Options {
            set,
            planner: planner.into(),
            levels: None,
            jobs: 1,
            threads: None,
            budget: Duration::from_secs(60),
            seed: 1,
            follow_standalone: false,
            unguided,
            results: PathBuf::from("r.jsonl"),
            table: PathBuf::from("t.md"),
        }
    }

    #[test]
    fn only_a_guided_set_c_run_of_the_new_planner_draws_the_page() {
        assert!(wanted(&opts(Set::C, "new", false)));
        assert!(!wanted(&opts(Set::C, "new", true)));
        assert!(!wanted(&opts(Set::A, "new", false)));
        assert!(!wanted(&opts(Set::C, "plan_exit", false)));
    }

    #[test]
    fn the_viewer_reads_the_runs_results_file() {
        let c = command(Path::new("/x/results.jsonl"));
        let args: Vec<String> = c.get_args().map(|a| a.to_string_lossy().into()).collect();
        assert_eq!(args, ["--planner", "--results", "/x/results.jsonl"]);
        assert!(c.get_program().to_string_lossy().ends_with("release/sf-routeview"));
    }
}
