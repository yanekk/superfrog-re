//! T16, the app drill's sizes (DESIGN §2.7, §2.8): the picture whole and centred at the default
//! scale, at scale 1 and in a resized window, and the recording bar adding its height under it and
//! taking it away. Through the script layer's `resize` and `shot`.
//!
//! In the full set, not smoke: a window's new size lands only when the app turns to the window's
//! events, and a debug build writing every frame's capture steps slower than the game's 50 Hz, so
//! the pacer's catch-up (`amiga_app::pacing`, uncapped) can hold those events for seconds while
//! ten apps start at once. Each check therefore waits seconds of steps for its size.


use std::path::PathBuf;

use fd_e2e::{App, Captured, LaunchOpts, Shot};

fn run(max_frames: u64, scale: Option<u32>, script: &str) -> (App, PathBuf, Captured) {
    let mut app = App::launch(LaunchOpts {
        max_frames,
        timeout_s: 180,
        scale,
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    let c = app.run_script_unread(script);
    assert!(c.exit.success(), "{}", c.stderr);
    (app, bugs, c)
}

/// The window's size in points, as `resize` asks for it.
fn points(s: &Shot) -> (u32, u32) {
    let p = |v: u32| (v as f64 / s.scale_factor).round() as u32;
    (p(s.surface.0), p(s.surface.1))
}

/// Each `shot`'s picture: a whole multiple of the game's, inside the window above the bar, and
/// centred there.
fn assert_whole_and_centred(c: &Captured, n: u64, bar: u32) -> Shot {
    let (_, shot) = c.read_shot(n);
    let pic = c.read_frame(n);
    let (x, y, w, h) = shot.viewport;
    let (sw, sh) = (shot.surface.0, shot.surface.1 - bar);
    assert!(
        w % pic.width == 0 && h % pic.height == 0 && w / pic.width == h / pic.height,
        "step {n}: {shot:?} is no whole multiple of {}×{}",
        pic.width,
        pic.height
    );
    assert!(x + w <= sw && y + h <= sh, "step {n}: {shot:?} is cut off");
    assert!(
        (sw - w) / 2 == x && (sh - h) / 2 == y,
        "step {n}: {shot:?} is not centred"
    );
    shot
}

/// `N: shot` lines from `from` to `to`, every 10 steps.
fn shots(from: u64, to: u64) -> String {
    (from..=to).step_by(10).map(|n| format!("{n}: shot\n")).collect()
}

/// A window's new size lands once the app has turned to the window's events, which under load (a
/// test set starting ten apps at once) can be many steps after it was asked for: every shot is
/// whole and centred, and the last of `from..=to` has the size.
fn resized_by(c: &Captured, from: u64, to: u64, bar: u32, want: (u32, u32)) -> Shot {
    for n in (from..=to).step_by(10) {
        assert_whole_and_centred(c, n, bar);
    }
    let (_, last) = c.read_shot(to);
    assert_eq!(points(&last), want, "not resized by step {to}: {last:?}");
    last
}

#[test]
fn the_picture_is_whole_and_centred_at_the_default_scale_at_scale_1_and_resized() {
    // The default scale is twice; a bigger window keeps the largest whole multiple that fits.
    let (_app, _, c) = run(610, None, &format!("10: shot\n20: resize 1000 700\n{}", shots(30, 600)));
    let s = assert_whole_and_centred(&c, 10, 0);
    let pic = c.read_frame(10);
    assert_eq!(points(&s), (pic.width * 2, pic.height * 2), "{s:?}");
    let s = resized_by(&c, 30, 600, 0, (1000, 700));
    assert_eq!(s.viewport.2 * 1000, s.surface.0 * 2 * pic.width, "twice, not three times: {s:?}");

    let (_app, _, c) = run(610, Some(1), &format!("10: shot\n20: resize 500 500\n{}", shots(30, 600)));
    let s = assert_whole_and_centred(&c, 10, 0);
    assert_eq!(points(&s), (pic.width, pic.height), "{s:?}");
    let s = resized_by(&c, 30, 600, 0, (500, 500));
    assert_eq!(s.viewport.2 * 500, s.surface.0 * pic.width, "scale 1 stays 1× in a bigger window: {s:?}");
}

#[test]
fn the_recording_bar_adds_its_height_under_the_picture_at_every_size_and_goes_with_it() {
    // The shrink lands as late as the resize: under load, measured 840 steps after the stop.
    let (_app, _, c) = run(
        2010,
        None,
        &format!(
            "10: menu start-level\nform start\n60: shot\n70: resize 700 600\n{}\
             610: menu stop-recording\nnote discard\n2000: shot\n",
            shots(80, 600)
        ),
    );
    let pic = c.read_frame(60);
    let (_, s) = c.read_shot(60);
    assert_eq!(points(&s).0, pic.width * 2, "{s:?}");
    // The bar, in the surface's pixels: what the window grew by.
    let bar = s.surface.1 - s.viewport.3;
    let bar_points = points(&s).1 - pic.height * 2;
    assert!(bar_points > 20 && bar_points < 60, "a bar of {bar_points} points: {s:?}");
    assert_eq!(assert_whole_and_centred(&c, 60, bar).viewport.1, 0, "the picture is at the top");
    let before = resized_by(&c, 80, 600, bar, (700, 600));
    let s = assert_whole_and_centred(&c, 2000, 0);
    assert_eq!(
        s.surface,
        (before.surface.0, before.surface.1 - bar),
        "the bar's height went with it"
    );
}
