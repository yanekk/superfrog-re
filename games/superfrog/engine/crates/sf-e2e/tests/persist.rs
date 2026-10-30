//! T23: what persists across launches (DESIGN §2.9), the single-instance lock and a Mac with no
//! audio output (DESIGN §2.11), and the keyboard moving the frog in W1-1 (DESIGN §2.6), through
//! the real binary.

mod common;

use std::time::{Duration, Instant};

use common::{cache_data, headless, not_black};
use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, Progress, Rng, VarId};
use sf_e2e::{App, Image, LaunchOpts};

#[test]
fn the_scale_set_in_one_run_is_the_window_of_the_next() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 20,
        scale: None,
        ..Default::default()
    });
    let c = app.run_script("10: scale 3\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("scale = 3"), "{settings}");
    app.opts.max_frames = 155;
    let c = app.run_script("150: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let (img, info) = c.read_shot(150);
    let sf = info.scale_factor as u32;
    assert_eq!(info.surface, (1002 * sf, 783 * sf), "relaunched at 3×: {info:?}");
    common::assert_integer_scaled(&img, &info, &c.read_frame(info.frame));
}

#[test]
fn a_remembered_fullscreen_stays_windowed_under_a_script_without_allow_fullscreen() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 30,
        ..Default::default()
    });
    std::fs::write(app.home.join("settings.toml"), "fullscreen = true\n").unwrap();
    let c = app.run_script("20: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("staying windowed"), "{}", c.stderr);
    let (_, info) = c.read_shot(20);
    let sf = info.scale_factor as u32;
    assert_eq!(info.surface, (334 * sf, 261 * sf));
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert_eq!(settings, "fullscreen = true\n", "the setting is kept for a normal launch");
}

#[test]
#[ignore = "takes over the display: SF_E2E_OS=1 cargo test -p sf-e2e --test persist -- --ignored"]
fn fullscreen_is_remembered_across_a_relaunch() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "the fullscreen run takes over the display; run it with SF_E2E_OS=1"
    );
    let mut app = App::launch(LaunchOpts {
        max_frames: 120,
        extra_args: vec!["--allow-fullscreen".into()],
        ..Default::default()
    });
    let c = app.run_script("10: down ControlLeft\n10: down SuperLeft\n10: press KeyF\n10: up SuperLeft\n10: up ControlLeft\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("fullscreen = true"), "{settings}");
    let c = app.run_script("100: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let (img, info) = c.read_shot(100);
    let sf = info.scale_factor as u32;
    assert!(info.surface.0 > 668 * sf && info.surface.1 > 522 * sf, "relaunched fullscreen: {info:?}");
    let frame = c.read_frame(info.frame);
    let s = common::assert_integer_scaled(&img, &info, &frame);
    let fit = (info.surface.0 / frame.width).min(info.surface.1 / frame.height);
    assert_eq!(s, fit, "fullscreen is the largest whole scale: {info:?}");
}

#[test]
fn damaged_settings_and_progress_give_the_defaults_a_message_each_and_the_game_starts() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 20,
        ..Default::default()
    });
    std::fs::write(app.home.join("settings.toml"), "scale = \"enormous\"\n").unwrap();
    std::fs::write(app.home.join("progress.toml"), [0xffu8, 0, 1, 2]).unwrap();
    let c = app.run_script("");
    assert!(c.exit.success(), "{}", c.stderr);
    let notices: Vec<&str> = c.prompts.lines().collect();
    assert_eq!(notices.len(), 2, "{}", c.prompts);
    assert!(notices[0].starts_with("notice: Your settings could not be read"), "{}", notices[0]);
    assert!(notices[1].starts_with("notice: Your progress could not be read"), "{}", notices[1]);
    assert_eq!(c.frames.len(), 21, "the game ran");
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("scale = \"auto\""), "replaced by the defaults: {settings}");
    assert!(!app.home.join("progress.toml").exists(), "the damaged progress is forgotten");
    let c = app.run_script("");
    assert_eq!(c.prompts, "", "the next launch is quiet");
}

/// T193's chain to W1-1 (fire on the title's intro, its menu, START GAME; W1-1 plays from about
/// frame 1 885, the frog takes the stick from about 2 045), then the keyboard: right held, Ctrl
/// and Space (fire) tapped, left held. (Right held much longer runs the frog to a lost life.)
const KEYS_SCRIPT: &str = "100: press Space\n200: press Space\n250: press Space\n\
    2050: down ArrowRight\n2075: up ArrowRight\n2078: down ControlLeft\n2079: up ControlLeft\n\
    2085: press Space\n2095: down ArrowLeft\n2120: up ArrowLeft\n";
