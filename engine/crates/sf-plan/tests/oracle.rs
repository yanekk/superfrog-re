//! Original mode (T10, DESIGN §2.6): stretch and repair against the oracle.
//!
//! The parting rule and the repeat rule are unit tests. The W1-1 stretch (~20 s on 10 threads,
//! release) and the wall-loop fixture (~5 s) run in the block. The whole-level runs are
//! `#[ignore]`d: W2-1 (~1.5 min), W5-2 unguided and W1-4 guided by its committed route (each up to
//! 30 min, T10's Done-when). Run them with
//! `cargo test --release -p sf-plan --test oracle -- --ignored --nocapture`.

mod common;

use std::time::{Duration, Instant};

use common::{id, threads};
use sf_conform::{Case, InputScript, Milestone, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::FrameInput;
use sf_core::layout::h0;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::repair::{points_from_trace, trace_engine};
use sf_plan::core::*;
use sf_plan::oracle::*;
use sf_plan::run::{RunOptions, plan_engine};
use sf_plan::trace::trace_original;

const CAP: u32 = 40_000;
/// A lag bar no test stretch reaches.
const NO_LAG: Lag = Lag {
    limit: u32::MAX,
    run: 0,
};

fn look(cell: Cell, lives: u16, hearts: u16) -> Look {
    Look {
        cell,
        lives,
        hearts,
        dying: false,
        playing: true,
    }
}

#[test]
fn a_heart_the_engine_kept_parts() {
    let before = look((10, 20), 3, 3);
    let orig = look((10, 20), 3, 2);
    let eng = look((10, 20), 3, 3);
    assert_eq!(parts(&before, &orig, &eng, false), Some(Parting::Heart));
    // Both lost it: the engine foresaw the hit, no parting.
    assert_eq!(parts(&before, &orig, &orig, false), None);
    // The engine lost one the original did not: not a parting either (the original is ahead).
    assert_eq!(parts(&before, &eng, &orig, false), None);
}

#[test]
fn the_parting_rule() {
    let before = look((10, 20), 3, 3);
    let same = look((11, 19), 3, 3);
    assert_eq!(parts(&before, &same, &look((10, 20), 3, 3), false), None);
    assert_eq!(
        parts(&before, &same, &look((13, 19), 3, 3), false),
        Some(Parting::Apart)
    );
    assert_eq!(
        parts(&before, &same, &look((11, 21), 3, 3), false),
        Some(Parting::Apart)
    );
    assert_eq!(
        parts(&before, &look((10, 20), 2, 3), &same, false),
        Some(Parting::LifeLost)
    );
    // A 1-up is not a loss.
    assert_eq!(parts(&before, &look((10, 20), 4, 3), &same, false), None);
    let dying = Look {
        dying: true,
        ..same
    };
    assert_eq!(parts(&before, &dying, &same, false), Some(Parting::Dying));
    assert_eq!(parts(&before, &same, &same, true), Some(Parting::WallLoop));
}

#[test]
fn the_same_parting_cell_three_times_running_is_accepted() {
    let mut r = Repeats::default();
    assert!(!r.accept((5, 5), &Parting::Heart));
    assert!(!r.accept((5, 5), &Parting::Heart));
    assert!(r.accept((5, 5), &Parting::Heart), "the third is accepted");
    // Accepted, the count starts over: a fourth parting there is repaired.
    assert!(!r.accept((5, 5), &Parting::Heart));

    // Another cell in between breaks the run.
    let mut r = Repeats::default();
    assert!(!r.accept((5, 5), &Parting::Apart));
    assert!(!r.accept((6, 5), &Parting::Apart));
    assert!(!r.accept((5, 5), &Parting::Apart));
    assert!(!r.accept((5, 5), &Parting::Apart));
    assert!(r.accept((5, 5), &Parting::Apart));

    // A death or a hang is never accepted, however often.
    let mut r = Repeats::default();
    for _ in 0..5 {
        assert!(!r.accept((5, 5), &Parting::Dying));
    }
    for why in [
        Parting::LifeLost,
        Parting::WallLoop,
        Parting::Oracle("cap".into()),
    ] {
        assert!(!why.acceptable());
    }
}

/// The start the planner gets: the oracle past the level-start freeze, its state transplanted.
fn transplanted(run: &OracleRun) -> Game {
    Game::with_display(
        sf_conform::display_data(),
        Start::Transplant(sync::core_memory(run.amiga.memory())),
    )
}

/// W1-1's first stretch: planned on the engine from the original's state, played in lockstep, and
/// the frames kept (to 16 before any parting) replay on the original with the frog in the engine's
/// cells, within one.
#[test]
fn w1_1_first_stretch_keeps_frames_the_original_agrees_to() {
    let mut run = start_at_level(id("W1-1"), CAP);
    assert!(run.inputs.len() > 150 && run.oracle_frames == run.inputs.len() as u64);
    let fa = facts_at(&run, id("W1-1"));
    let start = transplanted(&run);
    let ro = RunOptions {
        threads: threads(),
        ..RunOptions::default()
    };
    let plan = plan_engine(start.clone(), fa.clone(), Options::default(), &ro)
        .expect("W1-1 plans to its exit on the engine");
    let snap = run.amiga.snapshot();
    let mut frames = 0;
    let ls = lockstep(&mut run.amiga, start.clone(), &plan.inputs, &mut frames, NO_LAG);
    assert!(frames as usize >= ls.agreed && ls.agreed > 0, "{ls:?}");
    let keep = if ls.exited {
        ls.agreed
    } else {
        ls.agreed.saturating_sub(BACK_OFF)
    };
    assert!(ls.used_at(keep) <= keep, "a held frame uses no plan frame");
    eprintln!(
        "W1-1 plan {} frames, agreed {}, exited {}, parting {:?}",
        plan.inputs.len(),
        ls.agreed,
        ls.exited,
        ls.parting
    );
    run.amiga.restore(&snap);
    let script = script_of(&ls.fed[..keep]);
    let orig = trace_original(&mut run.amiga, &fa, &script, keep as u32).unwrap();
    let eng = trace_engine(&start, &fa, &plan.inputs[..ls.used_at(keep)]);
    assert!(!orig.is_empty());
    // Held frames make the original's trace the longer; it is compared where the two have points.
    assert!(orig.len() >= eng.len());
    for (k, (o, e)) in orig.iter().zip(&eng).enumerate() {
        assert!(
            o.0.0.abs_diff(e.0.0) <= 1 && o.0.1.abs_diff(e.0.1) <= 1,
            "frame {}: original {:?}, engine {:?}",
            k * 8,
            o.0,
            e.0
        );
    }
}

/// W1-2's scroller freezes the original's frog nine frames at x 1625 (frame 3 954 of this sf-plan
/// route, T14): played in lockstep from frame 3 900, the engine is held there and the two do not
/// part; stepped every frame, they parted at 3 955 on every repair. ~15 s.
#[test]
fn a_frozen_original_holds_the_engine() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/w1-2-scroller-lag.txt");
    let s = InputScript::load(&p).unwrap();
    let from = 3_900;
    let mut amiga = sync::reach(&SyncPoint::AtLevel(id("W1-2")), &InputScript::idle(), CAP)
        .unwrap()
        .amiga;
    for k in 0..from {
        sync::step_inserting(&mut amiga, s.input_at(k)).unwrap();
    }
    let eng = Game::with_display(
        sf_conform::display_data(),
        Start::Transplant(sync::core_memory(amiga.memory())),
    );
    let plan: Vec<FrameInput> = (from..from + 120).map(|k| s.input_at(k)).collect();
    let mut frames = 0;
    let ls = lockstep(&mut amiga, eng, &plan, &mut frames, NO_LAG);
    assert!(ls.parting.as_ref().is_none_or(|_| ls.agreed > 100), "{:?} at {}", ls.parting, ls.agreed);
    let held = ls.agreed - ls.used_at(ls.agreed);
    assert!((8..=10).contains(&held), "{held} frames held");
}

