//! The planner on W1-1 (T20, DESIGN §2.11): its facts against T00's and T21's readings, a planned
//! route replayed on the engine to its recorded point and drawn on the map page, the same route on
//! any thread count, the budget's stop, and the route played on the original. Needs the disks (and
//! the Kickstart for the original): absent, it fails naming where it looked.
//!
//! While the items (T22), the creatures (T24) and the level's end (T26) are unported the
//! engine never ends the level, so the furthest point possible is the exit's cells: a route is held
//! to reach them, not to leave the level.

use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use amiga_conform::script;
use amiga_plan::run::RunOptions;
use amiga_plan::{Cell, Compass, Options, PlanGame};
use fd_conform::Outcome;
use fd_core::GameData;
use fd_plan::{Planned, confirm, facts, plan, replay, script_text, start};

const BIN: &str = env!("CARGO_BIN_EXE_fd-plan");

/// The short search: seed 1, 100 rounds, ~0.8 s on 10 threads.
const ROUNDS: u64 = 100;
/// Where it ends on the engine (measured 2026-11-06 with contact, T70): beside the exit's cells,
/// 1292 frames in. Before T70 the engine never counted contact and the route was another.
const END: &str = "W1-1:207,90";
const FRAMES: usize = 1292;
/// The original and the engine agree on every compared variable on every pass of it (T70; before,
/// they parted at pass 952 where the original counted a contact pass the engine did not model).
const AGREED: u32 = 1279;

fn data() -> Arc<GameData> {
    static D: OnceLock<Arc<GameData>> = OnceLock::new();
    D.get_or_init(|| fd_conform::game_data().unwrap_or_else(|e| panic!("{e}")))
        .clone()
}

fn run(threads: usize, rounds: u64) -> RunOptions {
    RunOptions {
        threads,
        // The rounds end it, never the clock, so it is the same on any machine.
        budget: Duration::from_secs(600),
        max_rounds: Some(rounds),
        ..RunOptions::default()
    }
}

/// The command's defaults: the walk compass, seed 1.
fn opts() -> Options {
    Options {
        compass: Compass::Walk,
        ..Options::default()
    }
}

fn short() -> &'static Planned {
    static P: OnceLock<Planned> = OnceLock::new();
    P.get_or_init(|| plan(&data(), opts(), &run(4, ROUNDS)))
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fd-plan-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Within a cell of one of `cells`.
fn near(at: Cell, cells: &[Cell]) -> bool {
    cells
        .iter()
        .any(|&(c, r)| at.0.abs_diff(c) <= 1 && at.1.abs_diff(r) <= 1)
}

#[test]
fn the_facts_match_the_levels_readings() {
    let d = data();
    let f = facts(&d.level);
    // docs/05: 240 used columns, 96 rows.
    assert_eq!((f.cols, f.rows), (240, 96));
    // docs/10: the exit is `W1-1:207,91` to `208,93`, six cells.
    let mut exits = f.exits.clone();
    exits.sort_unstable();
    assert_eq!(
        exits,
        vec![
            (207, 91),
            (207, 92),
            (207, 93),
            (208, 91),
            (208, 92),
            (208, 93)
        ]
    );
    // docs/10 (T21): 332 coins, `$20` to `$23`; `items_left` 282 at the start (docs/09).
    assert_eq!(f.collectibles.len(), 128 + 108 + 51 + 45);
    assert!(f.gates.is_empty());
    let s = start(&d);
    assert_eq!(s.collectibles_left(), 282);
    assert!(f.collectibles.iter().all(|&c| s.collectible_live(c)));
    // docs/09: 32 lives at level 1; the character starts at `W1-1:7,7` (`char_x`, `char_y` $70).
    assert_eq!(s.lives(), 32);
    assert_eq!(s.cell(), (7, 7));
    assert!(s.playing() && !s.dying());
    // Shape 0 is solid in every level, shape 2 empty in W1 (docs/05).
    assert!(!s.open((0, 95)) || d.level.shape(0, 95) != 0);
    assert!(s.open((7, 7)));
}

#[test]
fn a_planned_route_reaches_the_exit_on_replay_and_the_page_shows_it() {
    let d = data();
    let p = short();
    assert_eq!(p.route.inputs.len(), FRAMES);
    assert_eq!(p.end.to_string(), END);
    assert!(near((p.end.col, p.end.row), &facts(&d.level).exits));
    // The level does not end (T22, T26), so the search stops at its rounds with the furthest route.
    let why = p.stalled.as_ref().expect("the engine has no level end yet");
    assert_eq!(why.coins_left, 282);

    // The route as its script, read back and replayed on a fresh engine.
    let text = script_text(&["test".into()], &p.route.inputs);
    let s = script::parse(&text).unwrap();
    let inputs: Vec<_> = (0..FRAMES as u32).map(|k| s.input_at(k)).collect();
    assert_eq!(inputs, p.route.inputs);
    let g = replay(&d, &inputs);
    assert_eq!(g.location().to_string(), END);
    assert!(g.playing() && g.lives() == 32);

    // The map page drawn from the script: its track at the route's last frame is the replay's.
    let dir = temp("page");
    let path = dir.join("route.txt");
    std::fs::write(&path, &text).unwrap();
    let (page, dump) = fd_routeview::write_page(&d, &path, &dir.join("page")).unwrap();
    assert!(page.exists());
    assert_eq!(dump.inputs_end as usize, FRAMES);
    let st = g.game().state();
    assert_eq!(dump.track[FRAMES - 1], (st.char_x as i32, st.char_y as i32));
    assert_eq!(dump.notes, vec!["# test".to_string()]);
}

