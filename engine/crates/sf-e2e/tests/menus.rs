//! T24: the native menu bar (DESIGN §2.7, §7). Every command is driven twice: from a script
//! (`N: menu …`, in the test block), and by clicking its item in the real menu bar through the
//! Accessibility API, or pressing its shortcut through CGEvent (the OS layer, `SF_E2E_OS=1`). Both
//! reach the app as the same `Command`, and each is checked by its effect: the frames against the
//! engine run headless, the window's size, the files in the home, the app's exit.
//!
//! The app logs each command as `superfrog: menu <id> after step N`, which lines an OS-layer
//! click up with the frames.

mod common;

use std::time::{Duration, Instant};

use common::{cache_data, headless_frames, not_black};
use sf_core::input::FrameInput;
use sf_core::{DisplayData, Game, Progress, Rng, Start};
use sf_e2e::{App, Captured, Image, Key, LaunchOpts, MenuItemState, Mods, read_wav};

/// The steps after which the app took the command `id`, in order.
fn after_steps(c: &Captured, id: &str) -> Vec<u64> {
    let tag = format!("superfrog: menu {id} after step ");
    c.stderr
        .lines()
        .filter_map(|l| l.strip_prefix(&tag)?.trim().parse().ok())
        .collect()
}

fn none(_: u64) -> FrameInput {
    FrameInput::NONE
}

/// The frames move: some frame in `range` differs from the one before it.
fn moves(frames: &[Image], range: std::ops::Range<usize>) -> bool {
    range.into_iter().any(|n| frames[n] != frames[n - 1])
}

#[test]
fn pause_from_a_script_holds_the_picture_and_the_sound_then_resumes_where_it_was() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 300,
        ..Default::default()
    });
    let c = app.run_script("100: menu pause\n150: menu pause\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(after_steps(&c, "pause"), [99, 149]);
    let mut plain = App::launch(LaunchOpts {
        max_frames: 250,
        ..Default::default()
    });
    let b = plain.run_script("");
    assert!(b.exit.success(), "{}", b.stderr);
    assert_eq!((c.frames.len(), b.frames.len()), (301, 251));
    for n in 0..100 {
        assert_eq!(c.frames[n], b.frames[n], "frame {n}, before the pause");
    }
    for n in 100..150 {
        assert_eq!(c.frames[n], b.frames[99], "frame {n}: paused, the picture stays");
    }
    for n in 150..=300 {
        assert_eq!(c.frames[n], b.frames[n - 50], "frame {n}: resumed where it was");
    }
    assert!(moves(&b.frames, 100..250), "the frames after the pause do not move: a weak check");
    assert!(b.frames[100..].iter().any(not_black));
    // The sound stops with the game and carries on from the same point.
    let (ra, sa) = read_wav(&c.audio_wav);
    let (rb, sb) = read_wav(&b.audio_wav);
    assert_eq!(ra, rb);
    assert!(sa == sb, "the paused run's sound is not the unpaused run's ({} vs {} samples)", sa.len(), sb.len());
    // The app's mixer still reads no chip memory (FINDINGS, T22), so both are silent today and
    // this holds trivially; it gains teeth when the app sounds.
}

#[test]
fn reset_from_a_script_is_power_on_and_the_frames_and_sound_then_equal_a_fresh_launch() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 300,
        ..Default::default()
    });
    let data = cache_data(&app.home);
    let c = app.run_script("100: menu reset\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(after_steps(&c, "reset"), [99]);
    let fresh = headless_frames(&data, 201, none);
    for n in 1..100 {
        assert_eq!(c.frames[n], fresh[n - 1], "frame {n}, before the reset");
    }
    for n in 100..=300 {
        assert_eq!(c.frames[n], fresh[n - 100], "frame {n} is not step {} of a fresh launch", n - 99);
    }
    assert!(moves(&fresh, 1..201));
    // Its sound from the reset on is a fresh launch's.
    let mut launch = App::launch(LaunchOpts {
        max_frames: 201,
        ..Default::default()
    });
    let f = launch.run_script("");
    assert!(f.exit.success(), "{}", f.stderr);
    let (_, sc) = read_wav(&c.audio_wav);
    let (_, sf) = read_wav(&f.audio_wav);
    assert!(sc.len() > sf.len());
    assert!(sc[sc.len() - sf.len()..] == sf[..], "the sound after the reset is not a fresh launch's");
    // Silent today, as above.
}