/// An accepted parting plays the same plan on; a repaired one, or an accepted one with nothing
/// left, plans again from the plan's rest (T14).
#[test]
fn an_accepted_parting_resumes_the_plan() {
    let x = |j: u8| FrameInput::joy(sf_core::input::Joy(j));
    let plan = Route {
        inputs: vec![x(1), x(2), x(4), x(8)],
        engine_frames: 500,
        hearts_lost: 1,
        rounds: 7,
        cells: 9,
    };
    match next_stretch(true, &plan, 1) {
        Next::Resume(r) => {
            assert_eq!(r.inputs, vec![x(2), x(4), x(8)]);
            assert_eq!((r.engine_frames, r.rounds, r.hearts_lost), (0, 0, 1));
        }
        n => panic!("{n:?}"),
    }
    assert_eq!(next_stretch(false, &plan, 1), Next::Repair(vec![x(2), x(4), x(8)]));
    assert_eq!(next_stretch(true, &plan, 4), Next::Repair(vec![]));
}

/// What a parting keeps (T14 review): counted in original frames, the held ones included, while
/// the plan resumes or is repaired from the plan frames those used; lag-tail frames are not the
/// plan's. A 40-frame plan, the original frozen three frames from its 10th (held), parting at
/// the 21st original frame or, past the plan's end, at the 45th.
#[test]
fn a_parting_keeps_original_frames_and_resumes_at_plan_frames() {
    let used_after = |j: usize| match j {
        0..10 => j + 1,
        10..13 => 10,
        _ => j - 2,
    };
    let ls = |n: usize| Lockstep {
        agreed: n - 1,
        exited: false,
        parting: Some((Parting::Apart, (1, 1))),
        fed: vec![FrameInput::NONE; n],
        used: (0..n).map(used_after).collect(),
        mid_run: (0..n as u32).collect(),
        hearts_lost: 0,
    };
    // Accepted mid-plan: the parting frame kept, the plan resumed at the frame it would use next.
    assert_eq!(kept(&ls(21), 40, true, 5), (21, 18, 20));
    // Repaired: BACK_OFF original frames before the parting.
    assert_eq!(kept(&ls(21), 40, false, 5), (20 - BACK_OFF, 4, 3));
    // Past the plan's end: agreed is counted to the plan's last frame (original frame 43).
    assert_eq!(kept(&ls(45), 40, false, 5), (43 - BACK_OFF, 24, 26));
    assert_eq!(kept(&ls(45), 40, true, 5), (44, 40, 43));
    // Nothing kept: the run carried in stands.
    assert_eq!(kept(&ls(10), 40, false, 5), (0, 0, 5));
}

