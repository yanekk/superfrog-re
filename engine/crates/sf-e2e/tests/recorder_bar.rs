//! bug-recorder T09: the recording bar under the picture (DESIGN §2.3, D4). The script layer: on
//! `start-level … record` the window is `GAME × n` and the bar's height, the picture drawn in it
//! unchanged in size, scale and content, and after Stop Recording the window is `GAME × n` again,
//! at 1×, 2×, 3× and 2× widescreen. The OS layer (`SF_E2E_OS=1`): the bar shows "Recording" and a
//! time that advances; STOP and ⌘. open the save path; Game ▸ Stop Recording is enabled only while
//! recording; nothing in the bar is clipped or overlapping at any size; and in fullscreen the
//! picture's scale leaves the bar on screen under it.

mod common;

use std::time::{Duration, Instant};

use common::{assert_integer_scaled, not_black};
use sf_e2e::{App, Captured, Image, Key, LaunchOpts, Mods, Shot};

/// The window at 1×, in points: a level's display window (RE T210).
const W: u32 = 334;
const H: u32 = 261;
/// `sf_app::window::BAR_HEIGHT` (the binary's, not linkable from here).
const BAR: u32 = 34;

/// The shot is `frame` at a whole scale, centred in the surface above a bar `bar` pixels high,
/// everything else black (the bar is AppKit's, over the surface, so the surface under it is the
/// black around the picture). Returns the scale in pixels a game pixel.
fn assert_above_bar(shot: &Image, info: &Shot, frame: &Image, bar: u32) -> u32 {
    assert_eq!((shot.width, shot.height), info.surface, "the shot is the whole surface");
    let area = (info.surface.0, info.surface.1 - bar);
    let (x, y, w, h) = info.viewport;
    let s = w / frame.width;
    assert!(s >= 1, "{info:?}");
    assert_eq!((w, h), (frame.width * s, frame.height * s), "a whole multiple: {info:?}");
    assert_eq!((x, y), ((area.0 - w) / 2, (area.1 - h) / 2), "centred above the bar: {info:?}");
    assert!(y + h <= area.1, "the picture reaches under the bar: {info:?}");
    let mut wrong = 0;
    for py in 0..shot.height {
        for px in 0..shot.width {
            let inside = (x..x + w).contains(&px) && (y..y + h).contains(&py);
            let want = if inside { frame.rgb((px - x) / s, (py - y) / s) } else { [0, 0, 0] };
            if shot.rgb(px, py) != want {
                wrong += 1;
            }
        }
    }
    assert_eq!(wrong, 0, "{wrong} pixels of the shot are not the frame at {s}×: {info:?}");
    s
}

/// Steps of `id`'s "after step N" lines in the run's log.
fn after(stderr: &str, what: &str) -> Vec<u64> {
    stderr
        .lines()
        .filter_map(|l| l.strip_prefix(&format!("superfrog: {what} after step ")))
        .filter_map(|n| n.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok())
        .collect()
}

#[test]
fn while_recording_the_window_is_the_bar_s_height_taller_and_the_picture_is_unchanged() {
    // A shot before, during and after the recording: the title, then W1-1 started recording at
    // step 30 and stopped at 101.
    const BEFORE: u64 = 20;
    const DURING: u64 = 90;
    const AFTER: u64 = 150;
    for (n, wide) in [(1u32, false), (2, false), (3, false), (2, true)] {
        let what = format!("{n}× (wide {wide})");
        let mut app = App::launch(LaunchOpts {
            max_frames: AFTER + 5,
            timeout_s: 120,
            scale: Some(n),
            ..Default::default()
        });
        let bugs = app.home.join("test-bugs");
        app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
        if wide {
            std::fs::write(app.home.join("settings.toml"), "widescreen = true\n").unwrap();
        }
        let c = app.run_script(&format!(
            "note x\n{BEFORE}: shot\n30: start-level W1-1 record\n{DURING}: shot\n101: menu stop-recording\n{AFTER}: shot\n"
        ));
        assert!(c.exit.success(), "{what}: {}", c.stderr);
        assert_eq!(after(&c.stderr, "recording bar shown"), [29], "{what}: {}", c.stderr);
        assert_eq!(after(&c.stderr, "recording bar hidden"), [100], "{what}: {}", c.stderr);
        let side = if wide { 65 } else { 0 };
        let gw = W + 2 * side;

        let (_, before) = c.read_shot(BEFORE);
        let sf = before.scale_factor as u32;
        assert_eq!(before.surface, (gw * n * sf, H * n * sf), "{what} before: {before:?}");
        let (img, during) = c.read_shot(DURING);
        assert_eq!(during.surface, (gw * n * sf, (H * n + BAR) * sf), "{what}: the bar's height more: {during:?}");
        let frame = c.read_frame(during.frame);
        assert_eq!((frame.width, frame.height), (gw, H), "{what}: W1-1's picture");
        assert!(not_black(&frame), "{what}: frame {} is black: a weak check", during.frame);
        assert_eq!(assert_above_bar(&img, &during, &frame, BAR * sf), n * sf, "{what}: the scale changed");
        assert_eq!(&during.viewport, &(0, 0, gw * n * sf, H * n * sf), "{what}: the picture moved");

        let (img, back) = c.read_shot(AFTER);
        assert_eq!(back.surface, before.surface, "{what}: back to GAME × {n} after STOP: {back:?}");
        let frame = c.read_frame(back.frame);
        assert_eq!(assert_integer_scaled(&img, &back, &frame), n * sf, "{what} after");
        // The recording was saved, as STOP does.
        assert!(c.prompts.contains("save-recording: bug-W1-1-"), "{what}: {}", c.prompts);
    }
}

