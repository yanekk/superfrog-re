//! T28, the plan's final proof (DESIGN §1, §4): the one route through W1-1 (T84, `fd-plan`'s
//! `tests/routes/W1-1.txt`, which the engine alone and the original both finish with no life lost)
//! played in the real `fd-app` from launch, as key presses on the default keys through the
//! script layer's real input path, to "Level 1 complete". The keys reach the game as the route's
//! joystick on every step; the app's state hash equals the headless game's after every step; the
//! level ends in the route's last step with nothing else on the way (no life lost, no unported
//! object); the window shows the message as the headless game draws it; a fresh fire press
//! restarts W1-1, lives full and score zero. The app is never the front app while it runs: a
//! scripted run takes no focus (`fd-app`'s `unattended`).
//!
//! The route runs unpaced (`fast-until`), the app's own path for a scripted run; the end on the
//! clock, so the window presents and the shot has a frame. `--capture-hashes` leaves the frames'
//! pictures out: 22 000 PNGs are 1.7 GB and most of the run. `finish_short.rs` is its smoke part.

mod common;

use amiga_input::FrameInput;
use fd_core::{Event, Scene};
use fd_e2e::{App, LaunchOpts};

/// The route's steps; `main_loop` finds the level's end in the last (`fd-plan`'s `full_route.rs`).
const END: u64 = 22464;
/// Steps on the clock before the end.
const PACED: u64 = 50;
/// The shot of the message, and the fresh fire press that restarts.
const SHOT: u64 = END + 20;
const FIRE: u64 = END + 40;
const FRAMES: u64 = END + 60;

/// The front app's process, as `lsappinfo` (macOS) names it; `None` if it names none.
fn front_pid() -> Option<u32> {
    let asn = std::process::Command::new("lsappinfo")
        .arg("front")
        .output()
        .ok()?;
    let asn = String::from_utf8_lossy(&asn.stdout).trim().to_string();
    let info = std::process::Command::new("lsappinfo")
        .args(["info", "-only", "pid", &asn])
        .output()
        .ok()?;
    // `"pid"=896`
    String::from_utf8_lossy(&info.stdout)
        .trim()
        .rsplit('=')
        .next()?
        .parse()
        .ok()
}

#[test]
#[ignore = "full: the whole of W1-1 in the app, about 1 min (debug); cargo test -q -p fd-e2e --test finish_level -- --ignored"]
fn the_route_finishes_w1_1_in_the_app_as_the_headless_game_does_every_frame() {
    let route = common::route();
    assert_eq!(route.len() as u64, END, "the one route");
    let mut app = App::launch(LaunchOpts {
        max_frames: FRAMES,
        timeout_s: 600,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let script = format!(
        "fast-until {}\n{SHOT}: shot\n{FIRE}: press AltLeft\n{}",
        END - PACED,
        common::key_lines(&route)
    );
    app.start_script(&script);
    let pid = app.pid().expect("running");
    // The front app, read every half second while the run lasts: never the app.
    let mut fronts = 0;
    while fd_e2e::group_alive(pid) {
        if let Some(front) = front_pid() {
            fronts += 1;
            assert_ne!(front, pid, "the scripted app took the focus");
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let c = app.finish_unread();
    assert!(fronts > 0, "the front app was never read");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: playing W1-1\n"), "{}", c.stderr);
    let said: Vec<&str> = c
        .stderr
        .lines()
        .filter(|l| l.starts_with("fd-app: ") && !l.starts_with("fd-app: playing"))
        .collect();
    assert_eq!(
        said,
        [
            "fd-app: level 1 complete",
            "fd-app: fire on the message; W1-1 restarts, lives full, score zero",
            &format!("fd-app: quit after step {FRAMES}"),
        ],
        "{}",
        c.stderr
    );

    let fed = common::inputs(&c);
    let hashes = common::hashes(&c);
    assert_eq!(
        hashes.len() as u64,
        FRAMES + 1,
        "a hash for the start and every step"
    );
    let mut g = common::headless();
    let (w, h) = (common::picture(&g).width, common::picture(&g).height);
    let (frame, window) = common::shown(&c, SHOT, w, h);
    assert!(frame > END, "the shot shows the message, frame {frame}");
    assert_eq!(hashes[0], common::hash(&g), "the start");
    let mut events = Vec::new();
    for n in 1..=FRAMES {
        let i = fed.get(&n).copied().unwrap_or(FrameInput::NONE);
        let want = match n {
            ..=END => route[n as usize - 1],
            FIRE => FrameInput::joy(amiga_input::Joy::FIRE),
            _ => FrameInput::NONE,
        };
        assert_eq!(i, want, "the input step {n} saw");
        g.step(i);
        events.extend(g.take_events().into_iter().map(|e| (n, e)));
        assert_eq!(
            hashes[n as usize],
            common::hash(&g),
            "the state after step {n}"
        );
        if n == END - 1 {
            assert_eq!(g.lives(), 32, "no life lost on the way");
        }
        if n == frame {
            assert!(matches!(g.scene(), Scene::Complete(_)), "{:?}", g.scene());
            assert_eq!(window, common::picture(&g), "the window shows the message");
        }
    }
    assert_eq!(events, [(END, Event::LevelEnd), (FIRE, Event::Restart)]);
    assert_eq!(*g.scene(), Scene::Level);
    assert_eq!(
        (g.lives(), g.state().score, g.state().world_index),
        (32, 0, 0)
    );
}