/// A lag parting is repaired from before the mid-pass run it ended (T25: W5-2 re-planned from
/// inside one 200 times). A 60-frame plan, the original mid-pass from its 25th frame to the 50th,
/// where the run passes the bar; with the run begun before the stretch, its start is kept.
#[test]
fn a_lag_parting_keeps_the_frames_before_its_run() {
    let ls = |parting: Parting, mid_run: Vec<u32>| Lockstep {
        agreed: 49,
        exited: false,
        parting: Some((parting, (1, 1))),
        fed: vec![FrameInput::NONE; 50],
        used: (1..=50).collect(),
        mid_run,
        hearts_lost: 0,
    };
    let late: Vec<u32> = (0..50).map(|j| if j < 24 { 0 } else { j - 23 }).collect();
    // The run's frames are 25–50 (`mid_run` 1–26): the 24 before it are kept.
    assert_eq!(kept(&ls(Parting::Lag, late.clone()), 60, false, 0), (24, 24, 0));
    // Any other parting backs off `BACK_OFF` frames, into the run.
    assert_eq!(kept(&ls(Parting::Apart, late), 60, false, 0), (49 - BACK_OFF, 33, 9));
    // A run carried in from the last stretch and never ended: nothing kept, the run stands.
    let carried: Vec<u32> = (0..50).map(|j| 60 + j + 1).collect();
    assert_eq!(kept(&ls(Parting::Lag, carried), 60, false, 60), (0, 0, 60));
}

/// The faithful check's lag bar (T14): this sf-plan W5-3 route keeps the original mid-pass 101
/// frames in a row from frame 5 310, one past `LAG_LIMIT`. Played in lockstep from 5 390, the run
/// so far carried in, it parts for lag at the 101st under the level's bar, and not under a looser
/// one. ~20 s.
#[test]
fn a_stretch_past_the_lag_bar_parts() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/w5-3-long-lag.txt");
    let s = InputScript::load(&p).unwrap();
    let from = 5_390;
    let at = || {
        let mut amiga = sync::reach(&SyncPoint::AtLevel(id("W5-3")), &InputScript::idle(), CAP)
            .unwrap()
            .amiga;
        let mut run = 0;
        for k in 0..from {
            sync::step_inserting(&mut amiga, s.input_at(k)).unwrap();
            let mid = sync::mid_pass(&amiga.cpu(), &sync::core_memory(amiga.memory()));
            run = if mid { run + 1 } else { 0 };
        }
        let eng = Game::with_display(
            sf_conform::display_data(),
            Start::Transplant(sync::core_memory(amiga.memory())),
        );
        (amiga, eng, run)
    };
    let plan: Vec<FrameInput> = (from..from + 300).map(|k| s.input_at(k)).collect();
    let limit = lag_limit_for(id("W5-3"), false);
    assert_eq!(limit, sf_conform::LAG_LIMIT);
    let (mut amiga, eng, run) = at();
    assert!(run > 50, "{run} frames mid-pass at {from}");
    let mut frames = 0;
    let ls = lockstep(&mut amiga, eng, &plan, &mut frames, Lag { limit, run });
    let (why, _) = ls.parting.clone().expect("a parting");
    assert_eq!(why, Parting::Lag, "at {}", ls.agreed);
    assert_eq!(ls.mid_run.last(), Some(&(limit + 1)));
    assert!(!why.acceptable());
    let (mut amiga, eng, run) = at();
    let ls = lockstep(&mut amiga, eng, &plan, &mut frames, Lag { limit: 400, run });
    assert_ne!(ls.parting.map(|p| p.0), Some(Parting::Lag));
}