#[test]
fn the_route_is_the_same_across_runs_and_thread_counts() {
    let d = data();
    let a = plan(&d, opts(), &run(1, 30));
    let b = plan(&d, opts(), &run(3, 30));
    let c = plan(&d, opts(), &run(3, 30));
    assert!(a.route.inputs.len() > 100, "it went somewhere");
    assert_eq!(a.route.inputs, b.route.inputs);
    assert_eq!(b.route.inputs, c.route.inputs);
    assert_eq!((a.end, a.route.cells), (b.end, b.route.cells));
    // And the short search's route again, on another thread count.
    assert_eq!(
        plan(&d, opts(), &run(2, ROUNDS)).route.inputs,
        short().route.inputs
    );
}

#[test]
fn the_budget_stops_the_search_and_it_reports_where() {
    let dir = temp("budget");
    let out = dir.join("route.txt");
    let run = Command::new(BIN)
        .args(["level", "W1-1", "--budget-mins", "0", "--no-confirm"])
        .arg("--out")
        .arg(&out)
        .arg("--page")
        .arg(dir.join("page"))
        .output()
        .unwrap();
    let (stdout, stderr) = (
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    assert_eq!(run.status.code(), Some(1), "{stdout}\n{stderr}");
    // No round ran: the furthest route is the start's, written and drawn.
    assert!(stderr.contains("stalled at W1-1:7,7"), "{stderr}");
    assert!(stderr.contains("282 items left"), "{stderr}");
    let last = stdout.lines().last().unwrap();
    assert!(
        last.starts_with("PLAN_STATS frames=0 rounds=0 ") && last.contains("peak_rss_mb="),
        "{last}"
    );
    assert!(out.exists() && dir.join("page/index.html").exists());
    assert_eq!(
        std::fs::read_to_string(&out).unwrap().lines().last(),
        Some("0: -")
    );

    // A bad argument is exit 2 and names it.
    let bad = Command::new(BIN).args(["level", "W2-1"]).output().unwrap();
    assert_eq!(bad.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("only W1-1"));
}

#[test]
fn the_routes_stretch_is_confirmed_on_the_original_where_modelled() {
    let d = data();
    let p = short();
    let v = confirm::confirm(&d, &p.route.inputs).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.goal_met.is_some(), "{v}");
    // Every pass was compared but those the original ended unseen (two in one step): the route's
    // frames and the idle slack after it, less one.
    assert_eq!(v.passes, AGREED, "{v}");
    assert_eq!(
        v.passes + v.unseen,
        (FRAMES as u32) + confirm::SLACK - 1,
        "{v}"
    );
    // The route is the original's past the level's middle, most of the way to the exit.
    let g = replay(&d, &p.route.inputs);
    assert!(g.location().col >= 120, "{}", g.location());
}

/// A stretch the two agree on all the way is confirmed whatever its length: the original ends its
/// passes about one behind the frames run, so a case of exactly the route's frames missed its goal
/// on the last pass at 600 and 900 steps (and met it at 300 and 950).
#[test]
fn a_route_the_original_agrees_with_meets_its_goal_at_any_length() {
    let d = data();
    let p = short();
    for n in [600, 900] {
        let v = confirm::confirm(&d, &p.route.inputs[..n]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(v.outcome, Outcome::Same, "{n} steps: {v}");
        assert!(v.goal_met.is_some(), "{n} steps: {v}");
    }
}

/// The longer search, through the real command: more rounds, the original and the page.
#[test]
#[ignore = "full: a 400-round search played on the original, about 5 s release, 15 s debug"]
fn the_command_plans_a_longer_search_confirms_it_and_draws_it() {
    let dir = temp("long");
    let out = dir.join("route.txt");
    let run = Command::new(BIN)
        .args(["level", "W1-1", "--rounds", "400", "--threads", "4"])
        .arg("--out")
        .arg(&out)
        .arg("--page")
        .arg(dir.join("page"))
        .output()
        .unwrap();
    let (stdout, stderr) = (
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    assert_eq!(run.status.code(), Some(1), "{stdout}\n{stderr}");
    assert!(stdout.contains("the original agrees for "), "{stdout}");
    assert!(dir.join("page/index.html").exists());
    let s = script::parse(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let n = s.lines.last().unwrap().0;
    let inputs: Vec<_> = (0..n).map(|k| s.input_at(k)).collect();
    // Shorter than the short search's, and as far.
    assert!((n as usize) < FRAMES, "{n} frames");
    let d = data();
    let g = replay(&d, &inputs);
    assert!(near(g.cell(), &facts(&d.level).exits), "{}", g.location());
}
