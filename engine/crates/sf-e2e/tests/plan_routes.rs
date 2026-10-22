//! smart-planner T12, DESIGN §4.1 set C: a route `sf-plan --app` planned, played in the real app.
//! The script layer starts the level as Game ▸ Start Level… does (`start-level W1-1`, the power-on
//! seed the route was planned with) and plays the route's joystick as key presses; the app's state
//! is the engine's on every frame from the app start to the exit, which the engine reaches with no
//! life lost. The engine alone cannot see a difference in how the app starts or feeds a script (a
//! frame offset, the seed), which is how bug-recorder's routes failed.
//!
//! The app runs at the PAL rate, so W1-1's ~2 900 frames take about 3 min with the debug build: ignored in the test
//! block (`sf-plan`'s `tests/app.rs` replays the engine's half there).
//! `cargo test -q -p sf-e2e --test plan_routes -- --ignored`.

use std::path::Path;

use sf_bench::verify::{app_parts, app_script, engine_app};
use sf_conform::InputScript;
use sf_e2e::{App, LaunchOpts};

const ROUTE: &str = "tests/data/w1-1-app-seed1.txt";

#[test]
#[ignore = "slow: about 3 min (the app at the PAL rate, debug build); cargo test -q -p sf-e2e --test plan_routes -- --ignored"]
fn an_app_route_reaches_w1_1s_exit_in_the_real_app_with_no_life_lost() {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(ROUTE)).unwrap();
    assert!(text.starts_with("# timing: app\n"), "an app-timed route");
    let seed: u32 = text.lines().nth(1).and_then(|l| l.strip_prefix("# seed: ")).unwrap().parse().unwrap();
    let route = InputScript::parse(&text).unwrap();
    let level = "W1-1".parse().unwrap();
    let frames = route.lines.last().unwrap().0 + 25;
    let engine = engine_app(level, seed, &route, frames);
    let exit = engine.exit.expect("the engine reaches the exit from the app start");
    assert_eq!(engine.lives_lost, 0);

    let mut app = App::launch(LaunchOpts {
        max_frames: exit as u64 + 1,
        timeout_s: 240,
        scale: Some(1),
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let c = app.run_script_unread(&app_script(level, seed, &route, exit));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: started W1-1 after step 0"), "{}", c.stderr);
    let hashes = std::fs::read_to_string(c.dir.join("hashes.txt")).unwrap();
    assert_eq!(app_parts(&hashes, &engine.states), None, "the app keeps the engine's state to the exit, frame {exit}");
}