/// The lag bar is the level's completion case's (`lag-limit` where it pins one), the whole
/// game's for a leg snapshot, `LAG_LIMIT` otherwise (T14).
#[test]
fn the_lag_bar_is_the_completion_cases() {
    assert_eq!(lag_limit_for(id("W4-3"), false), 261);
    assert_eq!(lag_limit_for(id("W3-1"), false), 105);
    assert_eq!(lag_limit_for(id("W1-1"), false), sf_conform::LAG_LIMIT);
    let whole = sf_conform::Case::load(&sf_conform::cases_dir().join("game-whole.toml")).unwrap();
    assert_eq!(
        lag_limit_for(id("W1-1"), true),
        whole.lag_limit.map_or(sf_conform::LAG_LIMIT, |l| l.frames)
    );
}

/// The frog placed against W2-4's closed gate at x 303, y 912, facing left: the original loops in
/// `frog_tiles`' wall loop and the engine falls on (rust-engine FINDINGS T168, `wall_place`).
/// Played in lockstep, idle, it parts at once, never later.
#[test]
fn a_route_into_the_wall_loop_parts_there() {
    let mut run = start_at_level(id("W2-4"), 2_000);
    for (o, v) in [(f::X, 303u16), (f::Y, 912), (f::FACING, 1), (f::VX, 0)] {
        run.amiga.poke(FROG + o, &v.to_be_bytes());
    }
    let start = transplanted(&run);
    let mut frames = 0;
    let ls = lockstep(&mut run.amiga, start, &[FrameInput::NONE; 200], &mut frames, NO_LAG);
    eprintln!(
        "W2-4 against the gate: agreed {}, {:?}",
        ls.agreed, ls.parting
    );
    assert!(!ls.exited);
    assert_eq!(ls.parting.map(|p| p.0), Some(Parting::WallLoop));
    assert!(ls.agreed < 10, "parted at {}", ls.agreed);
}

#[test]
fn stats_count_the_start_and_keep_the_share_in_range() {
    let run = start_at_level(id("W1-1"), CAP);
    let n = run.inputs.len();
    let fa = facts_at(&run, id("W1-1"));
    let ro = RunOptions {
        budget: Duration::ZERO,
        ..RunOptions::default()
    };
    let (route, stats, reason) =
        plan_original(run, fa, Options::default(), &ro).expect_err("no budget, no route");
    assert_eq!(
        stats.oracle_frames, n as u64,
        "the start's frames are counted"
    );
    assert!((0.0..=1.0).contains(&stats.confirm_share), "{stats:?}");
    assert_eq!(stats.confirmed_frames, n as u32);
    assert_eq!(&route.inputs[..n], &vec![FrameInput::NONE; n][..]);
    assert!(reason.coins_left > 0);
}

/// A route's inputs as a script (one line per change, closing `N: -`), as `sf-plan` writes it.
fn script_of(inputs: &[FrameInput]) -> InputScript {
    let mut s = String::from("# timing: original\n");
    let mut last = None;
    for (k, x) in inputs.iter().enumerate() {
        if Some(*x) != last {
            s += &format!("{k}: {}\n", InputScript::format_input(*x));
            last = Some(*x);
        }
    }
    s += &format!("{}: -\n", inputs.len());
    InputScript::parse(&s).unwrap()
}

/// The committed route `sf-bench` compares a set-A level with (`verify::committed_script`).
fn committed(level: LevelId) -> String {
    let l = level.to_string().to_lowercase();
    match l.as_bytes()[1] {
        b'1' | b'4' => format!("complete-{l}"),
        b'2' => format!("{l}-complete"),
        _ => format!("enemies-{l}-complete"),
    }
}