#[test]
fn import_disks_from_a_script_opens_the_picker_and_starts_again_on_the_disks_chosen() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 200,
        ..Default::default()
    });
    let data = cache_data(&app.home);
    let disks = app.disks_in_home("disks");
    let pick: Vec<String> = disks.iter().map(|p| p.display().to_string()).collect();
    let c = app.run_script(&format!("pick {}\n100: menu import\n", pick.join("; ")));
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(
        c.prompts,
        "picker: Choose your four Superfrog disk images, zipped or as .adf files.\n",
        "the picker opened, once"
    );
    let remembered = std::fs::read_to_string(app.home.join("disks.toml")).unwrap();
    assert!(disks.iter().all(|d| remembered.contains(&*d.to_string_lossy())), "{remembered}");
    let fresh = headless_frames(&data, 101, none);
    for n in 100..=200 {
        assert_eq!(c.frames[n], fresh[n - 100], "frame {n}: the game starts again on the disks");
    }

    // Cancelled, the game carries on and nothing is replaced.
    let mut app = App::launch(LaunchOpts {
        max_frames: 200,
        ..Default::default()
    });
    let c = app.run_script("pick cancel\n100: menu import\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.prompts.starts_with("picker: "), "{}", c.prompts);
    assert!(!app.home.join("disks.toml").exists());
    let on = headless_frames(&data, 200, none);
    for n in 1..=200 {
        assert_eq!(c.frames[n], on[n - 1], "frame {n}: a cancelled import changed the game");
    }
}

#[test]
fn quit_from_a_script_or_its_shortcut_ends_the_run_there_with_the_settings_saved() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 200,
        scale: None,
        ..Default::default()
    });
    let c = app.run_script("20: menu scale 3\n50: menu quit\n");
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert_eq!(c.frames.len(), 50, "frames 0 to 49: the run ended before step 50");
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("scale = 3"), "{settings}");
    assert!(c.audio_wav.exists(), "the capture was finished");

    let c = app.run_script("50: down SuperLeft\n50: press KeyQ\n50: up SuperLeft\n");
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert_eq!(after_steps(&c, "quit"), [49], "⌘Q is Quit");
    assert_eq!(c.frames.len(), 50);
    assert_eq!(c.inputs, "", "the Q pressed with ⌘ never reached the game");
}

#[test]
fn scale_from_a_script_sizes_the_window_and_is_remembered_and_fullscreen_is_refused() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 160,
        scale: None,
        ..Default::default()
    });
    let c = app.run_script("10: menu scale 2\n150: shot\n155: menu scale auto\n156: menu fullscreen\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let (img, info) = c.read_shot(150);
    let sf = info.scale_factor as u32;
    assert_eq!(info.surface, (668 * sf, 522 * sf), "2×: {info:?}");
    assert_eq!(common::assert_integer_scaled(&img, &info, &c.read_frame(info.frame)), 2 * sf);
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("scale = \"auto\""), "{settings}");
    assert!(c.stderr.contains("fullscreen refused"), "{}", c.stderr);
    assert!(settings.contains("fullscreen = false"), "{settings}");
}