const KEYS_FRAMES: u64 = 2140;

/// The `FrameInput` [`KEYS_SCRIPT`] gives step `n`.
fn keys_input(n: u64) -> FrameInput {
    match n {
        100 | 200 | 250 | 2078 | 2085 => FrameInput::joy(Joy::FIRE),
        2050..2075 => FrameInput::joy(Joy::RIGHT),
        2095..2120 => FrameInput::joy(Joy::LEFT),
        _ => FrameInput::NONE,
    }
}

fn var(game: &Game, v: VarId) -> u32 {
    game.probe().into_iter().find(|p| p.0 == v).map(|p| p.1).expect("a modelled variable")
}

#[test]
fn arrow_keys_and_fire_move_the_frog_in_w1_1_as_the_same_input_does_headless_and_progress_is_saved() {
    let mut app = App::launch(LaunchOpts {
        max_frames: KEYS_FRAMES,
        timeout_s: 150,
        ..Default::default()
    });
    let data = cache_data(&app.home);
    let c = app.run_script_unread(KEYS_SCRIPT);
    assert!(c.exit.success(), "the run failed ({}): {}", c.exit, c.stderr);
    let want_inputs: String = (1..=KEYS_FRAMES)
        .filter(|&n| keys_input(n) != FrameInput::NONE)
        .map(|n| format!("{n} {:02x} \n", keys_input(n).joy.bits()))
        .collect();
    assert_eq!(c.inputs, want_inputs, "the keys reached the game as the joystick");

    // The engine alone, from the same start and the same FrameInputs: every frame from the
    // first key on is the app's, and the keys moved the frog.
    let mut game = headless(&data);
    let mut x = std::collections::BTreeMap::new();
    for n in 1..=KEYS_FRAMES {
        let pic = game.step(keys_input(n)).picture;
        if n >= 2040 {
            assert_eq!(c.read_frame(n), Image::of_picture(&pic), "frame {n} is not the engine's");
            x.insert(n, var(&game, VarId::FrogX));
        }
    }
    assert_eq!(game.scene(), sf_core::Scene::Level, "the keys were pressed in W1-1");
    assert!(x[&2074] > x[&2049] + 50, "right moved the frog right: {} → {}", x[&2049], x[&2074]);
    assert!(x[&2119] + 50 < x[&2094], "left moved it left: {} → {}", x[&2094], x[&2119]);
    assert!(not_black(&c.read_frame(KEYS_FRAMES)));
    assert_eq!(
        std::fs::read_to_string(app.home.join("progress.toml")).unwrap(),
        "world = 1\nlevel = 1\n",
        "W1-1 reached is the progress remembered"
    );
}

#[test]
fn the_remembered_world_is_offered_and_continues_as_its_level_code_starts_it() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 300,
        ..Default::default()
    });
    let data = cache_data(&app.home);
    std::fs::write(app.home.join("progress.toml"), "world = 2\nlevel = 1\n").unwrap();
    let c = app.run_script("10: continue\n");
    assert!(c.exit.success(), "{}", c.stderr);
    // Headless: power-on to step 9, then the game a level code for W2-1 starts.
    let mut game = headless(&data);
    for n in 1..10 {
        let pic = Image::of_picture(&game.step(FrameInput::NONE).picture);
        assert_eq!(c.frames[n], pic, "frame {n}, before the continue");
    }
    let display = std::sync::Arc::new(sf_core::DisplayData::new(&data));
    let p = Progress { world: 2, level: 1 };
    let mut game = Game::start_at_with(display, &p, Rng::seeded(0, 0));
    let mut shown = 0;
    for n in 10..=300u64 {
        let pic = Image::of_picture(&game.step(FrameInput::NONE).picture);
        assert_eq!(c.frames[n as usize], pic, "frame {n} is not the continued game's");
        shown += not_black(&pic) as u32;
    }
    assert!(shown > 0, "the continued game showed nothing");
    assert_eq!(
        std::fs::read_to_string(app.home.join("progress.toml")).unwrap(),
        "world = 2\nlevel = 1\n",
        "a nearer level never lowers the furthest"
    );
}