/// A recording that is not started leaves the window alone, and a Start Level without the
/// recorder shows no bar.
#[test]
fn without_the_recorder_there_is_no_bar() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 95,
        timeout_s: 90,
        scale: Some(2),
        ..Default::default()
    });
    let c = app.run_script("30: start-level W1-1\n90: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(!c.stderr.contains("recording bar"), "{}", c.stderr);
    let (img, info) = c.read_shot(90);
    let sf = info.scale_factor as u32;
    assert_eq!(info.surface, (W * 2 * sf, H * 2 * sf));
    assert_integer_scaled(&img, &info, &c.read_frame(info.frame));
}

// The OS layer: the real bar, read and clicked through the Accessibility API, its window captured.

fn os_layer() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests read and click the real window; run them with SF_E2E_OS=1"
    );
    sf_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
}

/// Wait until `f` holds, at most 30 s (a start runs the game unseen for seconds).
fn until(what: &str, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(30), "{what}: not within 30 s");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The running app's log so far.
fn log(app: &App) -> String {
    std::fs::read_to_string(app.home.join("capture-0.stderr")).unwrap_or_default()
}

/// The bar's views lie inside the window, in a strip along its bottom, in order left to right,
/// none overlapping the next and none cut by the window's edge: no text is clipped. Returns the
/// views' frames, `[dot, label, time, hint, stop]`.
fn assert_bar_fits(app: &App, what: &str) -> Vec<[f64; 4]> {
    let win = game_window(app);
    let [wx, wy, ww, wh] = win.map(|v| v as f64);
    let ids = ["recording-dot", "recording-label", "recording-time", "recording-hint", "recording-stop"];
    let frames: Vec<[f64; 4]> = ids.iter().map(|id| app.os_frame_id(id)).collect();
    let strip = app.os_frame_id("recording-bar");
    // The strip: the window's width, BAR points high, at its bottom (the window server's bounds
    // may be scaled on a second display, FINDINGS RE T23: within a point or two).
    assert!((strip[2] - ww).abs() <= 2.0, "{what}: strip {strip:?} in window {win:?}");
    assert!((strip[1] + strip[3] - (wy + wh)).abs() <= 2.0, "{what}: strip {strip:?} not at the bottom of {win:?}");
    assert!((strip[3] - BAR as f64).abs() <= 2.0, "{what}: strip {strip:?}");
    for (i, f) in frames.iter().enumerate() {
        let [x, y, w, h] = *f;
        assert!(w > 0.0 && h > 0.0, "{what}: {} has no size: {f:?}", ids[i]);
        assert!(x >= wx && x + w <= wx + ww, "{what}: {} {f:?} is cut by the window {win:?}", ids[i]);
        assert!(y >= strip[1] - 0.5 && y + h <= strip[1] + strip[3] + 0.5, "{what}: {} {f:?} leaves the strip {strip:?}", ids[i]);
    }
    for i in 0..ids.len() - 1 {
        assert!(
            frames[i][0] + frames[i][2] <= frames[i + 1][0],
            "{what}: {} {:?} overlaps {} {:?}",
            ids[i],
            frames[i],
            ids[i + 1],
            frames[i + 1]
        );
    }
    frames
}

/// The game window's bounds: the app's largest window (fullscreen, macOS keeps a thin window of
/// the app's along the top of the screen as well).
fn game_window(app: &App) -> [i32; 4] {
    app.os_window();
    let all = app.os_windows();
    all.iter().map(|w| w.1).max_by_key(|b| b[2] as i64 * b[3] as i64).expect("a window")
}

