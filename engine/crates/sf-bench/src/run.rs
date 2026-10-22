//! A benchmark run (DESIGN §4.2): each level of the set planned by a child under the benchmark's
//! rules (the new planner guided by the committed route unless `--unguided`), verified, and its line appended before the next level's result is waited on.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use sf_conform::InputScript;

use crate::adapters::{Adapter, NewPlanner, OLD};
use crate::child::{self, ChildRun};
use crate::results::{self, LevelResult};
use crate::sets::{self, Set, Start};
use crate::verify::{self, Verdict};

/// A planner child past its budget by this much is killed (DESIGN §5.2).
pub const GRACE: Duration = Duration::from_secs(5 * crate::MINUTE);

pub struct Options {
    pub set: Set,
    /// `new`, `plan_exit`, `plan_exit_w6`, `explore` or `best-old`.
    pub planner: String,
    pub levels: Option<Vec<String>>,
    pub jobs: usize,
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
/// budget (an old tool, which takes none) is `over-budget`; otherwise `verify` decides.
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

/// D10's baseline from the three old tools' results on one level: a completed route before a
/// failed one, then the shorter route, then the quicker run.
pub fn best_of(rs: &[LevelResult]) -> LevelResult {
    let mut best = rs
        .iter()
        .min_by(|a, b| {
            (!a.completed, a.route_frames.unwrap_or(u32::MAX))
                .cmp(&(!b.completed, b.route_frames.unwrap_or(u32::MAX)))
                .then(a.wall_s.total_cmp(&b.wall_s))
        })
        .expect("three old tools' results")
        .clone();
    best.tool = Some(best.planner.clone());
    best.planner = "best-old".into();
    best
}

/// The run: every level of the set (or `levels`) not yet recorded for this commit, `jobs` at a
/// time; `RESULTS.md` re-rendered after each level.
pub fn bench(o: &Options) -> Result<(), String> {
    let old = ["plan_exit", "plan_exit_w6", "explore", "best-old"];
    if o.set == Set::C && old.contains(&o.planner.as_str()) {
        return Err("the old tools have no app mode: set C is the new planner's".into());
    }
    if o.follow_standalone && (o.planner != "new" || o.set != Set::B) {
        return Err("--follow-standalone is for --set B --planner new".into());
    }
    if o.unguided && o.planner != "new" {
        return Err("--unguided is for --planner new (the old tools are never guided)".into());
    }
    let new = NewPlanner {
        follow_standalone: o.follow_standalone,
        guided: !o.unguided,
    };
    let adapters: Vec<&dyn Adapter> = match o.planner.as_str() {
        "new" => vec![&new],
        "best-old" => OLD.iter().map(|a| *a as &dyn Adapter).collect(),
        p => vec![*OLD.iter().find(|a| a.name == p).ok_or(format!(
            "planner {p}? (new, plan_exit, plan_exit_w6, explore, best-old)"
        ))?],
    };
    let targets: Vec<_> = adapters.iter().map(|a| a.target()).collect();
    crate::adapters::build(&targets).map_err(|e| {
        if o.planner == "new" { format!("{e} (sf-plan is built by T11)") } else { e }
    })?;
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
        let _ = std::fs::write(&o.table, crate::table::render(&all));
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
                    let guided = o.planner == "new" && !o.unguided;
                    if done(&o.planner, guided).is_some() {
                        eprintln!("sf-bench: {} on {set} {level} recorded for {commit}; skipped", o.planner);
                        continue;
                    }
                    let ref_frames = verify::ref_frames(o.set, s);
                    let mut got = Vec::new();
                    for a in &adapters {
                        let r = done(a.name(), a.guided()).unwrap_or_else(|| {
                            let r = run_one(o, *a, s, &commit, ref_frames);
                            record(&r);
                            r
                        });
                        got.push(r);
                    }
                    if o.planner == "best-old" {
                        record(&best_of(&got));
                    }
                }
            });
        }
    });
    let all = all.into_inner().unwrap();
    std::fs::write(&o.table, crate::table::render(&all)).map_err(|e| format!("{}: {e}", o.table.display()))?;
    eprintln!("sf-bench: wrote {} and {}", o.results.display(), o.table.display());
    Ok(())
}
