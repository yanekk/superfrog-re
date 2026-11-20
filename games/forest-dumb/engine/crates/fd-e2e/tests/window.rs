//! T13: the window and the ways out (DESIGN §2.7, §2.12, §5.2): the window at the default scale
//! and at scale 1, Esc and Game ▸ Quit end the run cleanly, the watchdog ends a hung one, and a
//! second copy on the same home is refused.

use std::time::{Duration, Instant};

use fd_e2e::{App, LaunchOpts};

/// The step a run quit after, from its log.
fn quit_after(stderr: &str) -> u64 {
    let l = stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: quit after step "))
        .expect(stderr);
    l.parse().unwrap()
}

#[test]
fn the_window_shows_the_whole_picture_at_scale_1_and_twice_by_default() {
    for (scale, n) in [(Some(1), 1), (None, 2)] {
        let mut app = App::launch(LaunchOpts {
            max_frames: 12,
            scale,
            ..Default::default()
        });
        let c = app.run_script("10: shot\n");
        assert!(c.exit.success(), "{}", c.stderr);
        let frame = &c.frames[10];
        let (img, shot) = c.read_shot(10);
        let px = |v: u32| (v as f64 * shot.scale_factor).round() as u32;
        assert_eq!(
            shot.surface,
            (px(frame.width * n), px(frame.height * n)),
            "scale {scale:?}: {shot:?}"
        );
        assert_eq!(
            shot.viewport,
            (0, 0, shot.surface.0, shot.surface.1),
            "the picture fills the window"
        );
        // Every game pixel is a solid square of the frame's colour.
        let s = shot.surface.0 / frame.width;
        for (x, y) in [
            (0, 0),
            (frame.width / 2, frame.height / 2),
            (frame.width - 1, frame.height - 1),
        ] {
            assert_eq!(
                img.rgb(x * s + s / 2, y * s + s / 2),
                frame.rgb(x, y),
                "({x}, {y}) at scale {scale:?}"
            );
        }
    }
}

#[test]
fn esc_quits_cleanly_and_never_reaches_the_game() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 500,
        ..Default::default()
    });
    let c = app.run_script("10: press Escape\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr.contains("fd-app: menu quit after step 9"),
        "{}",
        c.stderr
    );
    assert_eq!(quit_after(&c.stderr), 9);
    assert!(c.inputs.is_empty(), "Esc is not the game's: {:?}", c.inputs);
    assert!(
        app.home.join("capture-0/pacing.txt").is_file(),
        "the capture was finished"
    );
}

#[test]
fn quit_from_the_menu_and_command_q_end_the_run_cleanly() {
    for line in ["7: menu quit\n", "7: down SuperLeft\n7: press KeyQ\n"] {
        let mut app = App::launch(LaunchOpts {
            max_frames: 500,
            ..Default::default()
        });
        let c = app.run_script(line);
        assert!(c.exit.success(), "{line}: {}", c.stderr);
        assert_eq!(quit_after(&c.stderr), 6, "{line}");
        assert!(
            !app.home.join("settings.toml").exists(),
            "quitting saves nothing that did not change"
        );
    }
}

#[test]
fn the_watchdog_ends_a_hung_run_and_its_whole_process_group() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 500,
        timeout_s: 4,
        ..Default::default()
    });
    let start = Instant::now();
    let c = app.run_script("5: hang\n");
    assert!(!c.exit.success(), "{}", c.stderr);
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "{:?}",
        start.elapsed()
    );
    assert!(
        c.stderr.contains("fd-app: the script hangs at frame 5"),
        "{}",
        c.stderr
    );
    assert!(c.stderr.contains("--timeout 4 s reached"), "{}", c.stderr);
}

#[test]
fn a_second_copy_on_the_same_home_is_refused() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 3000,
        timeout_s: 90,
        ..Default::default()
    });
    app.start_script("");
    let pgid = app.pid().unwrap();
    let log = app.home.join("capture-0.stderr");
    let start = Instant::now();
    while !std::fs::read_to_string(&log)
        .unwrap_or_default()
        .contains("playing W1-1")
    {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "the first copy never started"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let c = app.run_script("");
    app.kill();
    assert!(!fd_e2e::group_alive(pgid));
    assert_eq!(c.exit.code(), Some(1), "{}", c.stderr);
    assert!(
        c.stderr.contains("Forest Dumb is already running"),
        "{}",
        c.stderr
    );
    assert!(!c.stderr.contains("playing W1-1"), "{}", c.stderr);
}
