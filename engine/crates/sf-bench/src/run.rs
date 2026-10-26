//! A benchmark run (DESIGN §4.2): each level of the set planned by a child under the benchmark's
//! rules (the new planner guided by the committed route unless `--unguided`), verified, and its line appended before the next level's result is waited on.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use sf_conform::InputScript;

use crate::adapters::{Adapter, NewPlanner};
use crate::child::{self, ChildRun};
use crate::results::{self, LevelResult};
use crate::sets::{self, Set, Start};
use crate::verify::{self, Verdict};

/// A planner child past its budget by this much is killed (DESIGN §5.2).
pub const GRACE: Duration = Duration::from_secs(5 * crate::MINUTE);

pub struct Options {
    pub set: Set,
    /// `new`, the one planner benched since the old tools were retired (T34).
    pub planner: String,
    pub levels: Option<Vec<String>>,
    pub jobs: usize,
    /// `--threads`: each new-planner child's threads; `None` is its share of the cores,
    /// `adapters::threads_per_job(cores, jobs)`.
    pub threads: Option<usize>,
    pub budget: Duration,
    pub seed: u32,
    pub follow_standalone: bool,
    /// The new planner without its guide (DESIGN §4.2: the tuning run's unaided pass).
    pub unguided: bool,
    pub results: PathBuf,
    pub table: PathBuf,
}

/// What a planner's run comes to before and after its route is verified: a child killed for its
/// timeout is `timeout`; one whose `PLAN_STATS` says it used hints is `hinted` (DESIGN §2.10: the
/// benchmark passes none, so one used is a result that does not count); one that ran past its
/// budget is `over-budget`; otherwise `verify` decides.
pub fn judge(
    run: &ChildRun,
    stats: Option<&BTreeMap<String, String>>,
    budget: Duration,
    verify: impl FnOnce() -> Verdict,
) -> Verdict {
    if run.timed_out {
        return Verdict::failed("timeout");
    }
    if stats.and_then(|s| s.get("hints")).is_some_and(|h| h.trim() != "0") {
        return Verdict::failed("hinted");
    }
    if run.wall > budget {
        return Verdict::failed("over-budget");
    }
    let mut v = verify();
    if !v.completed
        && let Some(s) = run.status.filter(|s| !s.success())
    {
        v.reason = format!("{}; planner {s}", v.reason);
    }
    v
}

/// The commit the run measures.
pub fn commit() -> String {
    Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(crate::repo_dir())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Where a planner's route and its stderr go: beside the results file.
fn route_path(o: &Options, planner: &str, s: &Start) -> PathBuf {
    let dir = o.results.parent().unwrap_or(Path::new(".")).join("routes");
    dir.join(format!("{planner}-{}-{}.txt", o.set, s.level))
}

/// One planner on one level: run, judged, recorded.
pub fn run_one(
    o: &Options,
    adapter: &dyn Adapter,
    s: &Start,
    commit: &str,
    ref_frames: Option<u32>,
) -> LevelResult {
    let out = route_path(o, adapter.name(), s);
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    // A route left by an earlier run must not be verified as this one's.
    let _ = std::fs::remove_file(&out);
    let log = out.with_extension("log");
    let mut cmd = adapter.command(s, o.seed, &out, o.budget);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&log).unwrap());
    eprintln!(
        "sf-bench: {}{} on {} {} …",
        adapter.name(),
        if adapter.guided() { " (guided)" } else { "" },
        o.set,
        s.level
    );
    let run = child::run(cmd, o.budget + GRACE, None).unwrap_or_else(|e| panic!("{e}"));
    let stderr = std::fs::read_to_string(&log).unwrap_or_default();
    let stats = results::plan_stats(&stderr);
    let v = judge(&run, stats.as_ref(), o.budget, || match InputScript::load(&out) {
        Ok(script) if !script.lines.is_empty() => verify::verify(o.set, s, &script, o.seed),
        Ok(_) => Verdict::failed("no route"),
        Err(e) => Verdict::failed(format!("no route ({e})")),
    });
    let mut v = v;
    if !v.completed {
        v.reason = results::with_planner_reason(&v.reason, &stderr);
    }
    let num = |k: &str| stats.as_ref().and_then(|m| m.get(k)).and_then(|v| v.parse::<f64>().ok());
    let r = LevelResult {
        planner: adapter.name().to_string(),
        set: o.set.to_string(),
        level: s.level.to_string(),
        commit: commit.to_string(),
        completed: v.completed,
        wall_s: run.wall.as_secs_f64(),
        engine_frames: num("engine_frames").unwrap_or(0.0) as u64,
        oracle_frames: num("oracle_frames").unwrap_or(0.0) as u64,
        confirm_share: num("confirm_share").unwrap_or(0.0),
        route_frames: v.route_frames,
        ref_frames,
        hearts_lost: v.hearts_lost,
        reason: v.reason,
        tool: None,
        guided: adapter.guided(),
        threads: adapter.threads().map(|t| t as u32),
    };
    eprintln!(
        "sf-bench: {} on {} {}: {} in {:.1} min",
        r.planner,
        r.set,
        r.level,
        if r.completed { "completed".to_string() } else { format!("failed ({})", r.reason) },
        r.wall_s / 60.0
    );
    r
}

