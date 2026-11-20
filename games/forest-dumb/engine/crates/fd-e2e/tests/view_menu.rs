//! fd-view-menu: View ▸ Scale, Fullscreen and Filter in the real app. Each choice applies at once
//! and is saved in `settings.toml`, which the next launch starts from; a scripted run goes
//! fullscreen only with `--allow-fullscreen`.
//!
//! Two layers, both full (owner, 2026-11-20). The script layer: `menu scale-2`, `menu filter-geom`, `shot`. The OS
//! layer clicks the real menu bar through the Accessibility API, the one way to read its titles and
//! checks, and presses ⌃⌘F through CGEvent:
//! `FD_E2E_OS=1 cargo test --release -p fd-e2e --test view_menu -- --ignored --test-threads 1`
//! (release, as `drill_os.rs`; the screen unlocked, refused up front).

use std::time::{Duration, Instant};

use fd_e2e::{App, Image, Key, LaunchOpts, Mods, Shot};

/// The settings file the run left in its home.
fn settings(app: &App) -> String {
    std::fs::read_to_string(app.home.join("settings.toml")).unwrap_or_default()
}

/// The shot's scale (surface pixels a game pixel) and how many of its pixels are not the plain
/// integer scaling of `frame`, centred with black around it: none without a filter.
fn unlike_integer_scaling(shot: &Image, info: &Shot, frame: &Image) -> (u32, u32) {
    assert_eq!((shot.width, shot.height), info.surface, "the shot is the whole surface");
    let (x, y, w, h) = info.viewport;
    let s = w / frame.width;
    assert!(s >= 1, "{info:?}");
    assert_eq!((w, h), (frame.width * s, frame.height * s), "a whole multiple: {info:?}");
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
    (s, wrong)
}

/// The window at `n`× the frame, in the shot's pixels.
fn at_scale(info: &Shot, frame: &Image, n: u32) -> bool {
    let px = |v: u32| (v as f64 * info.scale_factor).round() as u32;
    info.surface == (px(frame.width * n), px(frame.height * n))
}

#[test]
fn scale_and_filter_from_a_script_apply_at_once_and_a_relaunch_starts_with_them() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 112,
        scale: Some(1),
        ..Default::default()
    });
    // A resize lands in a few frames on a quiet machine and in seconds under load: the shot waits
    // about 2 s for it (as Superfrog's scale test).
    let c = app.run_script("5: menu scale-2\n100: shot\n105: menu filter-geom\n110: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let (img, info) = c.read_shot(100);
    let frame = c.read_frame(info.frame);
    let sf = info.scale_factor.round() as u32;
    assert!(at_scale(&info, &frame, 2), "the window at 2× after menu scale-2: {info:?}");
    assert_eq!(unlike_integer_scaling(&img, &info, &frame), (2 * sf, 0), "plain integer scaling at 2×");
    let (img, info) = c.read_shot(110);
    let frame = c.read_frame(info.frame);
    let (s, wrong) = unlike_integer_scaling(&img, &info, &frame);
    assert_eq!(s, 2 * sf, "{info:?}");
    assert!(wrong > info.surface.0 * info.surface.1 / 10, "Geom shows: only {wrong} pixels differ");
    let saved = settings(&app);
    assert!(saved.contains("scale = 2") && saved.contains("filter = \"geom\""), "{saved}");
    assert!(saved.contains("fullscreen = false") && saved.contains("widescreen = false"), "{saved}");

    // The next launch, with no --scale, opens at the saved 2× and draws through Geom.
    app.opts.scale = None;
    app.opts.max_frames = 12;
    let c = app.run_script("10: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let (img, info) = c.read_shot(10);
    let frame = c.read_frame(info.frame);
    assert!(at_scale(&info, &frame, 2), "the relaunch at the saved 2×: {info:?}");
    let (s, wrong) = unlike_integer_scaling(&img, &info, &frame);
    assert_eq!(s, 2 * sf, "{info:?}");
    assert!(wrong > info.surface.0 * info.surface.1 / 10, "the saved Geom shows: only {wrong} pixels differ");
}

#[test]
fn a_scripted_run_stays_windowed_without_allow_fullscreen() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 12,
        ..Default::default()
    });
    let c = app.run_script("5: menu fullscreen\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: menu fullscreen after step 4"), "{}", c.stderr);
    assert!(
        c.stderr.contains("fd-app: fullscreen refused: a scripted run needs --allow-fullscreen"),
        "{}",
        c.stderr
    );
    assert!(!settings(&app).contains("fullscreen = true"), "{}", settings(&app));

    // A saved `fullscreen = true` opens windowed, saying so, and is left as it was.
    std::fs::write(app.home.join("settings.toml"), "fullscreen = true\n").unwrap();
    let c = app.run_script("10: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr.contains("fd-app: staying windowed: a scripted run goes fullscreen only with --allow-fullscreen"),
        "{}",
        c.stderr
    );
    let (_, info) = c.read_shot(10);
    assert!(at_scale(&info, &c.read_frame(info.frame), 1), "windowed at --scale 1: {info:?}");
    assert_eq!(settings(&app), "fullscreen = true\n");
}

