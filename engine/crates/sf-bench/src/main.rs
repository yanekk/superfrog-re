//! `sf-bench`: the smart-planner benchmark (plans/smart-planner DESIGN §4.2).
//!
//! ```text
//! sf-bench --set A|B|C --planner new|plan_exit|plan_exit_w6|explore|best-old
//!          [--levels W1-1,W1-4] [--jobs N] [--budget-min 30] [--seed 1] [--follow-standalone]
//!          [--results plans/smart-planner/bench/results.jsonl] [--table plans/smart-planner/bench/RESULTS.md]
//! sf-bench --table-only [--results …] [--table …]     # re-render RESULTS.md from results.jsonl
//! ```
//!
//! Run from `engine/`: `cargo run --release -p sf-bench -- …`. Each level's line is appended as it
//! ends; a rerun skips the levels recorded for the same planner, set and commit. Routes and each
//! planner's stderr go to `routes/` beside the results file.

use std::path::PathBuf;
use std::time::Duration;

use sf_bench::run::{Options, bench};

fn usage() -> ! {
    eprintln!(
        "sf-bench --set A|B|C --planner new|plan_exit|plan_exit_w6|explore|best-old [--levels W1-1,W1-4] \
         [--jobs N] [--budget-min 30] [--seed 1] [--follow-standalone] [--results FILE] [--table FILE]\n\
         sf-bench --table-only [--results FILE] [--table FILE]"
    );
    std::process::exit(2)
}

fn main() {
    let bench_dir = sf_bench::repo_dir().join("plans/smart-planner/bench");
    let mut set = None;
    let mut planner = None;
    let mut o = Options {
        set: sf_bench::sets::Set::A,
        planner: String::new(),
        levels: None,
        jobs: 1,
        budget: Duration::from_secs(30 * sf_bench::MINUTE),
        seed: 1,
        follow_standalone: false,
        results: bench_dir.join("results.jsonl"),
        table: bench_dir.join("RESULTS.md"),
    };
    let mut table_only = false;
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| usage());
        match k.as_str() {
            "--set" => set = Some(val().parse().unwrap_or_else(|e| panic!("{e}"))),
            "--planner" => planner = Some(val()),
            "--levels" => o.levels = Some(val().split(',').map(str::to_string).collect()),
            "--jobs" => o.jobs = val().parse().unwrap_or_else(|_| usage()),
            "--budget-min" => {
                o.budget = Duration::from_secs(val().parse::<u64>().unwrap_or_else(|_| usage()) * sf_bench::MINUTE)
            }
            "--seed" => o.seed = val().parse().unwrap_or_else(|_| usage()),
            "--follow-standalone" => o.follow_standalone = true,
            "--results" => o.results = PathBuf::from(val()),
            "--table" => o.table = PathBuf::from(val()),
            "--table-only" => table_only = true,
            _ => usage(),
        }
    }
    if table_only {
        let all = sf_bench::results::load(&o.results);
        std::fs::write(&o.table, sf_bench::table::render(&all))
            .unwrap_or_else(|e| panic!("{}: {e}", o.table.display()));
        eprintln!("sf-bench: wrote {} from {} results", o.table.display(), all.len());
        return;
    }
    let (Some(set), Some(planner)) = (set, planner) else { usage() };
    o.set = set;
    o.planner = planner;
    if let Err(e) = bench(&o) {
        eprintln!("sf-bench: {e}");
        std::process::exit(1);
    }
}