/// "mm:ss" as seconds.
fn seconds(t: &str) -> u64 {
    let v: Vec<u64> = t.split(':').map(|p| p.parse().unwrap_or_else(|_| panic!("time {t:?}"))).collect();
    v.iter().fold(0, |a, p| a * 60 + p)
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_bar -- --ignored --test-threads 1"]
fn os_layer_the_bar_shows_recording_and_its_time_and_stop_and_cmd_period_stop() {
    os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 6000,
        timeout_s: 180,
        scale: Some(2),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
    // Two recordings: the first stopped by clicking STOP, the second by ⌘.
    app.start_script("note by-click\nnote by-key\n150: start-level W1-1 record\n1500: start-level W1-2 record\n");
    // The window as it settles at 2×, before the recording starts at about 3 s.
    app.os_window();
    std::thread::sleep(Duration::from_millis(1500));
    let (_, plain) = app.os_window();
    assert!(!log(&app).contains("start-level"), "the window was read late: {}", log(&app));
    assert!(!app.os_menu_item(&["Game"], "Stop Recording").enabled, "disabled before any recording");
    until("the bar", || log(&app).contains("recording bar shown"));
    until("the window the bar's height taller", || {
        let (_, b) = app.os_window();
        b[3] - plain[3] >= BAR as i32 - 4
    });
    let (_, taller) = app.os_window();
    assert_eq!(taller[2], plain[2], "the width stays");
    assert!(app.os_menu_item(&["Game"], "Stop Recording").enabled, "enabled while recording");
    assert_eq!(app.os_text_id("recording-label"), "Recording");
    assert_eq!(app.os_text_id("recording-stop"), "STOP");
    assert_eq!(app.os_text_id("recording-hint"), "⌘.");
    let t0 = seconds(&app.os_text_id("recording-time"));
    std::thread::sleep(Duration::from_millis(2500));
    let t1 = seconds(&app.os_text_id("recording-time"));
    assert!(t1 >= t0 + 2, "the time advances: {t0} then {t1}");
    assert_bar_fits(&app, "2×");
    // What the screen shows, for the drill's eyes: kept in the home under SF_E2E_KEEP=1.
    let shot = app.os_capture();
    shot_bar_is_dark(&shot, "2×");

    app.os_press_id("recording-stop");
    until("the bar gone", || log(&app).contains("recording bar hidden"));
    until("the window back", || app.os_window().1[3] == plain[3]);
    assert!(!app.os_shows_id("recording-bar"), "hidden from the accessibility tree too");
    until("Stop Recording disabled", || !app.os_menu_item(&["Game"], "Stop Recording").enabled);
    let text = log(&app);
    assert!(text.contains("superfrog: STOP clicked"), "{text}");
    assert!(text.contains("recording saved to"), "{text}");

    until("the second recording's bar", || log(&app).matches("recording bar shown").count() == 2);
    std::thread::sleep(Duration::from_millis(500));
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('.'));
    until("the bar gone by ⌘.", || log(&app).matches("recording bar hidden").count() == 2);
    until("the window back again", || app.os_window().1[3] == plain[3]);
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert!(c.prompts.contains("-> note by-click"), "{}", c.prompts);
    assert!(c.prompts.contains("save-recording: bug-W1-2-"), "{}", c.prompts);
    assert!(c.prompts.contains("-> note by-key"), "{}", c.prompts);
    let zips = std::fs::read_dir(&bugs).unwrap().count();
    assert_eq!(zips, 2, "{}", c.prompts);
}