fn os_layer() {
    assert!(
        std::env::var_os("FD_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests click the real menu bar; run them with FD_E2E_OS=1"
    );
    fd_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
    assert!(!screen_locked(), "{LOCKED}");
}

const LOCKED: &str = "the screen is locked: unlock it and run again. Locked, the menu bar still \
                      answers but fullscreen never happens (as `drill_os.rs`)";

/// The console session's screen is locked (as `drill_os.rs`).
fn screen_locked() -> bool {
    let out = std::process::Command::new("osascript")
        .args([
            "-l",
            "JavaScript",
            "-e",
            "ObjC.import('CoreGraphics'); \
             const d = ObjC.deepUnwrap(ObjC.castRefToObject($.CGSessionCopyCurrentDictionary())); \
             String(!!(d && d.CGSSessionScreenIsLocked))",
        ])
        .output()
        .expect("running osascript");
    String::from_utf8_lossy(&out.stdout).trim() == "true"
}

fn until(what: &str, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        let locked = if screen_locked() { format!("; {LOCKED}") } else { String::new() };
        assert!(t.elapsed() < Duration::from_secs(20), "{what}: not within 20 s{locked}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Wait until the window's bounds satisfy `f`, at most 5 s (Superfrog's bound: over six times
/// the slowest fullscreen it measured); a failure lists every bounds seen and when.
fn until_window(app: &App, what: &str, mut f: impl FnMut([i32; 4]) -> bool) {
    let t = Instant::now();
    let mut seen: Vec<(u128, [i32; 4])> = Vec::new();
    loop {
        let b = app.os_window().1;
        if seen.last().is_none_or(|s| s.1 != b) {
            seen.push((t.elapsed().as_millis(), b));
        }
        if f(b) {
            return;
        }
        if t.elapsed() >= Duration::from_secs(5) {
            let locked = if screen_locked() { format!("; {LOCKED}") } else { String::new() };
            panic!("{what}: not within 5 s; bounds [x, y, w, h] by ms: {seen:?}{locked}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The titles checked in a View submenu.
fn checked(app: &App, sub: &str) -> Vec<String> {
    app.os_menu_items(&["View", sub]).into_iter().filter(|i| i.checked).map(|i| i.title).collect()
}

fn titles(app: &App, path: &[&str]) -> Vec<String> {
    app.os_menu_items(path).into_iter().map(|i| i.title).collect()
}

/// The app menu is titled with the process name (an unbundled binary), as `drill_os.rs`'s `QUIT`.
const QUIT: [&str; 2] = ["fd-app", "Quit Forest Dumb"];

#[test]
#[ignore = "OS layer, takes over the display: FD_E2E_OS=1 cargo test --release -p fd-e2e --test view_menu -- --ignored --test-threads 1"]
fn os_layer_the_view_menu_checks_follow_the_choices_survive_a_relaunch_and_fullscreen_toggles() {
    os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 3000,
        timeout_s: 120,
        scale: None,
        ..Default::default()
    });
    app.start_os();
    app.os_window();
    assert_eq!(app.os_menu_bar()[1..], ["fd-app", "Game", "View"]);
    // No AppKit "Enter Full Screen": `before_launch` took.
    assert_eq!(titles(&app, &["View"]), ["Scale", "Fullscreen", "Filter"]);
    assert_eq!(titles(&app, &["View", "Scale"]), ["1×", "2×", "3×", "Auto"]);
    assert_eq!(titles(&app, &["View", "Filter"]), ["None", "Basic", "Lottes", "Geom", "Easymode"]);
    assert_eq!(checked(&app, "Scale"), ["Auto"]);
    assert_eq!(checked(&app, "Filter"), ["None"]);
    assert!(!app.os_menu_item(&["View"], "Fullscreen").checked);

    app.os_menu(&["View", "Scale", "2×"]);
    until("2× checked", || checked(&app, "Scale") == ["2×"]);
    app.os_menu(&["View", "Filter", "Geom"]);
    until("Geom checked", || checked(&app, "Filter") == ["Geom"]);
    app.os_menu(&QUIT);
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    let saved = settings(&app);
    assert!(saved.contains("scale = 2") && saved.contains("filter = \"geom\""), "{saved}");

    // Relaunched on the same home: still checked.
    app.start_os();
    app.os_window();
    assert_eq!(checked(&app, "Scale"), ["2×"]);
    assert_eq!(checked(&app, "Filter"), ["Geom"]);

    // Fullscreen through the menu, back by ⌃⌘F; its check follows.
    let windowed = app.os_window().1;
    let full = |b: [i32; 4]| b[2] > windowed[2] + 200 && b[3] > windowed[3] + 200;
    let back = |b: [i32; 4]| b[2] < windowed[2] + 50;
    app.os_menu(&["View", "Fullscreen"]);
    until_window(&app, "fullscreen", full);
    assert!(app.os_menu_item(&["View"], "Fullscreen").checked);
    std::thread::sleep(Duration::from_millis(800));
    app.os_activate();
    app.os_key_with(Mods::CONTROL_COMMAND, Key::Char('f'));
    until_window(&app, "back to the window by ⌃⌘F", back);
    assert!(!app.os_menu_item(&["View"], "Fullscreen").checked);
    std::thread::sleep(Duration::from_millis(800));
    app.os_menu(&QUIT);
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    let toggles = c.stderr.matches("fd-app: menu fullscreen after step").count();
    assert_eq!(toggles, 2, "each toggle once: {}", c.stderr);
    let saved = settings(&app);
    assert!(saved.contains("fullscreen = false") && saved.contains("scale = 2"), "{saved}");
}