#[test]
fn continue_is_refused_before_world_2_and_from_world_3_starts_it_as_its_level_code() {
    let data = cache_data(&App::launch(LaunchOpts::default()).home);
    let on = headless_frames(&data, 120, none);
    for progress in [None, Some("world = 1\nlevel = 4\n")] {
        let mut app = App::launch(LaunchOpts {
            max_frames: 120,
            ..Default::default()
        });
        if let Some(p) = progress {
            std::fs::write(app.home.join("progress.toml"), p).unwrap();
        }
        let c = app.run_script("10: menu continue\n");
        assert!(c.exit.success(), "{}", c.stderr);
        assert!(c.stderr.contains("no progress to continue from"), "{progress:?}: {}", c.stderr);
        assert!(c.frames[1..] == on[..], "{progress:?}: a refused Continue changed the game");
    }

    let mut app = App::launch(LaunchOpts {
        max_frames: 300,
        ..Default::default()
    });
    std::fs::write(app.home.join("progress.toml"), "world = 3\nlevel = 1\n").unwrap();
    let c = app.run_script("10: menu continue\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let display = std::sync::Arc::new(DisplayData::new(&data));
    let p = Progress { world: 3, level: 1 };
    let mut game = Game::start_at_with(display, &p, Rng::seeded(0, 0));
    for n in 10..=300usize {
        let pic = Image::of_picture(&game.step(FrameInput::NONE).picture);
        assert_eq!(c.frames[n], pic, "frame {n} is not W3-1's level code's game");
    }
}

// The OS layer: real clicks in the real menu bar, real shortcuts.

fn os_layer() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests click the real menu bar; run them with SF_E2E_OS=1"
    );
    sf_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
}

/// The run's seed, from its capture's `seed.txt`.
fn seed(c: &Captured) -> (u32, u32) {
    let t = std::fs::read_to_string(c.dir.join("seed.txt")).unwrap();
    let v: Vec<u32> = t.split_whitespace().map(|v| v.parse().unwrap()).collect();
    (v[0], v[1])
}

/// The engine headless from power-on at `seed`: the pictures of steps 1..=`frames`.
fn power_on(data: &sf_core::GameData, seed: (u32, u32), frames: u64) -> Vec<Image> {
    let mut g = Game::new(data, Start::PowerOn { seed: Rng::seeded(seed.0, seed.1) });
    (1..=frames).map(|_| Image::of_picture(&g.step(FrameInput::NONE).picture)).collect()
}

/// Wait until `f` holds, at most 20 s.
fn until(what: &str, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(20), "{what}: not within 20 s");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn item(title: &str, enabled: bool, checked: bool) -> MenuItemState {
    MenuItemState {
        title: title.into(),
        enabled,
        checked,
    }
}

