//! T23: game over through the real app (DESIGN §2.4). The app quits on the Mac's Esc, so the owner
//! reaches the Amiga's Esc (give up) only by binding it to another key; the test does that in
//! `settings.toml`, as Game ▸ Controls… saves it, then gives up by key, waits out the game over screen
//! and lets go of fire. The app's state hash equals the headless game's after every step, the screen
//! is the headless game's picture, and the level starts again with lives full and score zero. At the
//! default scale and at scale 1, with a `shot` of the window on the screen.

mod common;

use fd_core::Scene;
use fd_e2e::{App, LaunchOpts};

/// Give up at step 5 (E bound to the Amiga's Esc, tapped), the screen from step 32 (`main_loop`'s
/// check after pass 32), fire on Left Option held from step 70 to 72, then right held.
const SCRIPT: &str =
    "5: press KeyE\n50: shot\n70: down AltLeft\n72: up AltLeft\n80: down ArrowRight\n";
const FRAMES: u64 = 140;

fn run(scale: Option<u32>) {
    let mut app = App::launch(LaunchOpts {
        max_frames: FRAMES,
        scale,
        ..Default::default()
    });
    std::fs::write(
        app.home.join("settings.toml"),
        "[keys]\n\"key Esc\" = [\"KeyE\"]\n",
    )
    .unwrap();
    let c = app.run_script(SCRIPT);
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: game over\n"), "{}", c.stderr);
    assert!(
        c.stderr.contains(
            "fd-app: the high scores are skipped; W1-1 restarts, lives full, score zero\n"
        ),
        "{}",
        c.stderr
    );

    let fed = common::inputs(&c);
    let hashes = common::hashes(&c);
    let mut g = common::headless();
    let (mut over, mut restarted) = (None, None);
    for n in 1..=FRAMES {
        g.step(
            fed.get(&n)
                .copied()
                .unwrap_or(amiga_input::FrameInput::NONE),
        );
        for e in g.take_events() {
            match e {
                fd_core::Event::GameOver => over = Some(n),
                fd_core::Event::Restart => restarted = Some(n),
                _ => {}
            }
        }
        assert_eq!(
            hashes[n as usize],
            common::hash(&g),
            "the state after step {n}"
        );
        if n == 50 {
            assert!(matches!(g.scene(), Scene::GameOver(_)));
            assert_eq!(c.read_frame(n), common::picture(&g), "the game over screen");
        }
    }
    // The Amiga's Esc reached the game through E, tapped in step 5 and let go in 6; the check after
    // pass 32 (step 31) shows the screen in step 32; fire let go in step 72 ends it there.
    assert_eq!((over, restarted), (Some(32), Some(72)), "{}", c.stderr);
    assert_eq!(*g.scene(), Scene::Level);
    // 68 passes since the restart: the countdown has lost one count of 999 (63 passes a count).
    assert_eq!((g.lives(), g.state().score, g.energy()), (32, 0, 998));
    assert_ne!(
        g.state().char_x,
        common::headless().state().char_x,
        "played on"
    );
    assert_eq!(c.read_frame(FRAMES), common::picture(&g), "the last frame");
    // The window shows the screen, not black.
    let (img, _) = c.read_shot(50);
    assert!(
        img.rgba.chunks(4).any(|p| p[..3] != [0, 0, 0]),
        "the window at the game over screen is black"
    );
}

#[test]
fn giving_up_in_the_app_shows_game_over_and_restarts_the_level_at_the_default_scale() {
    run(None);
}

#[test]
fn giving_up_in_the_app_shows_game_over_and_restarts_the_level_at_scale_1() {
    run(Some(1));
}