#[test]
fn a_second_copy_says_so_and_quits_and_a_killed_copy_s_lock_is_taken_over() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 1500,
        ..Default::default()
    });
    app.start_script("");
    let pid = app.pid().unwrap();
    let lock = app.home.join("lock");
    let t = Instant::now();
    while std::fs::read_to_string(&lock).unwrap_or_default().trim() != pid.to_string() {
        assert!(t.elapsed() < Duration::from_secs(20), "the first copy took no lock");
        std::thread::sleep(Duration::from_millis(50));
    }
    let second = app.run_script("");
    assert_eq!(second.exit.code(), Some(1), "{}", second.stderr);
    assert_eq!(
        second.prompts,
        format!(
            "notice: Superfrog is already running (process {pid}). Only one copy can run at a \
             time.\n"
        )
    );
    assert!(second.frames.is_empty(), "the second copy ran no game");

    // kill -9: the lock file stays, naming a dead process, and is taken over.
    app.kill();
    assert_eq!(std::fs::read_to_string(&lock).unwrap().trim(), pid.to_string());
    app.opts.max_frames = 10;
    let third = app.run_script("");
    assert!(third.exit.success(), "{}", third.stderr);
    assert_eq!(third.prompts, "");
    assert_eq!(third.frames.len(), 11);
}

#[test]
fn with_no_audio_output_the_game_runs_silently_at_the_same_pace() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 100,
        extra_args: vec!["--audio-device".into(), "sf-e2e-no-such-device".into()],
        ..Default::default()
    });
    let data = cache_data(&app.home);
    let t = Instant::now();
    let c = app.run_script("");
    let took = t.elapsed();
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr.contains("no audio output device is called \"sf-e2e-no-such-device\"; carrying on silently"),
        "{}",
        c.stderr
    );
    // 100 PAL frames are 2.003 s on the wall-clock pacer, as with a device.
    assert!(took >= Duration::from_millis(2000) && took < Duration::from_secs(8), "{took:?}");
    let want = common::headless_frames(&data, 100, |_| FrameInput::NONE);
    assert_eq!(c.frames.len(), 101);
    assert!(c.frames[1..] == want[..], "the frames are the engine's");
}

/// What the capture's `inputs.txt` says each step saw.
fn captured_inputs(text: &str) -> std::collections::BTreeMap<u64, FrameInput> {
    text.lines()
        .map(|l| {
            let mut w = l.split(' ');
            let n = w.next().unwrap().parse().unwrap();
            let joy = Joy(u8::from_str_radix(w.next().unwrap(), 16).unwrap());
            let mut keys = sf_core::input::KeySet::NONE;
            for k in w.next().unwrap_or("").split(',').filter(|k| !k.is_empty()) {
                keys = keys.with(u8::from_str_radix(k, 16).unwrap());
            }
            (n, FrameInput { joy, keys })
        })
        .collect()
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test persist -- --ignored"]
fn os_layer_a_real_key_press_moves_the_frog() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests post real events to the screen; run them with SF_E2E_OS=1"
    );
    sf_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
    // The script takes the game to W1-1 (it is the same chain as above); the right arrow is a real
    // key, posted once the frog takes the stick.
    const FRAMES: u64 = 2300;
    let mut app = App::launch(LaunchOpts {
        max_frames: FRAMES,
        timeout_s: 150,
        ..Default::default()
    });
    let data = cache_data(&app.home);
    app.start_script("100: press Space\n200: press Space\n250: press Space\n");
    let ready = app.home.join("capture-0/frame-02050.png");
    let t = Instant::now();
    while !ready.exists() {
        assert!(t.elapsed() < Duration::from_secs(100), "the run never reached frame 2050");
        std::thread::sleep(Duration::from_millis(20));
    }
    app.os_window();
    app.os_activate();
    std::thread::sleep(Duration::from_millis(200));
    app.os_key_down(sf_e2e::Key::Right);
    std::thread::sleep(Duration::from_millis(500));
    app.os_key_up(sf_e2e::Key::Right);
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);

    // Steps that saw the stick to the right: the real key, about 25 of them, after frame 2050.
    let inputs = captured_inputs(&c.inputs);
    let right: Vec<u64> = inputs.iter().filter(|(_, i)| i.joy == Joy::RIGHT).map(|(n, _)| *n).collect();
    assert!(right.len() >= 10, "the key reached {} steps: {:?}", right.len(), c.inputs);
    assert!(right[0] > 2050, "{right:?}");
    // The engine with the same inputs: the app's frames, and the frog moved right while it was held.
    let mut game = headless(&data);
    let mut x = std::collections::BTreeMap::new();
    for n in 1..=FRAMES {
        let pic = game.step(inputs.get(&n).copied().unwrap_or(FrameInput::NONE)).picture;
        x.insert(n, var(&game, VarId::FrogX));
        if n % 50 == 0 {
            assert_eq!(c.read_frame(n), Image::of_picture(&pic), "frame {n}");
        }
    }
    let (first, last) = (right[0], *right.last().unwrap());
    assert!(x[&last] > x[&(first - 1)] + 20, "the frog moved {} → {}", x[&(first - 1)], x[&last]);
}
