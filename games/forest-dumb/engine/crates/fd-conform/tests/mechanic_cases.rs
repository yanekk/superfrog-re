//! Level 1's mechanics in play against the original (plans/forest-dumb-level-1 T47, DESIGN §2.1,
//! §2.2, §4.3): T25 ported the springs (`h0_24ce`, `h0_21c0`) and the animated cells (`h0_62b6`) and
//! proved each on the rig; these cases run fd-core's `Game` from the level sync point through each
//! mechanic's spot, its timing cycle and back after scrolling away, comparing every character, camera,
//! edge, status, spring and animation variable on every pass and the picture on every frame. Nothing
//! is frozen in the original but the creatures (T24). Needs the disks and the Kickstart: absent, it
//! fails naming where it looked.

use std::path::Path;
use std::thread;

use amiga_conform::script;
use fd_conform::{Case, Goal, Outcome, Verdict, conform_dir, game_data, run};
use fd_core::{Game, LAYOUT, PORTED};

/// The vanishing blocks at W1-1:82,77..83, and the springs at W1-1:222..224,56.
const CASES: [&str; 2] = ["mechanic-vanish", "mechanic-spring"];

/// The routines T25 registered (`fd-core/src/ported.rs`, its section), and `h0_6882`, which adds a
/// cell scrolled into view to `h0_62b6`'s lists (T55's, frozen in every case until T47).
const T25: [&str; 3] = ["h0_21c0", "h0_24ce", "h0_62b6"];
const ADD_CELL: &str = "h0_6882";

/// The only task an exclusion of these cases may name: the creatures (T24, through `freeze.rs`). The
/// redraw race of a long pass is handed to the engine by the runner (T74).
const EXCLUDED_FOR: [&str; 1] = ["T24"];

