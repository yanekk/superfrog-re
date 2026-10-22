//! `sf-plan`, the command (T11, DESIGN §2.1, §2.9, §2.10, §2.11), run as a child as an agent runs it.
//!
//! The quick checks stop at `--frames 100`, inside the level-start freeze, so they cost an oracle
//! start (~3 s) and no search. M8 through the command plans W1-1 twice engine only, guided by its
//! committed route with shortening off: ~25 s a run on a quiet machine (57 s at load 30). The same
//! with `sf-plan`'s defaults (unguided, shortening on) is `#[ignore]`d: 6.5 min a run at load 27.
//! Run it with `cargo test --release -p sf-plan --test cli -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sf-plan-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `sf-plan args`, the hint log pointed at `log`.
fn plan(args: &[&str], log: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sf-plan"))
        .args(args)
        .env("SF_PLAN_HINTS_LOG", log)
        .output()
        .unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// The `PLAN_STATS` line's `key=value` words.
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

#[test]
fn a_bad_level_or_option_exits_2_with_a_usage_line() {
    let d = scratch("usage");
    let log = d.join("hints.log");
    for args in [
        &["W9-1"][..],
        &["W1-1", "--seed"],
        &["W1-1", "--via", "3"],
        &["W1-1", "--compass", "north"],
        &["WB-1"],
        &[],
    ] {
        let o = plan(args, &log);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {}", stderr(&o));
        assert!(stderr(&o).contains("usage: sf-plan <level>"), "{args:?}");
    }
    assert!(!log.exists());
}

/// `--frames 100` ends inside the level-start freeze: exit 1, the partial route written in its
/// timing, a reason line with the coins still needed, the least exit distance with the coins met
/// and the cells archived (DESIGN §2.9), and no hint log line.
#[test]
fn frames_exhausted_exits_1_with_a_partial_route_and_a_reason() {
    let d = scratch("frames");
    let (log, out) = (d.join("hints.log"), d.join("route.txt"));
    let o = plan(&["W1-1", "--frames", "100", "--out", out.to_str().unwrap()], &log);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    let reason = e.lines().find(|l| l.starts_with("sf-plan: no route")).expect(&e);
    for want in ["coins still needed", "least exit distance with the coins met", "cells archived", "confirmed to frame"] {
        assert!(reason.contains(want), "{want}: {reason}");
    }
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("# timing: original\n"), "{text}");
    let s = sf_conform::InputScript::parse(&text).unwrap();
    assert_eq!(s.lines.last().map(|l| l.0), Some(100));
    assert_eq!(stat(&o, "hints"), "0");
    assert_eq!(stat(&o, "guided"), "0");
    assert_eq!(stat(&o, "route_frames"), "100");
    assert!(!log.exists(), "a run with no hints writes nothing to the hint log");
}

/// A `--via` run appends one line to the hint log (date, level, start, hints, completed) and says
/// `hints=1`; a second appends a second.
#[test]
fn a_hinted_run_is_logged() {
    let d = scratch("hints");
    let log = d.join("hints.log");
    std::fs::write(&log, "# header\n").unwrap();
    let o = plan(&["W1-1", "--frames", "100", "--via", "40,60"], &log);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert_eq!(stat(&o, "hints"), "1");
    let text = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[1].contains(" W1-1 at-level --via 40,60 completed=no"), "{}", lines[1]);
    assert!(lines[1].as_bytes()[..10].iter().filter(|&&b| b == b'-').count() == 2, "a date first: {}", lines[1]);
    let o = plan(&["W1-1", "--frames", "100", "--avoid", "1,2,3,4", "--via", "5,6"], &log);
    assert_eq!(stat(&o, "hints"), "2");
    assert_eq!(std::fs::read_to_string(&log).unwrap().lines().count(), 3);
}

/// A waypoint guide is not a hint: `guided=1`, nothing logged. A guide with a bad line exits 2
/// naming it.
#[test]
fn a_waypoint_guide_is_guided_and_not_logged() {
    let d = scratch("guide");
    let (log, wp) = (d.join("hints.log"), d.join("w.txt"));
    std::fs::write(&wp, "# W1-1 by hand\n20,60\n40,58,80\n").unwrap();
    let o = plan(&["W1-1", "--frames", "100", "--guide", wp.to_str().unwrap()], &log);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert_eq!(stat(&o, "guided"), "1");
    assert_eq!(stat(&o, "hints"), "0");
    assert!(!log.exists());

    std::fs::write(&wp, "20,60\n40,x\n").unwrap();
    let o = plan(&["W1-1", "--guide", wp.to_str().unwrap()], &log);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("line 2"), "{}", stderr(&o));

    // A script guide with a bad line names it too, before the oracle starts.
    std::fs::write(&wp, "# timing: original\n0: -\n10: Q\n").unwrap();
    let o = plan(&["W1-1", "--guide", wp.to_str().unwrap()], &log);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("line 3"), "{}", stderr(&o));
    // A guide script that never leaves its start cell.
    std::fs::write(&wp, "# timing: original\n0: -\n400: -\n").unwrap();
    let o = plan(&["W1-1", "--guide", wp.to_str().unwrap()], &log);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("never leaves its start"), "{}", stderr(&o));
}

/// M8 through the command: one seed, one level, one start, two thread counts, byte-identical
/// route files that reach the exit.
#[test]
fn engine_only_twice_gives_the_same_file() {
    let d = scratch("m8");
    let log = d.join("hints.log");
    let guide = committed("complete-w1-1");
    let run = |name: &str, threads: &str| {
        let out = d.join(name);
        let o = plan(
            &[
                "W1-1", "--engine-only", "--seed", "1", "--shorten-share", "0", "--guide", &guide,
                "--threads", threads, "--out", out.to_str().unwrap(),
            ],
            &log,
        );
        assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
        assert_eq!(stat(&o, "guided"), "1");
        std::fs::read(&out).unwrap()
    };
    let a = run("a.txt", "4");
    let b = run("b.txt", "8");
    assert_eq!(a, b, "same seed, same route file, any thread count (M8)");
    let text = String::from_utf8(a).unwrap();
    assert!(text.starts_with("# timing: original\n"));
    // The tally's tail: fire 69 frames after the exit frame, held 4.
    let s = sf_conform::InputScript::parse(&text).unwrap();
    let n = s.lines.len();
    assert!(n >= 3);
    let (fire, end) = (s.lines[n - 2].0, s.lines[n - 1].0);
    assert_eq!(end - fire, 4);
    assert_eq!(s.lines[n - 3], (fire - 69, sf_core::input::FrameInput::NONE));
    assert!(!log.exists());
}

/// M8 with `sf-plan`'s defaults: unguided, shortening on.
#[test]
#[ignore]
fn engine_only_defaults_twice_give_the_same_file() {
    let d = scratch("m8-defaults");
    let log = d.join("hints.log");
    let run = |name: &str| {
        let out = d.join(name);
        let o = plan(&["W1-1", "--engine-only", "--seed", "1", "--out", out.to_str().unwrap()], &log);
        assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
        eprintln!("{}", stderr(&o).lines().last().unwrap());
        std::fs::read(&out).unwrap()
    };
    assert_eq!(run("a.txt"), run("b.txt"));
}
