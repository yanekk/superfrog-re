//! The whole of W1-1 (T27, DESIGN §2.11, §1): the committed route `routes/W1-1.txt`, planned by
//! `fd-plan`, replayed on the engine alone from the level's start to the level's end with no life
//! lost, past every milestone below; and, in full, the planner finding that route again. Needs the
//! disks: absent, it fails naming where it looked. The route on the original is T48's.
//!
//! The route is inputs only. A change to the engine that moves the character by a pixel anywhere
//! on it breaks this file: plan again with the command in the route's header (about 4 minutes on
//! 10 threads), replace the route, and set `MILESTONES` from the failure's printed track.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use amiga_conform::script;
use amiga_input::FrameInput;
use amiga_plan::run::RunOptions;
use amiga_plan::{Compass, Options, PlanGame};
use fd_core::{Event, Game, GameData, Scene, Start};
use fd_plan::{PlanState, plan, script_text};

const ROUTE: &str = include_str!("routes/W1-1.txt");
/// The route's steps; `main_loop` finds the level's end in the last.
const FRAMES: usize = 22528;
/// The rounds the search is given; the exit comes in round 2 095 (seed 1, the walk compass).
const ROUNDS: u64 = 2100;
/// Lives at the level's start (docs/09).
const LIVES: i16 = 32;

/// Where the engine alone is on the route, every 1 500 steps and at its end (measured 2026-11-10):
/// the step, the character's cell and the items still to take.
const MILESTONES: [(usize, (u16, u16), u16); 16] = [
    (1499, (114, 92), 251),
    (2999, (87, 72), 223),
    (4499, (26, 79), 190),
    (5999, (172, 90), 159),
    (7499, (36, 70), 144),
    (8999, (174, 59), 140),
    (10499, (201, 31), 116),
    (11999, (189, 7), 98),
    (13499, (103, 38), 68),
    (14999, (92, 4), 43),
    (16499, (122, 17), 34),
    (17999, (143, 21), 27),
    (19499, (63, 22), 22),
    (20999, (230, 81), 20),
    (22499, (209, 92), 0),
    (22527, (208, 92), 0),
];
/// A milestone is met within this many steps of its own and within a cell of its cell.
const WITHIN_STEPS: usize = 50;

fn data() -> Arc<GameData> {
    static D: OnceLock<Arc<GameData>> = OnceLock::new();
    D.get_or_init(|| fd_conform::game_data().unwrap_or_else(|e| panic!("{e}")))
        .clone()
}

/// The committed route's inputs, a step each.
fn route() -> Vec<FrameInput> {
    let s = script::parse(ROUTE).unwrap_or_else(|e| panic!("routes/W1-1.txt: {e}"));
    let n = s.lines.last().expect("an empty route").0;
    (0..n).map(|k| s.input_at(k)).collect()
}

/// What one step of the replay left: the cell, the items left, the lives and whether play goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct At {
    cell: (u16, u16),
    items: u16,
    lives: i16,
    dying: bool,
}

/// The route on the engine alone, from the level's start: every step's reading, the events with
/// their steps, and the game after the last.
fn replayed() -> &'static (Vec<At>, Vec<(usize, Event)>, PlanState) {
    static R: OnceLock<(Vec<At>, Vec<(usize, Event)>, PlanState)> = OnceLock::new();
    R.get_or_init(|| {
        let mut g = PlanState(Game::new(data(), Start::Level1));
        let (mut track, mut events) = (Vec::new(), Vec::new());
        for (k, &i) in route().iter().enumerate() {
            // The game's own step: `PlanState::step` drops the events.
            g.0.step(i);
            events.extend(g.0.take_events().into_iter().map(|e| (k, e)));
            track.push(At {
                cell: g.cell(),
                items: g.collectibles_left(),
                lives: g.lives(),
                dying: g.dying(),
            });
        }
        (track, events, g)
    })
}

#[test]
fn the_route_reaches_the_levels_end_with_no_life_lost() {
    let (track, events, g) = replayed();
    assert_eq!(track.len(), FRAMES);
    // No life lost and never dying, on every step, not only at the end: a route may pass a
    // comparison while dying (DESIGN §2.11).
    let lost = track.iter().position(|a| a.lives != LIVES || a.dying);
    assert_eq!(lost, None, "a life lost at step {lost:?}");
    // The level's end and nothing else: no object left unported, no game over, no restart.
    assert_eq!(*events, vec![(FRAMES - 1, Event::LevelEnd)]);
    assert!(g.game().finished() && !g.playing());
    assert!(
        matches!(g.game().scene(), Scene::LevelEnd(_)),
        "{:?}",
        g.game().scene()
    );
    // Every item needed is taken (282 at the start, docs/09) and the character stands in the exit
    // (`W1-1:207,91` to `208,93`, docs/10).
    assert_eq!(g.collectibles_left(), 0);
    assert_eq!(g.location().to_string(), "W1-1:208,92");
    // A step earlier play still went on: the route is no longer than it needs.
    assert_eq!(track[FRAMES - 2].items, 0);
    assert!(events.iter().all(|(k, _)| *k == FRAMES - 1));
}

#[test]
fn the_engine_alone_meets_every_milestone_within_tolerance() {
    let (track, _, _) = replayed();
    let near =
        |a: &At, cell: (u16, u16)| a.cell.0.abs_diff(cell.0) <= 1 && a.cell.1.abs_diff(cell.1) <= 1;
    let mut missed = Vec::new();
    for &(step, cell, items) in &MILESTONES {
        let from = step.saturating_sub(WITHIN_STEPS);
        let to = (step + WITHIN_STEPS).min(track.len() - 1);
        if !track[from..=to]
            .iter()
            .any(|a| near(a, cell) && a.items <= items)
        {
            let a = track[step.min(track.len() - 1)];
            missed.push(format!(
                "step {step}: want W1-1:{},{} with {items} items left, at W1-1:{},{} with {}",
                cell.0, cell.1, a.cell.0, a.cell.1, a.items
            ));
        }
    }
    assert!(missed.is_empty(), "{}", missed.join("\n"));
    // The milestones are in order and end at the route's last step, the level's end.
    assert!(
        MILESTONES
            .windows(2)
            .all(|w| w[0].0 < w[1].0 && w[0].2 >= w[1].2)
    );
    assert_eq!(MILESTONES.last().unwrap().0, FRAMES - 1);
}

/// The route as the planner writes it reads back as the committed one, so the fixture is a route
/// `fd-routeview --script` and a case can play.
#[test]
fn the_committed_route_is_a_route_script() {
    let inputs = route();
    let text = script_text(&[], &inputs);
    let body: String = ROUTE
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(text, body);
}

/// The planner finds the whole level (DESIGN §2.11): the same route as the committed one, to the
/// exit, the rounds ending the search and never the clock.
#[test]
#[ignore = "full: a 2 095-round search, 20 million engine frames, 2.5 to 4 minutes on 10 threads"]
fn the_planner_finds_the_committed_route() {
    let run = RunOptions {
        threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
        budget: Duration::from_secs(3600),
        max_rounds: Some(ROUNDS),
        ..RunOptions::default()
    };
    let opt = Options {
        compass: Compass::Walk,
        ..Options::default()
    };
    let p = plan(&data(), opt, &run);
    assert_eq!(p.stalled, None, "stalled at {}", p.end);
    assert_eq!(p.end.to_string(), "W1-1:208,92");
    assert_eq!(p.route.hearts_lost, 0);
    assert!(
        p.route.inputs == route(),
        "another route: {} steps",
        p.route.inputs.len()
    );
}
