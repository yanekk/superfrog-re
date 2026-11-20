//! T26: save states (DESIGN §2.8). Through the real app, at 2×: Game → Save State mid-level, play
//! on, Load State, and the screen is the frame at save time and the game goes on from there as the
//! engine does headless; the shortcuts ⇧⌘N and ⌘N do the same; a state from another build is
//! refused with a message naming both builds, and the game carries on untouched.

mod common;

use common::{cache_data, headless, not_black};
use sf_core::input::FrameInput;
use sf_core::{Game, Progress, Rng, Scene};
use sf_e2e::{App, Image, LaunchOpts};

fn opts(max_frames: u64) -> LaunchOpts {
    LaunchOpts {
        max_frames,
        timeout_s: 120,
        scale: Some(2),
        ..Default::default()
    }
}

/// The game a scripted run plays after `10: continue` from World 2, stepped to step `to` with no
/// input, and its frames from step 10 on (index = step).
fn continued(data: &sf_core::GameData, to: u64) -> (Game, Vec<Option<Image>>) {
    let mut frames = vec![None; to as usize + 1];
    let mut game = headless(data);
    for _ in 1..10 {
        game.step(FrameInput::NONE);
    }
    let display = std::sync::Arc::new(sf_core::DisplayData::new(data));
    let mut game = Game::start_at_with(display, &Progress { world: 2, level: 1 }, Rng::seeded(0, 0));
    for n in 10..=to {
        frames[n as usize] = Some(Image::of_picture(&game.step(FrameInput::NONE).picture));
    }
    (game, frames)
}

#[test]
fn a_state_saved_mid_level_and_loaded_after_playing_on_shows_the_saved_frame_and_goes_on_from_it() {
    // W2-1 plays from about step 1 657 after a continue at step 10.
    let mut app = App::launch(LaunchOpts {
        timeout_s: 180,
        ..opts(2100)
    });
    let data = cache_data(&app.home);
    std::fs::write(app.home.join("progress.toml"), "world = 2\nlevel = 1\n").unwrap();
    let c = app.run_script(
        "10: continue\n1800: menu save-state-1\n1810: down ArrowRight\n1850: up ArrowRight\n\
         1900: menu pause\n1910: menu load-state-1\n1920: menu pause\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: saved state 1 at game frame "), "{}", c.stderr);
    assert!(app.home.join("states/slot-1.sfstate").is_file(), "the state is in --home/states/");

    // Up to the save, the app is the engine headless; the save comes after step 1799, in W2-1.
    let (mut game, frames) = continued(&data, 1799);
    assert_eq!(game.scene(), Scene::Level, "saved mid-level");
    for n in 10..=1799 {
        assert_eq!(&c.frames[n], frames[n].as_ref().unwrap(), "frame {n}, before the save");
    }
    let saved = &c.frames[1799];
    assert!(not_black(saved));
    // Played on: the frog was moved, so the picture before the load is not the saved one.
    assert_ne!(&c.frames[1909], saved, "played on after the save: a weak check otherwise");
    // Loaded while paused: the screen is the frame at save time until the game resumes.
    for n in 1910..1920 {
        assert_eq!(&c.frames[n], saved, "frame {n}: loaded, paused, the saved frame is shown");
    }
    // Resumed: the game goes on from the save as the engine does from step 1799.
    for n in 1920..=2100 {
        let want = Image::of_picture(&game.step(FrameInput::NONE).picture);
        assert_eq!(c.frames[n], want, "frame {n}: the loaded game goes on from the saved one");
    }
    assert!(c.prompts.is_empty(), "no message: {}", c.prompts);
}

#[test]
fn shift_cmd_n_saves_and_cmd_n_loads() {
    let mut app = App::launch(opts(300));
    let c = app.run_script(
        "150: down SuperLeft\n150: down ShiftLeft\n150: press Digit3\n150: up ShiftLeft\n150: up SuperLeft\n\
         200: down SuperLeft\n200: press Digit3\n200: up SuperLeft\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: menu save-state-3 after step 149"), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: menu load-state-3 after step 199"), "{}", c.stderr);
    assert!(app.home.join("states/slot-3.sfstate").is_file());
    // Step 200 is step 150 again, and so on: the keys never reached the game.
    for n in 200..250 {
        assert_eq!(c.frames[n], c.frames[n - 50], "frame {n} is frame {} again", n - 50);
    }
    assert!(c.inputs.is_empty(), "keys with ⌘ reached the game: {}", c.inputs);
}