fn load(name: &str) -> Case {
    Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

/// Each case on the level engine, each on its own thread (one oracle each).
fn verdicts(cases: Vec<Case>) -> Vec<(Case, Verdict)> {
    let runs: Vec<_> = cases
        .into_iter()
        .map(|case| {
            thread::spawn(move || {
                let v = run(&case).unwrap_or_else(|e| panic!("{}: {e}", case.name));
                (case, v)
            })
        })
        .collect();
    runs.into_iter().map(|t| t.join().unwrap()).collect()
}

fn hits(v: &Verdict, name: &str) -> u32 {
    v.coverage
        .hits
        .iter()
        .filter(|(n, _)| *n == name)
        .map(|(_, h)| h)
        .sum()
}

fn goal(case: &Case) -> (&[(String, u32)], u32) {
    match &case.goal {
        Some(Goal::All { all, by_frame }) => (all, *by_frame),
        _ => panic!("{}: states no goal of variables", case.name),
    }
}

#[test]
fn the_mechanics_match_the_original_on_every_pass() {
    let verdicts = verdicts(CASES.iter().map(|n| load(n)).collect());
    for (case, v) in &verdicts {
        let name = &case.name;
        assert_eq!(v.outcome, Outcome::Same, "{name}: {v}");
        assert!(
            case.compare.vars && case.compare.picture,
            "{name}: compares variables and picture"
        );
        assert!(v.goal_met.is_some(), "{name}: {v}");
        assert!(v.events.is_empty(), "{name}: {v}");
        // Frozen for the creatures alone: freeze.rs holds no other entry (mechanic_routines.rs).
        assert!(case.freeze && case.thaw.is_empty(), "{name}");
        // Nearly every pass compared, and every frame's picture but those showing a pass whose end
        // the original hid (README, "How a level case compares").
        assert!(v.passes + v.unseen + 1 >= case.frames - 5, "{name}: {v}");
        assert!(v.pictures + v.unseen + 10 >= case.frames, "{name}: {v}");
        // No exclusion is left to T25, and every one hid something.
        for (e, n) in &v.excluded {
            assert!(
                EXCLUDED_FOR.contains(&e.task.as_str()),
                "{name}: {}",
                e.what
            );
            assert!(*n > 0, "{name}: {} hid nothing", e.what);
        }
        // Every character, camera, edge, status, spring and animation variable LAYOUT holds.
        for var in LAYOUT.iter().filter(|v| {
            ["char_", "camera_", "edge_", "spring_"]
                .iter()
                .any(|p| v.name.starts_with(p))
                || [
                    "score",
                    "items_left",
                    "lives",
                    "countdown",
                    "anim_tick",
                    "water_cycle",
                ]
                .contains(&v.name)
        }) {
            let compared = case.vars.iter().any(|v| v == var.name);
            let excluded = case
                .exclude
                .iter()
                .any(|e| e.var.as_deref() == Some(var.name));
            assert!(compared || excluded, "{name}: {} not compared", var.name);
        }
        // Each case runs every T25 routine and adds cells scrolled into view, on the original.
        for r in T25.iter().chain([&ADD_CELL]) {
            assert!(PORTED.iter().any(|p| p.name == *r), "{r} is not in PORTED");
            assert!(hits(v, r) > 0, "{name}: {r} never ran: {v}");
        }
    }
    // Both pictures and every edge latch are compared whole, with no exclusion (T74): the passes
    // that came during main_loop's edge redraws were handed to the engine, pass 293 of the
    // vanishing blocks and pass 495 of the springs among them (docs/09).
    for (case, v) in &verdicts {
        assert!(case.exclude.is_empty(), "{}: {:?}", case.name, case.exclude);
        for latch in [
            "edge_latch_down",
            "edge_latch_up",
            "edge_latch_left",
            "edge_latch_right",
        ] {
            assert!(
                case.vars.iter().any(|v| v == latch),
                "{}: {latch}",
                case.name
            );
        }
        assert!(v.edge_cuts >= 10, "{}: {v}", case.name);
    }
}

/// The case's input script, from its sync frame.
fn input(case: &Case) -> script::InputScript {
    let name = case
        .input
        .as_deref()
        .expect("a mechanic case names its input");
    let path = conform_dir().join("scripts").join(format!("{name}.txt"));
    script::parse(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

/// The case's input on its engine alone (started at its own level start, not transplanted), a step
/// a frame up to the case's last frame: the goal's frame (if met) and the state after every step.
fn alone(name: &str) -> (Option<u32>, Vec<Game>) {
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let case = load(name);
    let (all, by_frame) = goal(&case);
    let input = input(&case);
    let mut game = fd_conform::engine_for(&case, data);
    let (mut met, mut states) = (None, Vec::new());
    for f in 0..case.frames {
        game.step(input.input_at(f));
        assert!(game.take_events().is_empty(), "{name}: frame {f}");
        let s = game.state();
        let value = |var: &str| match var {
            "char_x" => s.char_x,
            "char_y" => s.char_y,
            "spring_x" => s.spring.x,
            "spring_timer" => s.spring.timer as u32,
            other => panic!("{other}"),
        };
        if met.is_none()
            && f < by_frame - case.sync_frame
            && all.iter().all(|(v, want)| value(v) == *want)
        {
            met = Some(f);
        }
        states.push(game.clone());
    }
    (met, states)
}

/// Whether the cell `(c, r)` is in the view of a camera at (`x`, `y`): its top-left is level pixel
/// (`x + 17`, `y + 1`), 320 by 192 pixels (docs/09).
fn in_view(g: &Game, (c, r): (u32, u32)) -> bool {
    let cam = &g.state().camera;
    let (x, y) = (cam.x as u32 + 17, cam.y as u32 + 1);
    (x / 16..=(x + 320) / 16).contains(&c) && (y / 16..=(y + 192) / 16).contains(&r)
}

/// The vanishing blocks on the engine alone: jumps under the column meet the block at 82,83 shown
/// (the head stops at `char_y` `$50a` or so) and gone (rising through to `$4d9`); the column leaves the
/// view and comes back; then the character stands on the top block and drops onto the next as each
/// vanishes in turn, to the floor, and the goal (on the middle block) is met.
#[test]
fn the_engine_alone_rides_the_vanishing_blocks_down() {
    let (met, states) = alone("mechanic-vanish");
    assert!(met.is_some(), "goal missed");
    let y = |f: usize| states[f].state().char_y;
    let x = |f: usize| states[f].state().char_x;
    // The jumps' highest points under the column (frames 450 to 850): both kinds.
    let peaks: Vec<u32> = (451..849)
        .filter(|&f| y(f) <= y(f - 1) && y(f) < y(f + 1))
        .map(y)
        .collect();
    assert!(peaks.iter().any(|&p| p < 0x4e0), "{peaks:x?}");
    assert!(peaks.iter().any(|&p| p > 0x500), "{peaks:x?}");
    // Away and back: the column out of view, then in view again.
    let column = (82, 80);
    let away = (860..1100).find(|&f| !in_view(&states[f], column));
    assert!(away.is_some(), "the column never left the view");
    assert!(in_view(&states[1300], column));
    // Standing still on each block in turn, top to bottom, then the floor: no input from 1294 on.
    let mut stands = Vec::new();
    for f in 1300..states.len() {
        if x(f) == 0x531 && y(f) == y(f - 1) && stands.last() != Some(&y(f)) {
            stands.push(y(f));
        }
    }
    assert_eq!(stands, [0x4a8, 0x4d8, 0x508, 0x552]);
}

/// The springs on the engine alone: the spring fires on each landing (its timer runs 1 to 6, the
/// picture's two redraws), about 85 frames apart, once again at once while the timer ran; the springs
/// leave the view down the shaft and come back, and the spring fires again where the goal says.
#[test]
fn the_engine_alone_bounces_on_the_spring_and_comes_back() {
    let (met, states) = alone("mechanic-spring");
    assert!(met.is_some(), "goal missed");
    let spring = |f: usize| &states[f].state().spring;
    // Each fire: the step its timer goes to 1, or stays 1 with the spring moved (fired again at once).
    let fired = |f: usize| {
        let (now, then) = (spring(f), spring(f - 1));
        now.timer == 1 && (then.timer != 1 || (now.x, now.y) != (then.x, then.y))
    };
    let fires: Vec<usize> = (1..states.len()).filter(|&f| fired(f)).collect();
    assert!(fires.len() >= 6, "{fires:?}");
    // A fire while the timer still ran.
    assert!(fires.iter().any(|&f| spring(f - 1).timer >= 1), "{fires:?}");
    // The cycle: consecutive landings about 85 frames apart, from 80 to 90.
    let gaps: Vec<usize> = fires.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        gaps.iter().filter(|g| (80..=90).contains(*g)).count() >= 3,
        "{gaps:?}"
    );
    // Before and after the trip down the shaft, where the springs are out of view.
    let springs = (223, 56);
    let (first, last) = (fires[0], *fires.last().unwrap());
    let away = (first..last).find(|&f| !in_view(&states[f], springs));
    assert!(away.is_some(), "the springs never left the view");
    assert_eq!(spring(first).x, 0xdf1);
    assert_eq!(spring(last).x, 0xdfa);
}
