//! `sf-bench`: the smart-planner benchmark (plans/smart-planner DESIGN §4.2).
//!
//! ```text
//! sf-bench --set A|B|C --planner new|plan_exit|plan_exit_w6|explore|best-old
//!          [--levels W1-1,W1-4] [--jobs N] [--threads N] [--budget-min 30] [--seed 1] [--follow-standalone] [--unguided]
//!          [--results plans/smart-planner/bench/results.jsonl] [--table plans/smart-planner/bench/RESULTS.md] [--no-view]
//! sf-bench --table-only [--results …] [--table …]     # re-render RESULTS.md from results.jsonl
//! ```
//!
//! Run from `engine/`: `cargo run --release -p sf-bench -- …`. Each level's line is appended as it
//! ends; a rerun skips the levels recorded for the same planner, set and commit, guided or not.
//! The new planner is guided by each level's committed route (DESIGN §4.2, D21); `--unguided`
//! runs it with no guide, recorded beside the guided result. `--unguided` is not used any more
//! (D28, owner): it stays, and warns when given. Routes and each
//! planner's stderr go to `routes/` beside the results file. A guided set-C run of the new planner
//! ends by rebuilding the route page, `build/route-viz/index.html` (`sf-routeview --planner`;
//! `--no-view` skips it).

use std::path::PathBuf;
use std::time::Duration;

use sf_bench::run::{Options, bench};

fn usage() -> ! {
    eprintln!(
        "sf-bench --set A|B|C --planner new|plan_exit|plan_exit_w6|explore|best-old [--levels W1-1,W1-4] \
         [--jobs N] [--threads N] [--budget-min 30] [--seed 1] [--follow-standalone] [--unguided (not used, D28)] [--results FILE] [--table FILE] [--no-view]\n\
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
        threads: None,
        budget: Duration::from_secs(30 * sf_bench::MINUTE),
        seed: 1,
        follow_standalone: false,
        unguided: false,
        results: bench_dir.join("results.jsonl"),
        table: bench_dir.join("RESULTS.md"),
    };
    let mut table_only = false;
    let mut view = true;
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| usage());
        match k.as_str() {
            "--set" => set = Some(val().parse().unwrap_or_else(|e| panic!("{e}"))),
            "--planner" => planner = Some(val()),
            "--levels" => o.levels = Some(val().split(',').map(str::to_string).collect()),
            "--jobs" => o.jobs = val().parse().unwrap_or_else(|_| usage()),
            "--threads" => {
                o.threads = Some(val().parse().ok().filter(|&n: &usize| n > 0).unwrap_or_else(|| usage()))
            }
            "--budget-min" => {
                o.budget = Duration::from_secs(val().parse::<u64>().unwrap_or_else(|_| usage()) * sf_bench::MINUTE)
            }
            "--seed" => o.seed = val().parse().unwrap_or_else(|_| usage()),
            "--follow-standalone" => o.follow_standalone = true,
            "--unguided" => o.unguided = true,
            "--results" => o.results = PathBuf::from(val()),
            "--table" => o.table = PathBuf::from(val()),
            "--table-only" => table_only = true,
            "--no-view" => view = false,
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
    if o.unguided {
        eprintln!(
            "sf-bench: warning: --unguided is not used any more (DESIGN D28): its routes overwrite the guided run's \
             routes/new-C-<level>.txt, which the route page draws"
        );
    }
    o.set = set;
    o.planner = planner;
    if let Err(e) = bench(&o) {
        eprintln!("sf-bench: {e}");
        std::process::exit(1);
    }
    if view && sf_bench::view::wanted(&o) {
        match sf_bench::view::run(&o.results) {
            Ok(page) => eprintln!("sf-bench: routes drawn: open {}", page.display()),
            Err(e) => eprintln!("sf-bench: no route page ({e}); the results stand"),
        }
    }
}