/// The run: every level of the set (or `levels`) not yet recorded for this commit, `jobs` at a
/// time; `RESULTS.md` re-rendered after each level.
pub fn bench(o: &Options) -> Result<(), String> {
    if o.planner != "new" {
        return Err(format!(
            "planner {}? only `new` (sf-plan) runs: the old tools were retired (T34), their recorded \
             `best-old` results stay in the table",
            o.planner
        ));
    }
    if o.follow_standalone && o.set != Set::B {
        return Err("--follow-standalone is for --set B".into());
    }
    let new = NewPlanner {
        follow_standalone: o.follow_standalone,
        guided: !o.unguided,
        threads: o
            .threads
            .unwrap_or_else(|| crate::adapters::threads_per_job(crate::adapters::cores(), o.jobs)),
    };
    crate::adapters::build(&[new.target()])?;
    let mut starts = sets::starts(o.set);
    if let Some(ls) = &o.levels {
        for l in ls {
            if !starts.iter().any(|s| s.level.to_string() == *l) {
                return Err(format!("{l} is not one of the 24 normal levels"));
            }
        }
        starts.retain(|s| ls.contains(&s.level.to_string()));
    }
    let commit = commit();
    let all = Mutex::new(results::load(&o.results));
    let next = AtomicUsize::new(0);
    let record = |r: &LevelResult| {
        let mut all = all.lock().unwrap();
        results::append(&o.results, r).unwrap_or_else(|e| panic!("{}: {e}", o.results.display()));
        all.push(r.clone());
        let _ = std::fs::write(&o.table, crate::table::render_in(&all, o.table.parent().unwrap_or(Path::new("."))));
    };
    std::thread::scope(|sc| {
        for _ in 0..o.jobs.max(1) {
            sc.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(s) = starts.get(i) else { return };
                    let level = s.level.to_string();
                    let set = o.set.to_string();
                    let done = |p: &str, g: bool| results::recorded(&all.lock().unwrap(), p, g, &set, &level, &commit);
                    if done(new.name(), new.guided()).is_some() {
                        eprintln!("sf-bench: {} on {set} {level} recorded for {commit}; skipped", o.planner);
                        continue;
                    }
                    let r = run_one(o, &new, s, &commit, verify::ref_frames(o.set, s));
                    record(&r);
                }
            });
        }
    });
    let all = all.into_inner().unwrap();
    std::fs::write(&o.table, crate::table::render_in(&all, o.table.parent().unwrap_or(Path::new(".")))).map_err(|e| format!("{}: {e}", o.table.display()))?;
    eprintln!("sf-bench: wrote {} and {}", o.results.display(), o.table.display());
    Ok(())
}
