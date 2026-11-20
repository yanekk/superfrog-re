//! T28, smoke: the first stretch of the one route through W1-1 (T84, `fd-plan`'s
//! `tests/routes/W1-1.txt`) played in the real `fd-app` as key presses on the default keys, the
//! same path `finish_level.rs` takes to the level's end. The keys reach the game as the route's
//! joystick, step for step, the app's state hash equals the headless game's after every step, and
//! the window shows the headless game's picture at the stretch's end. The stretch runs unpaced
//! (`fast-until`), its last steps on the clock so the window draws; `--capture-hashes` leaves
//! the frames' pictures out.

mod common;

use amiga_input::FrameInput;
use fd_e2e::{App, LaunchOpts};

/// The route's first steps: past its first jumps, turns and the first drop (`fd-conform`'s
/// `full_route.rs` checks the same 600 against the original).
const STEPS: u64 = 600;
/// Steps on the clock at the end, so the window presents frames and the shot has one to show.
const PACED: u64 = 25;
/// The shot's step: before the last, which ends the run before a shot is taken.
const SHOT: u64 = STEPS - 5;

#[test]
fn the_routes_first_stretch_plays_in_the_app_as_the_headless_game_does_every_frame() {
    let route = common::route();
    let stretch = &route[..STEPS as usize];
    let mut app = App::launch(LaunchOpts {
        max_frames: STEPS,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let script = format!(
        "fast-until {}\n{SHOT}: shot\n{}",
        STEPS - PACED,
        common::key_lines(stretch)
    );
    let c = app.run_script_unread(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        !c.dir.join("frame-00000.png").exists(),
        "--capture-hashes writes no frames"
    );

    let fed = common::inputs(&c);
    let hashes = common::hashes(&c);
    assert_eq!(
        hashes.len() as u64,
        STEPS + 1,
        "a hash for the start and every step"
    );
    let mut g = common::headless();
    let (w, h) = (common::picture(&g).width, common::picture(&g).height);
    let (frame, window) = common::shown(&c, SHOT, w, h);
    assert!(
        frame > STEPS - PACED,
        "the shot shows a paced frame, {frame}"
    );
    assert_eq!(hashes[0], common::hash(&g), "the start");
    for n in 1..=STEPS {
        let i = fed.get(&n).copied().unwrap_or(FrameInput::NONE);
        assert_eq!(
            i,
            stretch[n as usize - 1],
            "the route's input reached step {n}"
        );
        g.step(i);
        g.take_events();
        assert_eq!(
            hashes[n as usize],
            common::hash(&g),
            "the state after step {n}"
        );
        if n == frame {
            assert_eq!(window, common::picture(&g), "the window at step {n}");
        }
    }
    assert_eq!(g.lives(), 32, "no life lost");
}