#[test]
fn a_state_from_another_build_is_refused_with_both_builds_named_and_the_game_carries_on() {
    let mut app = App::launch(opts(300));
    let data = cache_data(&app.home);
    let hash = sf_assets::cache_hash(&data);
    let mut other = headless(&data);
    for _ in 0..50 {
        other.step(FrameInput::NONE);
    }
    let theirs = "superfrog 0.0.9 (0123456789abcdef)";
    std::fs::create_dir_all(app.home.join("states")).unwrap();
    std::fs::write(app.home.join("states/slot-2.sfstate"), other.save(theirs, &hash)).unwrap();
    let c = app.run_script("100: menu load-state-2\n200: down SuperLeft\n200: press Digit2\n200: up SuperLeft\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let lines: Vec<&str> = c.prompts.lines().collect();
    assert_eq!(lines.len(), 2, "one message a load: {}", c.prompts);
    let head = format!("notice: Save state 2 was made by another version of this game, {theirs}; this is superfrog ");
    for l in &lines {
        assert!(l.starts_with(&head) && l.ends_with(". It was not loaded."), "{l}");
        let ours = &l[head.len() - "superfrog ".len()..l.len() - ". It was not loaded.".len()];
        assert!(ours.starts_with("superfrog 0.1.0 (") && ours != theirs, "{ours}");
    }
    // Untouched: every frame is the engine's from power-on with no input.
    let mut g = headless(&data);
    for n in 1..=300 {
        let want = Image::of_picture(&g.step(FrameInput::NONE).picture);
        assert_eq!(c.frames[n], want, "frame {n}: the game carried on as if nothing was loaded");
    }
}

// The OS layer: the real menu bar clicked, the real shortcuts pressed (SF_E2E_OS=1).

/// The steps after which the app took the command `id`, in order.
fn after_steps(c: &sf_e2e::Captured, id: &str) -> Vec<u64> {
    let tag = format!("superfrog: menu {id} after step ");
    c.stderr.lines().filter_map(|l| l.strip_prefix(&tag)?.trim().parse().ok()).collect()
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test states -- --ignored --test-threads 1"]
fn os_layer_save_state_from_the_menu_bar_and_cmd_1_loads_it() {
    use sf_e2e::{Key, Mods};
    use std::time::Duration;
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests click the real menu bar; run them with SF_E2E_OS=1"
    );
    sf_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
    let mut app = App::launch(LaunchOpts {
        timeout_s: 90,
        ..opts(3000)
    });
    app.start_os();
    app.os_window();
    let titles: Vec<String> = app.os_menu_items(&["Game", "Load State"]).into_iter().map(|i| i.title).collect();
    assert_eq!(titles, ["1", "2", "3", "4", "5"]);
    assert!(!app.os_menu_item(&["Game", "Load State"], "1").enabled, "an empty slot cannot be loaded");
    std::thread::sleep(Duration::from_millis(500));
    app.os_menu(&["Game", "Save State", "1"]);
    std::thread::sleep(Duration::from_secs(2));
    assert!(app.os_menu_item(&["Game", "Load State"], "1").enabled, "a saved slot can be loaded");
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('1'));
    std::thread::sleep(Duration::from_secs(2));
    app.os_key_with(Mods::SHIFT_COMMAND, Key::Char('2'));
    std::thread::sleep(Duration::from_millis(500));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    let [saved] = after_steps(&c, "save-state-1")[..] else { panic!("{}", c.stderr) };
    let [loaded] = after_steps(&c, "load-state-1")[..] else { panic!("{}", c.stderr) };
    assert_eq!(after_steps(&c, "save-state-2").len(), 1, "⇧⌘2 saves: {}", c.stderr);
    assert!(app.home.join("states/slot-1.sfstate").is_file() && app.home.join("states/slot-2.sfstate").is_file());
    // After the load the game goes on from the save: the same frames again, with no input.
    let end = (c.frames.len() as u64 - 1).min(loaded + 50);
    assert!(end > loaded + 10, "too few frames after the load");
    for k in 1..=end - loaded {
        assert_eq!(
            c.frames[(loaded + k) as usize],
            c.frames[(saved + k) as usize],
            "frame {} after the load is frame {} after the save",
            loaded + k,
            saved + k
        );
    }
}
