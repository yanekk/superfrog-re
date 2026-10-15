//! Check 4, plays alone (DESIGN §2.1, T182): the engine started once from a case's sync point and
//! given only the inputs, the original's pacing and its clock's seeds, against the original's
//! milestones. The judge's own tests are `milestones.rs`'s; these need what `runner.rs` needs.

mod common;

use sf_conform::{Alone, Case, Milestone, cases_dir, display_data, sync};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{AmigaMemory, Ram, VarId, def};
use sf_core::level::beam::NO_STEP;
use sf_core::{Game, Pacing, Start};

fn case(name: &str) -> Case {
    Case::load(&cases_dir().join(format!("{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

/// Every case that steps the engine runs check 4; a `render`-only case steps none (owner,
/// 2026-10-15).
#[test]
fn every_stepping_case_plays_alone_and_a_render_case_does_not() {
    let stepping = Case::parse("a", "sync = \"power-on\"\nframes = 1\ncompare = [\"vars\"]").unwrap();
    assert_eq!(stepping.alone, Alone::With);
    let render = Case::parse("b", "sync = \"power-on\"\nframes = 1\ncompare = [\"render\"]").unwrap();
    assert_eq!(render.alone, Alone::Off);
    let e = Case::parse("c", "sync = \"power-on\"\nframes = 1\nalone = false").unwrap_err();
    assert!(e.contains("unknown key `alone`"), "{e}");
}

/// A `Pacing::Late` whose cut the engine's pass never reaches (more Paula writes than its frame
/// makes, in a call late or early in the pass, in a pass that has met a vertical blank, which
/// none of the engine's does here) cannot end its frame there: the frame ends on the engine's own
/// clock, as in normal play, and nothing panics (T182: a run alone drifts from the original's
/// writes). In a pass that has met none, as the engine's, the frame ends at the pass's end
/// instead (T184: the engine goes on into no pass the original had not begun).
#[test]
fn a_late_cut_the_engine_cannot_reach_ends_the_frame_as_normal_play_does() {
    let display = display_data();
    let unreachable = [
        Pacing::Late {
            writes: 500,
            line: 200,
            hpos: 0,
            phase: 3,
            step: NO_STEP,
            blanks: 1,
        },
        Pacing::Late {
            writes: 500,
            line: 40,
            hpos: 100,
            phase: 1,
            step: 4,
            blanks: 1,
        },
    ];
    for late in unreachable {
        let mut paced = Game::with_display(display.clone(), Start::Transplant(common::level().clone()));
        let mut normal = paced.clone();
        for f in 0..400 {
            let input = FrameInput {
                joy: if f % 50 < 25 { Joy::RIGHT } else { Joy::RIGHT | Joy::FIRE },
                ..FrameInput::NONE
            };
            paced.step_paced(input, late);
            normal.step(input);
            assert!(!paced.mid_pass(), "{late:?}: frame {f} was cut");
            assert_eq!(paced.probe(), normal.probe(), "{late:?}: frame {f}");
        }
    }
    let late = Pacing::Late {
        writes: 500,
        line: 40,
        hpos: 100,
        phase: 1,
        step: 4,
        blanks: 0,
    };
    let mut paced = Game::with_display(display.clone(), Start::Transplant(common::level().clone()));
    let mut cut = 0;
    for f in 0..400 {
        let input = FrameInput {
            joy: if f % 50 < 25 { Joy::RIGHT } else { Joy::RIGHT | Joy::FIRE },
            ..FrameInput::NONE
        };
        paced.step_paced(input, late);
        cut += paced.mid_pass() as u32;
    }
    assert!(cut > 100, "{cut} frames cut at the pass's end");
}

/// `complete-w1-2` (green under checks 1–3 with no re-sync) played alone from its sync point:
/// every coin, the fruit, the kills, the exit and the tally, as the original.
#[test]
fn complete_w1_2_plays_alone() {
    let case = Case {
        alone: Alone::Only,
        ..case("complete-w1-2")
    };
    let met = sf_conform::run(&case)
        .unwrap_or_else(|d| panic!("{d}"))
        .milestones
        .expect("check 4 ran");
    let count = |p: &dyn Fn(&Milestone) -> bool| met.iter().filter(|(_, m)| p(m)).count();
    // The 68 the exit needs, and one more once it is open.
    assert_eq!(count(&|m| matches!(m, Milestone::CoinTaken { .. })), 69);
    assert_eq!(count(&|m| *m == Milestone::CoinTaken { left: 0 }), 2);
    for left in 1..68 {
        assert_eq!(count(&|m| *m == Milestone::CoinTaken { left }), 1, "{left}");
    }
    assert_eq!(count(&|m| matches!(m, Milestone::EnemyKilled { .. })), 5);
    assert!(count(&|m| matches!(m, Milestone::ItemTaken { kind: 55, .. })) > 20);
    assert_eq!(count(&|m| matches!(m, Milestone::LifeLost { .. })), 0);
    let exits: Vec<u64> = met
        .iter()
        .filter(|(_, m)| *m == Milestone::LevelExit { world: 1, level: 2 })
        .map(|(f, _)| *f)
        .collect();
    assert_eq!(exits, [5182]);
    assert_eq!(count(&|m| matches!(m, Milestone::Tally { .. })), 1);
}

/// The amendment's measurement (2026-10-15): `Start::Level(W1-1)` replaying `complete-w1-1` loses
/// a life, the original with the same inputs does not. Not a mechanic: from the case's sync point
/// with the original's pacing the engine plays the level alone to its exit. Unpaced (normal play
/// never lags) the script, fitted to a lagging original, loses the life at 555. Paced, a start
/// whose keyboard poll is in the other phase (`kbd_toggle`, flipped every blank: a `Start::Level`
/// has it 0, the case's sync `$ff`) keeps it too since T184: its INTENA writes move a late frame's
/// write-counted cut, but no longer into the next pass. `Start::Level`'s leftovers from the front
/// end are T21's (`level_entry.rs`).
#[test]
fn the_w1_1_replay_loses_its_life_to_pacing_and_the_start_not_a_mechanic() {
    let case = case("complete-w1-1");
    let synced = sync::reach(&case.sync, &case.script, 2 * case.frames + 1000).unwrap();
    let mut amiga = synced.amiga;
    let mut paces = Vec::new();
    for f in 0..case.frames {
        let cap = sync::step_inserting(&mut amiga, case.script.input_at(f)).unwrap();
        let mem = sync::core_memory(amiga.memory());
        paces.push((sync::pacing(&amiga.cpu(), &mem, &cap.paula), amiga.frames_run()));
    }
    let display = display_data();
    let lives = def(VarId::FrogLives).addr;
    let first_loss = |start: AmigaMemory, paced: bool| {
        let mut g = Game::with_display(display.clone(), Start::Transplant(start));
        let l0 = g.state().w(lives);
        for f in 0..case.frames {
            let input = case.script.input_at(f);
            let (pacing, n) = if paced { paces[f as usize] } else { (Pacing::Unpaced, 1) };
            for _ in 0..n {
                g.step_paced(input, pacing);
            }
            if g.state().w(lives) != l0 {
                return Some(f);
            }
        }
        None
    };
    assert_eq!(first_loss(synced.memory.clone(), true), None);
    assert_eq!(first_loss(synced.memory.clone(), false), Some(555));
    let k = def(VarId::Keyboard);
    assert_eq!(synced.memory.byte(k.addr), 0xff);
    let mut other_phase = synced.memory.clone();
    other_phase.slow[(k.addr - sf_core::layout::SLOW_BASE) as usize] = 0;
    assert_eq!(first_loss(other_phase, true), None);
    let level = Game::with_display(display.clone(), Start::Level("W1-1".parse().unwrap()));
    assert_eq!(level.peek(k.addr, 1), [0]);
}

/// Check 4 fails a run whose engine, alone, parts from the original. No case is red alone since
/// T184, so the fault is planted: `enemies-w3-activation`, green alone, with the frog of the
/// engine run alone moved 32 pixels right at 202 (the first frame from 200 not in the middle of a
/// pass, which would run again from its start) loses a life the original does not, at 203. Pins
/// that the runner judges the engine's own milestones, not only reads the original's.
#[test]
fn a_case_red_alone_fails_check_4_with_its_milestone_and_first_variable() {
    let case = Case {
        alone: Alone::Only,
        ..case("enemies-w3-activation")
    };
    sf_conform::run(&case).unwrap_or_else(|d| panic!("green alone: {d}"));
    let x = def(VarId::FrogX).addr;
    let mut planted = None;
    let d = sf_conform::run_with(&case, |f, g| {
        if f >= 200 && planted.is_none() && !g.mid_pass() {
            planted = Some(f);
            let at = g.peek(x, 2);
            g.poke(x, &(u16::from_be_bytes([at[0], at[1]]) + 32).to_be_bytes());
        }
    })
    .expect_err("the frog moved");
    assert_eq!(planted, Some(202));
    let m = d.milestone.as_ref().expect("a check 4 divergence");
    assert_eq!(m.frame, 203, "{d}");
    assert_eq!(m.theirs, None, "{d}");
    assert!(matches!(m.ours, Some(Milestone::LifeLost { .. })), "{d}");
    let (at, var) = d.alone_first_var.as_ref().expect("the first differing variable");
    assert_eq!(*at, 203, "{d}");
    assert!(var.starts_with("frog_x"), "{d}");
}
