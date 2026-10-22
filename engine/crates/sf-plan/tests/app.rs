//! `sf-plan --app` (T12, DESIGN §2.1, §2.6, set C): the level as Game ▸ Start Level… starts it under
//! a script, planned and confirmed on the engine, written `# timing: app`.
//!
//! M8 through `--app` plans W1-1 twice, guided by its committed route with shortening off (~22 s a
//! run on a quiet machine); the column-92 shaft check plans it unguided (~25 s). Repairing the
//! original-timed `complete-w1-1` into app timing (`--follow`) takes ~70 s and is `#[ignore]`d:
//! `cargo test --release -p sf-plan --test app -- --ignored`. The real app's half of set C is
//! sf-e2e's `plan_routes.rs`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use sf_conform::InputScript;
use sf_core::input::FrameInput;
use sf_plan::app;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sf-plan-app-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn plan(args: &[&str], d: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sf-plan"))
        .args(args)
        .env("SF_PLAN_HINTS_LOG", d.join("hints.log"))
        .output()
        .unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn stat(o: &Output, key: &str) -> String {
    let e = stderr(o);
    let line = e.lines().last().unwrap_or("");
    assert!(line.starts_with("PLAN_STATS"), "last stderr line: {e}");
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("no {key} in {line}"))
        .to_string()
}

fn committed(name: &str) -> String {
    sf_conform::case::script_path(name).display().to_string()
}

fn w1_1() -> sf_core::LevelId {
    "W1-1".parse().unwrap()
}

/// The exit frame an exit route's comment names (`# The level ends at frame N,`).
fn exit_frame(text: &str) -> u32 {
    let rest = text.split("# The level ends at frame ").nth(1).expect(text);
    rest.split(',').next().unwrap().parse().unwrap()
}

fn inputs_to(script: &InputScript, n: u32) -> Vec<FrameInput> {
    (0..n).map(|k| script.input_at(k)).collect()
}

/// An app route file: app timing and its seed first, then a route that replays from the app start
/// with `seed` to the exit on the frame its comment names, with no life lost.
fn assert_app_route(text: &str, seed: u32) {
    assert!(text.starts_with(&format!("# timing: app\n# seed: {seed}\n")), "{text}");
    assert_eq!(app::route_seed(text), Some(seed));
    let s = InputScript::parse(text).unwrap();
    let exit = exit_frame(text);
    let r = app::replay(w1_1(), seed, &inputs_to(&s, exit + 100));
    assert!(r.completed(), "{r:?}");
    assert_eq!(r.exit, Some(exit), "the route replayed from the app start, its frame 0 the level's first");
}

#[test]
fn bad_app_options_exit_2() {
    let d = scratch("usage");
    for args in [
        &["W1-1", "--app", "--snap", "x.snap"][..],
        &["W1-1", "--app", "--engine-only"],
        &["W1-1", "--app", "--seed", "4294967296"],
    ] {
        let o = plan(args, &d);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {}", stderr(&o));
        assert!(stderr(&o).contains("usage: sf-plan <level>"), "{args:?}");
    }
    let wp = d.join("g.txt");
    std::fs::write(&wp, "# timing: lagged\n0: -\n10: R\n").unwrap();
    let o = plan(&["W1-1", "--app", "--guide", wp.to_str().unwrap()], &d);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("timing `lagged`"), "{}", stderr(&o));
}

/// `--frames 100` ends in the settle: exit 1, a partial route written `# timing: app` with its seed,
/// and nothing on the original: an app-timed guide is played on the engine from its own app start
/// (DESIGN §2.11), so the run's oracle frames are 0.
#[test]
fn an_app_run_and_an_app_guide_never_touch_the_original() {
    let d = scratch("frames");
    let (g, out) = (d.join("guide.txt"), d.join("route.txt"));
    std::fs::write(&g, "# timing: app\n# seed: 3\n0: -\n180: R\n400: -\n").unwrap();
    let o = plan(
        &["W1-1", "--app", "--seed", "3", "--frames", "100", "--guide", g.to_str().unwrap(), "--out", out.to_str().unwrap()],
        &d,
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    let reason = e.lines().find(|l| l.starts_with("sf-plan: no route")).expect(&e);
    assert!(reason.contains("coins still needed") && !reason.contains("confirmed to frame"), "{reason}");
    assert_eq!(stat(&o, "guided"), "1");
    assert_eq!(stat(&o, "oracle_frames"), "0");
    assert_eq!(stat(&o, "route_frames"), "100");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("# timing: app\n# seed: 3\n"), "{text}");
    assert!(text.contains("A partial route"), "{text}");
    assert_eq!(InputScript::parse(&text).unwrap().lines.last().map(|l| l.0), Some(100));
}

