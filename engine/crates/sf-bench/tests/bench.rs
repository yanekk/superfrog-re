//! sf-bench (smart-planner T01): sets, verification, the results file, the judge, the children and
//! the table.

use std::process::Command;
use std::time::{Duration, Instant};

use sf_bench::child::{self, ChildRun};
use sf_bench::results::{self, LevelResult};
use sf_bench::run::{best_of, judge};
use sf_bench::sets::{self, LEVELS, Set, StartKind};
use sf_bench::verify::{self, Verdict};

fn scratch(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sf-bench-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn sets_a_and_c_list_the_24_normal_levels_in_order() {
    for (set, kind) in [(Set::A, StartKind::AtLevel), (Set::C, StartKind::App)] {
        let s = sets::starts(set);
        let names: Vec<String> = s.iter().map(|s| s.level.to_string()).collect();
        assert_eq!(names, LEVELS.to_vec());
        assert!(s.iter().all(|s| s.kind == kind));
    }
}

/// Set B's starts are the 24 normal legs' snapshots (`leg-NN.snap`, the bonus level's 20 and the
/// witch's 25 left out), each at the frame its snapshot was taken.
#[test]
fn set_b_lists_the_24_normal_legs_from_their_snapshots() {
    let dir = scratch("legs");
    for i in 0..26u32 {
        let mut b = (1000 + i).to_le_bytes().to_vec();
        b.push(0);
        std::fs::write(dir.join(format!("leg-{i:02}.snap")), b).unwrap();
    }
    let s = sets::starts_in(Set::B, &dir);
    let names: Vec<String> = s.iter().map(|s| s.level.to_string()).collect();
    assert_eq!(names, LEVELS.to_vec());
    let frames: Vec<u32> = s
        .iter()
        .map(|s| match &s.kind {
            StartKind::Snap(_, f) => *f,
            k => panic!("{k:?}"),
        })
        .collect();
    let want: Vec<u32> = (0..20).chain(21..25).map(|i| 1000 + i).collect();
    assert_eq!(frames, want);
    assert!(!names.iter().any(|n| n == "WB-1" || n == "WW-1"));
    // A missing leg is an error, not a silent gap.
    std::fs::remove_file(dir.join("leg-24.snap")).unwrap();
    assert!(!sets::legs_present(&dir));
    assert!(sets::starts_from(Set::B, &dir).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn committed_routes_are_the_completion_scripts_and_chain_legs() {
    let l = |s: &str| s.parse().unwrap();
    assert_eq!(verify::committed_script(Set::A, &l("W1-1")), "complete-w1-1");
    assert_eq!(verify::committed_script(Set::A, &l("W2-4")), "w2-4-complete");
    assert_eq!(verify::committed_script(Set::A, &l("W3-1")), "enemies-w3-1-complete");
    assert_eq!(verify::committed_script(Set::C, &l("W4-4")), "complete-w4-4");
    assert_eq!(verify::committed_script(Set::B, &l("W6-3")), "chain-w6-3");
    assert_eq!(verify::committed_script(Set::B, &l("W5-2")), "enemies-w5-2-complete");
    for set in [Set::A, Set::B] {
        for level in sets::levels() {
            let p = sf_conform::case::script_path(&verify::committed_script(set, &level));
            assert!(p.is_file(), "{}", p.display());
        }
    }
}

/// Slow: ~2 min on a quiet machine (two faithful runs of ~4 500 frames and their replays).
#[test]
#[ignore]
fn verify_accepts_complete_w1_1_and_rejects_it_cut_before_the_exit() {
    let t0 = Instant::now();
    let s = &sets::starts(Set::A)[0];
    let script =
        sf_conform::InputScript::load(&sf_conform::case::script_path("complete-w1-1")).unwrap();
    let v = verify::verify(Set::A, s, &script, 1);
    assert!(v.completed, "{v:?}");
    // The case's comment: the exit at frame 4473.
    assert!(v.route_frames.is_some_and(|f| (4470..=4476).contains(&f)), "{v:?}");
    let cut = 4473 - 200;
    let mut short = script.clone();
    short.lines.retain(|l| l.0 < cut);
    short.lines.push((cut, sf_core::input::FrameInput::NONE));
    let v = verify::verify(Set::A, s, &short, 1);
    assert!(!v.completed, "{v:?}");
    assert_eq!(v.route_frames, None);
    eprintln!("verify test took {:.0?}", t0.elapsed());
}

/// A leg whose snapshot frame ends mid-pass (T31) is synced `lead` frames on, and the route played
/// from there: its first `lead` frames must hold what the whole game holds there.
#[test]
fn a_leg_with_a_lead_is_synced_and_played_from_the_frame_after() {
    use sf_conform::{InputScript, SyncPoint};
    use sf_core::input::{FrameInput, Joy};
    let s = sets::Start {
        level: "W2-2".parse().unwrap(),
        kind: StartKind::Snap("leg-05.snap".into(), 48527),
    };
    let at = |frame| SyncPoint::AtFrame { script: "game-whole".into(), frame, df1: true };
    assert_eq!(verify::sync_point(&s, 0), at(48527));
    assert_eq!(verify::sync_point(&s, 1), at(48528));
    let r = FrameInput::joy(Joy::RIGHT);
    let route = InputScript { lines: vec![(0, FrameInput::NONE), (150, r), (300, FrameInput::NONE)] };
    let whole = InputScript { lines: vec![(48000, r), (48520, FrameInput::NONE), (48700, r)] };
    assert_eq!(verify::from_lead(&route, None, 0).unwrap(), route);
    let played = verify::from_lead(&route, Some((&whole, 48527)), 1).unwrap();
    assert_eq!(played.lines, vec![(0, FrameInput::NONE), (149, r), (299, FrameInput::NONE)]);
    // A route holding something else on a frame the sync plays is refused, not silently replaced.
    let early = InputScript { lines: vec![(0, r)] };
    assert!(verify::from_lead(&early, Some((&whole, 48527)), 1).is_err());
}

/// The set-B legs' leads (T31): the three whose snapshot frame ends in the loop's first pass, and
/// none for the other 21, whose starts are unchanged. Needs the oracle and the leg snapshots
/// (`sf-bench` makes them once, ~10 min); then ~10 s.
#[test]
#[ignore = "needs the oracle and set B's leg snapshots"]
fn set_b_leads_are_one_frame_at_w2_2_w5_1_w6_3_and_none_elsewhere() {
    for s in sets::starts(Set::B) {
        let want = u32::from(matches!(s.level.to_string().as_str(), "W2-2" | "W5-1" | "W6-3"));
        assert_eq!(verify::lead(&s).unwrap(), want, "{}", s.level);
    }
}

/// The committed `chain-w2-2`, `chain-w5-1` and `chain-w6-3` complete faithfully from their set-B
/// starts as `sf-bench` builds them (T31; each parted at frame 1 when synced at the snapshot's
/// frame). Slow: the first run reaches each sync point from power-on (up to ~9 min each), then
/// ~1 min a route.
#[test]
#[ignore = "slow: the whole game's oracle to each leg, then three faithful runs"]
fn chain_w2_2_w5_1_w6_3_complete_from_their_set_b_starts() {
    let starts = sets::starts(Set::B);
    for l in ["W2-2", "W5-1", "W6-3"] {
        let s = starts.iter().find(|s| s.level.to_string() == l).unwrap();
        let name = verify::committed_script(Set::B, &s.level);
        let script = sf_conform::InputScript::load(&sf_conform::case::script_path(&name)).unwrap();
        let v = verify::verify(Set::B, s, &script, 1);
        assert!(v.completed, "{name}: {v:?}");
    }
}

fn fixture(planner: &str, set: &str, level: &str, completed: bool, wall_s: f64) -> LevelResult {
    LevelResult {
        planner: planner.into(),
        set: set.into(),
        level: level.into(),
        commit: "abc123".into(),
        completed,
        wall_s,
        engine_frames: 1_000_000,
        oracle_frames: 20_000,
        confirm_share: 0.5,
        route_frames: completed.then_some(4000),
        ref_frames: Some(3500),
        hearts_lost: 0,
        reason: if completed { String::new() } else { "no exit \"quoted\"".into() },
        tool: None,
        // The benchmark guides only the new planner (DESIGN §4.2).
        guided: planner == "new",
        threads: (planner == "new").then_some(5),
    }
}

#[test]
fn results_append_reload_and_skip_a_level_recorded_for_the_same_commit() {
    let dir = scratch("results");
    let p = dir.join("results.jsonl");
    assert!(results::load(&p).is_empty());
    let a = fixture("plan_exit", "A", "W1-1", true, 61.5);
    let mut b = fixture("best-old", "A", "W1-4", false, 1800.0);
    b.tool = Some("explore".into());
    b.route_frames = None;
    results::append(&p, &a).unwrap();
    results::append(&p, &b).unwrap();
    let back = results::load(&p);
    assert_eq!(back, vec![a.clone(), b.clone()]);
    assert!(results::recorded(&back, "plan_exit", false, "A", "W1-1", "abc123").is_some());
    assert!(results::recorded(&back, "plan_exit", false, "A", "W1-1", "def456").is_none());
    assert!(results::recorded(&back, "plan_exit", false, "B", "W1-1", "abc123").is_none());
    assert!(results::recorded(&back, "explore", false, "A", "W1-1", "abc123").is_none());
    // A run killed mid-line: the cut line is ignored, not fatal, and the next append starts a
    // line of its own.
    let mut f = std::fs::read_to_string(&p).unwrap();
    f += "{\"planner\":\"new\",\"set\":\"A\",\"lev";
    std::fs::write(&p, f).unwrap();
    assert_eq!(results::load(&p).len(), 2);
    let c = fixture("new", "A", "W2-1", true, 10.0);
    results::append(&p, &c).unwrap();
    assert_eq!(results::load(&p), vec![a, b, c]);
    std::fs::remove_dir_all(&dir).unwrap();
}

fn ran(wall: Duration) -> ChildRun {
    ChildRun {
        status: Command::new("true").status().ok(),
        timed_out: false,
        wall,
    }
}

#[test]
fn plan_stats_is_read_off_the_last_stats_line() {
    let e = "noise\nPLAN_STATS engine_frames=1 hints=0\nmore\nPLAN_STATS engine_frames=12 oracle_frames=34 confirm_share=0.25 hints=2\n";
    let s = results::plan_stats(e).unwrap();
    assert_eq!(s["engine_frames"], "12");
    assert_eq!(s["oracle_frames"], "34");
    assert_eq!(s["confirm_share"], "0.25");
    assert_eq!(s["hints"], "2");
    assert!(results::plan_stats("no stats\n").is_none());
}

/// DESIGN §2.10: the benchmark passes no hints, so a result whose stats say it used one does not
/// count, however good its route.
#[test]
fn a_result_whose_stats_used_hints_is_not_completed() {
    let budget = Duration::from_secs(1800);
    let good = || Verdict {
        completed: true,
        route_frames: Some(4000),
        hearts_lost: 0,
        reason: String::new(),
    };
    let stats = results::plan_stats("PLAN_STATS engine_frames=1 oracle_frames=1 confirm_share=0.1 hints=1").unwrap();
    let v = judge(&ran(Duration::from_secs(60)), Some(&stats), budget, good);
    assert!(!v.completed);
    assert_eq!(v.reason, "hinted");
    let stats = results::plan_stats("PLAN_STATS hints=0").unwrap();
    assert!(judge(&ran(Duration::from_secs(60)), Some(&stats), budget, good).completed);
    // An old tool, which takes no budget, past it: not within budget.
    let v = judge(&ran(budget + Duration::from_secs(1)), Some(&stats), budget, good);
    assert_eq!((v.completed, v.reason.as_str()), (false, "over-budget"));
}

/// A failed run carries the planner's own reason line after the bench's (T14).
#[test]
fn a_failure_keeps_the_planners_reason() {
    let err = "sf-plan: guide traced\nsf-plan: no route: budget spent; coins still needed 0, least exit \
               distance with the coins met 15 cells, cells archived 9\nPLAN_STATS hints=0\n";
    assert_eq!(
        results::with_planner_reason("over-budget", err),
        "over-budget; budget spent; coins still needed 0, least exit distance with the coins met 15 cells, cells archived 9"
    );
    assert_eq!(results::with_planner_reason("timeout", "PLAN_STATS hints=0\n"), "timeout");
}

/// DESIGN §5.2: a child past its timeout (budget + 5 min in a run; a second here) is killed, its
/// whole process group with it, and recorded as `timeout`.
#[test]
fn a_child_past_its_timeout_is_killed_and_recorded_as_timeout() {
    let t0 = Instant::now();
    let mut cmd = Command::new("/bin/sh");
    // A grandchild too: the group goes, not only the child.
    cmd.args(["-c", "sleep 61.25 & sleep 61.5; wait"]);
    let run = child::run(cmd, Duration::from_secs(1), None).unwrap();
    assert!(run.timed_out);
    assert!(t0.elapsed() < Duration::from_secs(10), "{:?}", t0.elapsed());
    let v = judge(&run, None, Duration::from_secs(1800), || panic!("not verified"));
    assert_eq!((v.completed, v.reason.as_str()), (false, "timeout"));
    std::thread::sleep(Duration::from_millis(300));
    assert!(!pgrep("sleep 61.25") && !pgrep("sleep 61.5"), "a child outlived its kill");
}

fn pgrep(pattern: &str) -> bool {
    Command::new("pgrep").args(["-f", pattern]).output().is_ok_and(|o| o.status.success())
}

/// A benchmark killed outright (SIGKILL, which it cannot catch) leaves no planner running: each
/// child's guard sees the benchmark gone and kills the child's group.
#[test]
fn a_child_dies_with_the_process_it_runs_for() {
    let mut stand_in = Command::new("sleep").arg("62.75").spawn().unwrap();
    let watch = stand_in.id();
    let killer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        stand_in.kill().unwrap();
        stand_in.wait().unwrap();
    });
    let t0 = Instant::now();
    let mut cmd = Command::new("/bin/sh");
    cmd.args(["-c", "sleep 62.25 & sleep 62.5; wait"]);
    let run = child::run(cmd, Duration::from_secs(60), Some(watch)).unwrap();
    killer.join().unwrap();
    assert!(!run.timed_out);
    assert!(t0.elapsed() < Duration::from_secs(10), "{:?}", t0.elapsed());
    std::thread::sleep(Duration::from_millis(300));
    assert!(!pgrep("sleep 62.25") && !pgrep("sleep 62.5"), "a child outlived the benchmark");
}

/// A line written before `guided` existed (T01–T02's sample) reads as unguided.
#[test]
fn an_old_line_with_no_guided_field_reads_as_unguided() {
    let line = r#"{"planner":"new","set":"A","level":"W1-1","commit":"abc123","completed":true,"wall_s":61.5,"engine_frames":1,"oracle_frames":2,"confirm_share":0.1000,"route_frames":4000,"ref_frames":4474,"hearts_lost":0,"reason":""}"#;
    let r = LevelResult::from_json(line).expect("an old line parses");
    assert!(!r.guided);
    let mut g = r.clone();
    g.guided = true;
    assert!(g.to_json().contains("\"guided\":true"));
    assert_eq!(LevelResult::from_json(&g.to_json()), Some(g.clone()));
    assert_ne!(r.key(), g.key());
    assert!(LevelResult::from_json(&line.replace(r#""reason":"""#, r#""guided":"yes","reason":"""#)).is_none());
}

/// T14's unguided pass into the file of the guided one: neither skips the other's level.
#[test]
fn guided_and_unguided_results_of_one_level_are_both_recorded() {
    let g = fixture("new", "A", "W1-1", true, 60.0);
    let mut u = g.clone();
    u.guided = false;
    u.completed = false;
    u.reason = "timeout".into();
    let only_g = vec![g.clone()];
    assert!(results::recorded(&only_g, "new", true, "A", "W1-1", "abc123").is_some());
    assert!(results::recorded(&only_g, "new", false, "A", "W1-1", "abc123").is_none());
    let both = vec![g.clone(), u.clone()];
    assert_eq!(results::recorded(&both, "new", false, "A", "W1-1", "abc123"), Some(u.clone()));
    assert_eq!(results::recorded(&both, "new", true, "A", "W1-1", "abc123"), Some(g));
    let t = sf_bench::table::render(&both);
    assert!(t.contains("| Level | `new` guided | `new` unguided |"), "{t}");
    // The measures read the guided result, not the later unguided one.
    let m1 = t.lines().find(|l| l.starts_with("| M1 |")).unwrap();
    assert!(m1.contains("| — | 1 / 24 (1 run) |"), "{m1}");
}

#[test]
fn best_old_keeps_a_completed_route_then_the_shortest() {
    let mut a = fixture("plan_exit", "A", "W1-1", false, 100.0);
    let mut b = fixture("plan_exit_w6", "A", "W1-1", true, 900.0);
    let mut c = fixture("explore", "A", "W1-1", true, 300.0);
    b.route_frames = Some(4000);
    c.route_frames = Some(4500);
    a.route_frames = Some(10);
    let best = best_of(&[a, b, c]);
    assert_eq!(best.planner, "best-old");
    assert_eq!(best.tool.as_deref(), Some("plan_exit_w6"));
    assert!(best.completed);
}

#[test]
fn the_table_shows_baseline_and_new_side_by_side() {
    let mut rs = Vec::new();
    for (i, l) in LEVELS.iter().enumerate() {
        rs.push(fixture("best-old", "A", l, i < 20, 600.0 + i as f64));
        let mut n = fixture("new", "A", l, true, 300.0);
        n.route_frames = Some(if i == 3 { 5000 } else { 3600 });
        rs.push(n);
    }
    rs.push(fixture("new", "B", "W1-1", true, 120.0));
    let t = sf_bench::table::render(&rs);
    for id in ["M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "C1", "C3", "C4"] {
        assert!(t.contains(&format!("| {id} |")), "no {id} row:\n{t}");
    }
    let row = |id: &str| t.lines().find(|l| l.starts_with(&format!("| {id} |"))).unwrap().to_string();
    assert!(row("M1").contains("| 20 / 24 | 24 / 24 | 24 / 24 |"), "{}", row("M1"));
    // W1-4 is baseline-completed (index 3 < 20); W2-4 (7) too; W6-2, W6-3 (21, 22) not.
    assert!(row("M2").contains("| 2 / 4 | 4 / 4 |"), "{}", row("M2"));
    assert!(row("M3").contains("median 5.0 min, slowest 5.0 min"), "{}", row("M3"));
    // W1-4's new route is 5000 frames against 3500: one over +25%.
    assert!(row("M4").contains("1 over +25% (worst +43%"), "{}", row("M4"));
    assert!(row("M5").contains("| — | 1 / 24 (1 run) |"), "{}", row("M5"));
    assert!(t.contains("## Set A") && t.contains("## Set B"));
    assert!(t.contains("| Level | `best-old` unguided | `new` guided |"), "{t}");
    assert!(t.contains("failed (no exit \"quoted\")"));
}

fn args(c: &Command) -> Vec<String> {
    c.get_args().map(|a| a.to_string_lossy().into_owned()).collect()
}

#[test]
fn the_planner_command_carries_its_share_of_the_cores() {
    use sf_bench::adapters::{Adapter, NewPlanner, OLD, threads_per_job};
    assert_eq!(threads_per_job(10, 1), 10);
    assert_eq!(threads_per_job(10, 2), 5);
    assert_eq!(threads_per_job(10, 3), 3);
    assert_eq!(threads_per_job(2, 4), 1);
    assert_eq!(threads_per_job(10, 0), 10);
    let s = &sets::starts(Set::A)[0];
    let out = std::path::Path::new("route.txt");
    let budget = Duration::from_secs(30 * sf_bench::MINUTE);
    let new = NewPlanner {
        follow_standalone: false,
        guided: false,
        threads: threads_per_job(10, 2),
    };
    let a = args(&new.command(s, 1, out, budget));
    let i = a.iter().position(|x| x == "--threads").expect("--threads");
    assert_eq!(a[i + 1], "5", "{a:?}");
    assert_eq!(new.threads(), Some(5));
    // The old tools take no thread option.
    for t in OLD {
        assert!(!args(&t.command(s, 1, out, budget)).contains(&"--threads".to_string()));
        assert_eq!(t.threads(), None);
    }
}

/// A line records the planner's threads; a new-planner line without them (written before sf-bench
/// passed `--threads`, owner 2026-10-24) still reads, is kept in the table and is marked there.
#[test]
fn a_new_planner_line_without_threads_is_kept_and_marked() {
    let with = fixture("new", "A", "W1-1", true, 60.0);
    assert!(with.to_json().contains("\"threads\":5,"));
    assert_eq!(LevelResult::from_json(&with.to_json()), Some(with.clone()));
    let mut before = fixture("new", "A", "W1-2", true, 60.0);
    before.threads = None;
    before.commit = "old000".into();
    assert!(!before.to_json().contains("threads"));
    assert_eq!(LevelResult::from_json(&before.to_json()), Some(before.clone()));
    let old_tool = fixture("plan_exit", "A", "W1-1", true, 60.0);
    let t = sf_bench::table::render(&[with, before, old_tool]);
    let line = |l: &str, p: &str| t.lines().find(|x| x.starts_with(&format!("| {l} |"))).map(|x| x.contains(p));
    assert_eq!(line("W1-2", "*before the thread share*"), Some(true), "{t}");
    assert_eq!(line("W1-1", "*before the thread share*"), Some(false), "{t}");
    assert!(t.contains("Cells marked *before the thread share*"), "{t}");
    let m1 = t.lines().find(|l| l.starts_with("| M1 |")).unwrap();
    assert!(m1.contains("2 / 24"), "{m1}");
}

/// T11: the new planner benched on W1-1, guided by `complete-w1-1`, is recorded completed and
/// guided; an `--unguided` run into the same file is recorded beside it, not skipped as already
/// recorded, and the table shows both columns. Slow: the guided run is minutes on the oracle
/// (sf-plan in original mode, then `verify`), the unguided one up to its 10-minute budget. Run with
/// `cargo test --release -p sf-bench --test bench -- --ignored new_planner --nocapture`.
#[test]
#[ignore]
fn new_planner_guided_then_unguided_on_w1_1() {
    let dir = scratch("new-w1-1");
    let (res, table) = (dir.join("results.jsonl"), dir.join("RESULTS.md"));
    let bench = |extra: &[&str]| {
        let st = Command::new(env!("CARGO_BIN_EXE_sf-bench"))
            .args(["--set", "A", "--planner", "new", "--levels", "W1-1"])
            .arg("--results")
            .arg(&res)
            .arg("--table")
            .arg(&table)
            .args(extra)
            .status()
            .unwrap();
        assert!(st.success());
    };
    bench(&[]);
    let rs = results::load(&res);
    assert_eq!(rs.len(), 1, "{rs:?}");
    assert!(rs[0].guided && rs[0].completed, "{:?}", rs[0]);
    bench(&["--unguided", "--budget-min", "10"]);
    let rs = results::load(&res);
    assert_eq!(rs.len(), 2, "the unguided run is not skipped: {rs:?}");
    assert!(!rs[1].guided && rs[1].planner == "new", "{:?}", rs[1]);
    let t = std::fs::read_to_string(&table).unwrap();
    assert!(t.contains("| Level | `new` guided | `new` unguided |"), "{t}");
    eprintln!("{}", std::fs::read_to_string(&res).unwrap());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Set C (T12): a route's joystick becomes the app's key lines, its frame `i` the app's step
/// `i + 1` after Start Level at step 1 under the route's power-on seed; the engine's replay of
/// the committed W1-1 app route from the app start reaches the exit with no life lost, and its
/// state hashes are what the app's `hashes.txt` is compared with.
#[test]
fn set_c_turns_a_route_into_the_apps_script_and_replays_it_on_the_engine() {
    let route = sf_conform::InputScript::parse("# timing: app\n0: -\n3: R\n5: R F\n6: -\n8: -\n").unwrap();
    let level = "W1-2".parse().unwrap();
    assert_eq!(
        verify::app_script(level, 7, &route, 8),
        "seed 7 7\n1: start-level W1-2\n4: down ArrowRight\n6: down Space\n7: up ArrowRight\n7: up Space\n"
    );

    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sf-e2e/tests/data/w1-1-app-seed1.txt"),
    )
    .unwrap();
    let route = sf_conform::InputScript::parse(&text).unwrap();
    let w1_1 = "W1-1".parse().unwrap();
    let v = verify::verify_app(w1_1, 1, &route, verify::AppCheck::Engine);
    assert!(v.completed, "{v:?}");
    let r = verify::engine_app(w1_1, 1, &route, 200);
    assert_eq!(r.states.len(), 200);
    // The app's frame 0 (before Start Level) is not compared; frames 1.. are.
    let line = |n: usize, s: u64| format!("{n} {:016x} {s:016x}\n", 0);
    let mut hashes = format!("{}\n{}", sf_core::feed::HASHES_HEADER, line(0, 0));
    for (k, s) in r.states.iter().enumerate() {
        hashes += &line(k + 1, *s);
    }
    assert_eq!(verify::app_parts(&hashes, &r.states), None);
    let cut: String = hashes.lines().take(101).map(|l| format!("{l}\n")).collect();
    assert_eq!(verify::app_parts(&cut, &r.states).as_deref(), Some("the app stopped at frame 99 of 200"));
    let other = verify::engine_app(w1_1, 2, &route, 200);
    let mut wrong = format!("{}\n{}", sf_core::feed::HASHES_HEADER, line(0, 0));
    for (k, s) in other.states.iter().enumerate() {
        wrong += &line(k + 1, *s);
    }
    assert_eq!(verify::app_parts(&wrong, &r.states).as_deref(), Some("the app parts from the engine at frame 1"));
}

/// Set C's baseline (T15): `plan_exit` alone plans on the engine from the app's start, at the
/// benchmark's seed, so `best-old` on set C runs it and nothing else.
#[test]
fn set_c_runs_plan_exit_on_the_engine_at_the_seed() {
    use sf_bench::adapters::{Adapter, PLAN_EXIT, old_for};
    let s = &sets::starts(Set::C)[0];
    assert_eq!(s.kind, StartKind::App);
    let a = args(&PLAN_EXIT.command(s, 1, std::path::Path::new("route.txt"), Duration::from_secs(60)));
    assert!(a.contains(&"--engine".to_string()), "{a:?}");
    let i = a.iter().position(|x| x == "--seed").expect("--seed");
    assert_eq!(a[i + 1], "1", "{a:?}");
    let names = |set| old_for(set).iter().map(|t| t.name).collect::<Vec<_>>();
    assert_eq!(names(Set::C), ["plan_exit"]);
    assert_eq!(names(Set::A), ["plan_exit", "plan_exit_w6", "explore"]);
    assert_eq!(names(Set::B), names(Set::A));
}