/// The strip's colour, the prototype's #2b2b2b, along the bottom of a window capture.
fn shot_bar_is_dark(img: &Image, what: &str) {
    let y = img.height - 4;
    let grey = (0..img.width).filter(|&x| {
        let [r, g, b] = img.rgb(x, y);
        (r as i32 - 0x2b).abs() <= 8 && (g as i32 - 0x2b).abs() <= 8 && (b as i32 - 0x2b).abs() <= 8
    });
    assert!(grey.count() as u32 > img.width / 3, "{what}: the bar's strip is not along the bottom");
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_bar -- --ignored --test-threads 1"]
fn os_layer_nothing_in_the_bar_is_clipped_at_any_size() {
    os_layer();
    for (n, wide) in [(1u32, false), (2, false), (3, false), (2, true)] {
        let what = format!("{n}× (wide {wide})");
        let mut app = App::launch(LaunchOpts {
            max_frames: 1500,
            timeout_s: 90,
            scale: Some(n),
            ..Default::default()
        });
        if wide {
            std::fs::write(app.home.join("settings.toml"), "widescreen = true\n").unwrap();
        }
        app.start_script("30: start-level W1-1 record\n");
        until(&what, || log(&app).contains("recording bar shown"));
        until(&format!("{what}: the window's height"), || {
            let (_, b) = app.os_window();
            app.os_frame_id("recording-bar")[1] + BAR as f64 - (b[1] + b[3]) as f64 <= 2.0
        });
        let f = assert_bar_fits(&app, &what);
        let shot = app.os_capture();
        shot_bar_is_dark(&shot, &what);
        if let Some(dir) = std::env::var_os("SF_E2E_BAR_SHOTS") {
            let path = std::path::Path::new(&dir).join(format!("bar-{n}x{}.png", if wide { "-wide" } else { "" }));
            std::fs::copy(app.home.join(format!("window-{}.png", app.os_window().0)), path).unwrap();
        }
        eprintln!("{what}: bar views {f:?}");
        app.kill();
    }
}

#[test]
#[ignore = "OS layer, takes over the display: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_bar -- --ignored --test-threads 1"]
fn os_layer_in_fullscreen_the_picture_s_scale_leaves_the_bar_on_screen() {
    os_layer();
    // A fullscreen run is at most 300 frames (RE DESIGN §5.2), so two runs. Each records from
    // step 1 and goes fullscreen by ⌃⌘F at 40.
    let keys = |n: u64| {
        format!("{n}: down ControlLeft\n{n}: down SuperLeft\n{n}: press KeyF\n{n}: up SuperLeft\n{n}: up ControlLeft\n")
    };
    let launch = || {
        let mut app = App::launch(LaunchOpts {
            max_frames: 300,
            timeout_s: 90,
            extra_args: vec!["--allow-fullscreen".into()],
            ..Default::default()
        });
        let bugs = app.home.join("test-bugs");
        app.opts.extra_args.extend(["--bugs-dir".into(), bugs.display().to_string()]);
        app
    };

    // One: a shot at 150 with the bar; the bar's views read off the screen before STOP at 296.
    let mut app = launch();
    app.start_script(&format!("note full\n1: start-level W1-1 record\n{}150: shot\n296: menu stop-recording\n", keys(40)));
    until("the shot at 150", || app.home.join("capture-0/window-00150.png").exists());
    let f = assert_bar_fits(&app, "fullscreen");
    let screen = game_window(&app);
    let c: Captured = app.finish();
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("menu fullscreen after step 39"), "{}", c.stderr);
    assert!(c.prompts.contains("-> note full"), "STOP saved it: {}", c.prompts);
    let (img, full) = c.read_shot(150);
    let sf = full.scale_factor as u32;
    assert!(full.surface.0 > W * sf * 2, "fullscreen: {full:?}");
    let frame = c.read_frame(full.frame);
    let area = (full.surface.0, full.surface.1 - BAR * sf);
    let fit = (area.0 / frame.width).min(area.1 / frame.height);
    assert_eq!(assert_above_bar(&img, &full, &frame, BAR * sf), fit, "the largest whole scale above the bar");
    // The bar's views are under the picture, in points down from the top of the screen.
    let picture_bottom = (full.viewport.1 + full.viewport.3) as f64 / full.scale_factor;
    assert!(
        f.iter().all(|v| v[1] - screen[1] as f64 >= picture_bottom - 1.0),
        "{f:?} overlaps the picture ending {picture_bottom} points down {screen:?}"
    );

    // Two: STOP at 120 in fullscreen, a shot at 160 (the whole height again), fullscreen off at
    // 180, a shot at 290: the 1× window without the bar it had when fullscreen began.
    let mut app = launch();
    let c = app.run_script(&format!(
        "note full\n1: start-level W1-1 record\n{}121: menu stop-recording\n160: shot\n{}290: shot\n",
        keys(40),
        keys(180)
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    let (img, nobar) = c.read_shot(160);
    assert!(nobar.surface.0 > W * nobar.scale_factor as u32 * 2, "still fullscreen: {nobar:?}");
    assert_integer_scaled(&img, &nobar, &c.read_frame(nobar.frame));
    let (img, back) = c.read_shot(290);
    let sf = back.scale_factor as u32;
    assert_eq!(back.surface, (W * sf, H * sf), "{back:?}");
    assert_integer_scaled(&img, &back, &c.read_frame(back.frame));
}