/// The window's width, in the window server's units, is `units` game pixels: its bounds are in
/// the main display's points, so on another display they are that display's points scaled
/// (measured 0.9 here), and 3 points in from each side of the frame (FINDINGS, T23).
fn window_is(app: &App, units: f64, pixels: u32) -> bool {
    let (_, b) = app.os_window();
    (b[2] as f64 - units * pixels as f64).abs() <= 8.0
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test menus -- --ignored --test-threads 1"]
fn os_layer_the_menu_bar_holds_every_item_and_continue_follows_the_progress() {
    os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 1500,
        timeout_s: 90,
        ..Default::default()
    });
    let data = cache_data(&app.home);
    app.start_os();
    app.os_window();
    assert_eq!(app.os_menu_bar()[1..], ["superfrog", "File", "Game", "View"]);
    assert_eq!(
        app.os_menu_items(&["superfrog"]),
        [
            item("About Superfrog", true, false),
            item("Services", true, false),
            item("Hide Superfrog", true, false),
            item("Hide Others", true, false),
            item("Show All", true, false),
            item("Quit Superfrog", true, false),
        ]
    );
    assert_eq!(app.os_menu_items(&["File"]), [item("Import Disks…", true, false)]);
    assert_eq!(
        app.os_menu_items(&["Game"]),
        [
            item("Continue", false, false),
            item("Pause", true, false),
            item("Reset", true, false),
            item("Controls…", true, false),
            item("Save State", true, false),
            item("Load State", true, false),
        ],
        "no progress: Continue is disabled"
    );
    assert_eq!(app.os_menu_items(&["View"]), [item("Scale", true, false), item("Fullscreen", true, false)]);
    assert_eq!(
        app.os_menu_items(&["View", "Scale"]),
        [item("1×", true, true), item("2×", true, false), item("3×", true, false), item("Auto", true, false)]
    );

    // About: the standard panel, a window of its own beside the game's. It stays open until the
    // Quit below.
    assert_eq!(app.os_windows().len(), 1);
    app.os_menu(&["superfrog", "About Superfrog"]);
    until("the About panel", || app.os_windows().len() == 2);

    // Hide, and back.
    app.os_menu(&["superfrog", "Hide Superfrog"]);
    until("the app hidden", || app.os_hidden());
    app.os_activate();
    app.os_unhide();
    until("the app shown again", || !app.os_hidden());

    app.os_menu(&["superfrog", "Quit Superfrog"]);
    let c = app.finish();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);

    // With World 2 reached, Continue names it and starts it as its level code does.
    std::fs::write(app.home.join("progress.toml"), "world = 2\nlevel = 1\n").unwrap();
    app.start_os();
    app.os_window();
    let cont = app.os_menu_item(&["Game"], "Continue from World 2");
    assert!(cont.enabled, "{cont:?}");
    std::thread::sleep(Duration::from_millis(500));
    app.os_menu(&["Game", "Continue from World 2"]);
    std::thread::sleep(Duration::from_secs(3));
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish();
    assert_eq!(c.exit.code(), Some(0), "⌘Q quits: {}", c.stderr);
    assert_eq!(after_steps(&c, "quit").len(), 1, "{}", c.stderr);
    let [at] = after_steps(&c, "continue")[..] else { panic!("{}", c.stderr) };
    let s = seed(&c);
    let before = power_on(&data, s, at);
    for n in 1..=at as usize {
        assert_eq!(c.frames[n], before[n - 1], "frame {n}, before Continue");
    }
    let display = std::sync::Arc::new(DisplayData::new(&data));
    let mut game = Game::start_at_with(display, &Progress { world: 2, level: 1 }, Rng::seeded(s.0, s.1));
    let last = c.frames.len() - 1;
    assert!(last as u64 > at + 100, "only {last} frames");
    for n in at as usize + 1..=last {
        let pic = Image::of_picture(&game.step(FrameInput::NONE).picture);
        assert_eq!(c.frames[n], pic, "frame {n} is not W2-1's level code's game");
    }
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("fullscreen = false"), "the settings were saved on Quit: {settings}");
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test menus -- --ignored --test-threads 1"]
fn os_layer_pause_reset_and_a_cancelled_import_through_the_menu_bar() {
    os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 3000,
        timeout_s: 120,
        ..Default::default()
    });
    let data = cache_data(&app.home);
    app.start_os();
    app.os_window();
    let wait = || std::thread::sleep(Duration::from_millis(1500));
    wait();
    app.os_menu(&["Game", "Pause"]);
    until("Pause checked", || app.os_menu_item(&["Game"], "Pause").checked);
    wait();
    app.os_menu(&["Game", "Pause"]);
    until("Pause unchecked", || !app.os_menu_item(&["Game"], "Pause").checked);
    wait();
    app.os_menu(&["Game", "Reset"]);
    wait();
    // The picker is a sheet on the game's window: a second window. Escape cancels it, as it
    // does any open panel.
    app.os_menu(&["File", "Import Disks…"]);
    until("the picker", || app.os_windows().len() == 2);
    app.os_activate();
    std::thread::sleep(Duration::from_millis(500));
    app.os_key_with(Mods::COMMAND, Key::Char('.'));
    until("the picker gone", || app.os_windows().len() == 1);
    wait();
    app.os_menu(&["superfrog", "Quit Superfrog"]);
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert!(c.stderr.contains("menu import after step"), "{}", c.stderr);
    let [p1, p2] = after_steps(&c, "pause")[..] else { panic!("{}", c.stderr) };
    let [r] = after_steps(&c, "reset")[..] else { panic!("{}", c.stderr) };
    let [q] = after_steps(&c, "quit")[..] else { panic!("{}", c.stderr) };
    assert!(p1 > 10 && p2 > p1 + 20 && r > p2 + 20 && q > r + 60, "{p1} {p2} {r} {q}");
    let s = seed(&c);
    let on = power_on(&data, s, q);
    let frame = |n: u64| c.read_frame(n);
    for n in 1..=p1 {
        assert_eq!(frame(n), on[n as usize - 1], "frame {n}, before the pause");
    }
    for n in p1 + 1..=p2 {
        assert_eq!(frame(n), on[p1 as usize - 1], "frame {n}: paused, the picture stays");
    }
    for n in p2 + 1..=r {
        assert_eq!(frame(n), on[(n - (p2 - p1)) as usize - 1], "frame {n}: resumed where it was");
    }
    // Reset: power-on again; the cancelled import changed nothing.
    for n in r + 1..=q {
        assert_eq!(frame(n), on[(n - r) as usize - 1], "frame {n} is not step {} of a fresh launch", n - r);
    }
    assert!(!app.home.join("disks.toml").exists());
}