/// M8 through `--app`: one seed, two thread counts, byte-identical route files that reach the exit
/// from the app start with no life lost. The guide is the committed original-timed route, played on
/// the original: cells, not inputs, so it crosses no timing (DESIGN §2.6).
#[test]
fn app_twice_gives_the_same_file_and_it_replays_from_the_app_start() {
    let d = scratch("m8");
    let guide = committed("complete-w1-1");
    let run = |name: &str, threads: &str| {
        let out = d.join(name);
        let o = plan(
            &[
                "W1-1", "--app", "--seed", "1", "--shorten-share", "0", "--guide", &guide, "--threads", threads,
                "--out", out.to_str().unwrap(),
            ],
            &d,
        );
        assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
        assert_eq!(stat(&o, "guided"), "1");
        assert_eq!(stat(&o, "hearts_lost"), "0");
        std::fs::read(&out).unwrap()
    };
    let a = run("a.txt", "4");
    let b = run("b.txt", "8");
    assert_eq!(a, b, "same seed, same route file, any thread count (M8)");
    assert_app_route(&String::from_utf8(a).unwrap(), 1);
    assert!(!d.join("hints.log").exists());
}

/// bug-recorder T11 (2026-10-20): `plan_exit --engine` stalled on W1-1 from the app start, its beam
/// trapped by the floating-ledge coins above the shaft at column 92, until hinted past them. The
/// explorer plans the level unguided to the exit within 2 minutes, through that shaft.
#[test]
fn w1_1s_column_92_shaft_is_not_a_stall() {
    let d = scratch("shaft");
    let out = d.join("route.txt");
    let t = Instant::now();
    let o = plan(
        &["W1-1", "--app", "--seed", "1", "--shorten-share", "0", "--budget-min", "2", "--out", out.to_str().unwrap()],
        &d,
    );
    let took = t.elapsed();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(took < Duration::from_secs(120), "{took:?}");
    assert_eq!(stat(&o, "guided"), "0");
    let text = std::fs::read_to_string(&out).unwrap();
    assert_app_route(&text, 1);
    // The frog passes column 92 on its way.
    let s = InputScript::parse(&text).unwrap();
    let mut g = app::app_start(w1_1(), 1);
    let mut cols = std::collections::BTreeSet::new();
    for k in 0..exit_frame(&text) {
        g.step_unseen(s.input_at(k));
        cols.insert(sf_plan::core::frog_cell(g.state()).0);
    }
    assert!(cols.contains(&92), "column 92 crossed");
}

/// `--app --follow` of an original-timed route repairs it into app timing (DESIGN §2.7): the old
/// route played from the app start does not finish the level (bug-recorder T06: `complete-w1-1`
/// ends in game over from `start_level`), the result does, and it is written `# timing: app`.
#[test]
#[ignore = "slow: ~70 s; cargo test --release -p sf-plan --test app -- --ignored"]
fn follow_repairs_an_original_route_into_app_timing() {
    let d = scratch("follow");
    let old = committed("complete-w1-1");
    let old_script = InputScript::load(Path::new(&old)).unwrap();
    let n = old_script.lines.last().unwrap().0;
    assert!(!app::replay(w1_1(), 1, &inputs_to(&old_script, n)).completed(), "the old route needs repair");
    let out = d.join("route.txt");
    let o = plan(
        &["W1-1", "--app", "--seed", "1", "--shorten-share", "0", "--follow", &old, "--out", out.to_str().unwrap()],
        &d,
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stat(&o, "guided"), "0", "--follow is not --guide");
    assert_app_route(&std::fs::read_to_string(&out).unwrap(), 1);
}