/// DESIGN §4.1's check, as `sf-bench`'s `verify` makes it (not linked: sf-bench depends on the
/// planners): a case from `AtLevel`, the route, 25 frames past it, resync, the committed case's
/// lag limit, passes `sf_conform::run` and its milestones include the level's exit. Returns the
/// hearts the original lost on the way.
fn verify(level: LevelId, script: &InputScript) -> Result<u32, String> {
    let frames = script.lines.last().map_or(0, |l| l.0) + 25;
    let text = format!("sync = {{ level = \"{level}\" }}\nframes = {frames}\nresync = true\n");
    let mut case = Case::parse(&format!("t10-{level}"), &text)?;
    case.script = script.clone();
    let path = sf_conform::cases_dir().join(format!("{}.toml", committed(level)));
    case.lag_limit = Case::load(&path)?.lag_limit;
    let o = sf_conform::run(&case).map_err(|d| {
        format!(
            "not faithful: {}",
            d.to_string().lines().next().unwrap_or("")
        )
    })?;
    let t = level.to_string();
    let b = t.as_bytes();
    let (world, lv) = ((b[1] - b'0') as u16, (b[3] - b'0') as u16);
    let exit = Milestone::LevelExit { world, level: lv };
    if !o
        .milestones
        .as_ref()
        .is_some_and(|m| m.iter().any(|(_, x)| *x == exit))
    {
        return Err("no exit".into());
    }
    // The hearts lost, read off the original replaying it.
    let mut a = sync::reach(
        &SyncPoint::AtLevel(level),
        &InputScript::idle(),
        frames + 100,
    )?
    .amiga;
    let hearts = |a: &sf_oracle::Amiga| {
        let b = a.peek(h0(0x830), 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    let (mut h, mut lost) = (hearts(&a), 0);
    for k in 0..frames {
        sync::step_inserting(&mut a, script.input_at(k))?;
        let now = hearts(&a);
        lost += h.saturating_sub(now) as u32;
        h = now;
    }
    Ok(lost)
}

/// Plans `level` in original mode from `AtLevel`, guided by `guide`'s committed route if given,
/// and checks the route by §4.1. Prints the FINDINGS numbers.
fn original_mode(level: &str, guide: Option<&str>, minutes: u64) -> (Stats, u32) {
    let t0 = Instant::now();
    let level = id(level);
    let run = start_at_level(level, CAP);
    let fa = facts_at(&run, level);
    let mut opt = Options::default();
    if let Some(g) = guide {
        let script = InputScript::load(&sf_conform::case::script_path(g)).unwrap();
        let n = script.lines.last().map_or(0, |l| l.0);
        let mut a = sync::reach(&SyncPoint::AtLevel(level), &InputScript::idle(), n + 200)
            .unwrap()
            .amiga;
        opt.guide = points_from_trace(&trace_original(&mut a, &fa, &script, n).unwrap());
        assert!(!opt.guide.is_empty());
    }
    let ro = RunOptions {
        threads: threads(),
        budget: Duration::from_secs(minutes * 60).saturating_sub(t0.elapsed()),
        ..RunOptions::default()
    };
    let (route, stats) = match plan_original(run, fa, opt, &ro) {
        Ok(r) => r,
        Err((r, s, why)) => panic!(
            "{level}: no route in {minutes} min: {why:?}, {s:?}, {} frames",
            r.inputs.len()
        ),
    };
    let secs = t0.elapsed().as_secs();
    let script = script_of(&route.inputs);
    let lost = verify(level, &script).unwrap_or_else(|e| panic!("{level}: {e}"));
    eprintln!(
        "{level} original mode{}: {secs} s, route {} frames, repairs {}, confirm share {:.3}, \
         engine frames {}, oracle frames {}, hearts lost {lost} (planner says {}), rounds {}",
        guide.map_or(String::new(), |g| format!(" guided by {g}")),
        route.inputs.len(),
        stats.repairs,
        stats.confirm_share,
        stats.engine_frames,
        stats.oracle_frames,
        route.hearts_lost,
        route.rounds,
    );
    assert!(stats.oracle_frames > 0 && (0.0..=1.0).contains(&stats.confirm_share));
    assert_eq!(stats.confirmed_frames as usize, route.inputs.len());
    assert_eq!(
        route.hearts_lost as u32, lost,
        "the planner counts hearts as verify does"
    );
    (stats, lost)
}

#[test]
#[ignore = "~1.5 min on 10 threads (the experiment: 85 s, 1 repair)"]
fn w2_1_original_mode_passes_the_check() {
    original_mode("W2-1", None, 10);
}

#[test]
#[ignore = "T10 Done-when: up to 30 min (the experiment: 482 s, 8 repairs)"]
fn w5_2_original_mode_unguided_passes_the_check() {
    original_mode("W5-2", None, 30);
}

#[test]
#[ignore = "T10 Done-when: up to 30 min"]
fn w1_4_original_mode_guided_passes_the_check() {
    original_mode("W1-4", Some("complete-w1-4"), 30);
}