#[test]
#[ignore = "OS layer, takes over the display: SF_E2E_OS=1 cargo test -p sf-e2e --test menus -- --ignored --test-threads 1"]
fn os_layer_scale_and_fullscreen_through_the_menu_bar_and_its_shortcut() {
    os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 3000,
        timeout_s: 120,
        scale: None,
        ..Default::default()
    });
    app.start_os();
    // Auto opens the window at 2×: what a game pixel measures in the window server's units.
    let units = app.os_window().1[2] as f64 / 668.0;
    for (title, n) in [("3×", 3), ("1×", 1), ("2×", 2)] {
        app.os_menu(&["View", "Scale", title]);
        until(&format!("the window at {title}"), || window_is(&app, units, 334 * n));
        let items = app.os_menu_items(&["View", "Scale"]);
        assert!(items.iter().all(|i| i.checked == (i.title == title)), "{items:?}");
    }
    app.os_menu(&["View", "Scale", "Auto"]);
    until("Auto checked", || app.os_menu_item(&["View", "Scale"], "Auto").checked);

    let windowed = app.os_window().1;
    app.os_menu(&["View", "Fullscreen"]);
    until("fullscreen", || {
        let b = app.os_window().1;
        b[2] > windowed[2] + 200 && b[3] > windowed[3] + 200
    });
    assert!(app.os_menu_item(&["View"], "Fullscreen").checked);
    std::thread::sleep(Duration::from_millis(800));
    app.os_activate();
    app.os_key_with(Mods::CONTROL_COMMAND, Key::Char('f'));
    until("back to the window", || app.os_window().1[2] < windowed[2] + 50);
    assert!(!app.os_menu_item(&["View"], "Fullscreen").checked);
    std::thread::sleep(Duration::from_millis(800));
    app.os_key_with(Mods::CONTROL_COMMAND, Key::Char('f'));
    until("fullscreen by ⌃⌘F", || app.os_window().1[2] > windowed[2] + 200);
    std::thread::sleep(Duration::from_millis(800));
    app.os_menu(&["View", "Fullscreen"]);
    until("back to the window again", || app.os_window().1[2] < windowed[2] + 50);
    app.os_menu(&["superfrog", "Quit Superfrog"]);
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert_eq!(after_steps(&c, "fullscreen").len(), 4, "each toggle once: {}", c.stderr);
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("scale = \"auto\"") && settings.contains("fullscreen = false"), "{settings}");
}

