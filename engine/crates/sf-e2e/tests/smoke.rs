//! T22's end-to-end smoke tests: the real `superfrog` binary, driven by both layers (DESIGN §5).
//!
//! The script layer runs in the test block. The OS-layer tests are `#[ignore]`d there, so they
//! show as ignored, never passed; run them with
//! `SF_E2E_OS=1 cargo test -p sf-e2e --test smoke -- --ignored` (README.md).

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, Rng, Start, VarId, View};
use sf_e2e::{App, Captured, Image, Key, LaunchOpts, binary, os_permissions, read_wav};
use sf_paula::Paula;

/// The script of the shared run: one tap on the joystick's left at step 10.
const SCRIPT: &str = "10: press ArrowLeft\n";
const FRAMES: u64 = 500;

/// One 500-frame scripted run, shared by the tests that read it: what it captured, the game data
/// it ran on and its audio (read before its home is removed).
type Run = (Captured, sf_core::GameData, (u32, Vec<[i16; 2]>));

fn shared_run() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(|| {
        let mut app = App::launch(LaunchOpts {
            max_frames: FRAMES,
            ..Default::default()
        });
        let data = cache_data(&app.home);
        let c = app.run_script(SCRIPT);
        assert!(c.exit.success(), "the run failed ({}): {}", c.exit, c.stderr);
        let audio = read_wav(&c.audio_wav);
        (c, data, audio)
    })
}

/// The game data the app ran on: its home's cache, decoded.
fn cache_data(home: &Path) -> sf_core::GameData {
    let path = home.join(format!("cache/assets-v{}.bin", sf_assets::CACHE_FORMAT));
    sf_assets::cache_decode(&std::fs::read(&path).unwrap()).expect("the home's cache")
}

/// The input the shared run's script gives step `n`.
fn script_input(n: u64) -> FrameInput {
    if n == 10 { FrameInput::joy(Joy::LEFT) } else { FrameInput::NONE }
}

/// The engine, headless, from the start a scripted run makes (power-on, seed 0 0).
fn headless(data: &sf_core::GameData) -> Game {
    Game::new(data, Start::PowerOn { seed: Rng::seeded(0, 0) })
}

#[test]
fn the_first_frame_is_power_on_and_frame_400_is_the_engine_s() {
    let (c, data, _) = shared_run();
    assert_eq!(c.frames.len() as u64, FRAMES + 1, "frames 0..={FRAMES}");
    let mut game = headless(data);
    assert_eq!(c.frames[0], Image::of_picture(&game.render(View::default())));
    assert_eq!((c.frames[0].width, c.frames[0].height), (320, 256));
    assert!(c.frames[0].rgba.chunks(4).all(|p| p[..3] == [0, 0, 0]), "power-on is black");
    let mut pic = None;
    for n in 1..=400 {
        pic = Some(game.step(script_input(n)).picture);
    }
    assert_eq!(c.frames[400], Image::of_picture(&pic.unwrap()));
}

/// Chip RAM with nothing in it, as the app's mixer reads it until `Game` runs the sound driver.
struct NoChip;
impl sf_paula::ChipMemory for NoChip {
    fn chip_word(&self, _: u32) -> u16 {
        0
    }
}

#[test]
fn the_captured_audio_is_the_mixer_s_output_for_the_same_writes() {
    let (_, data, (rate, samples)) = shared_run();
    let rate = *rate;
    let mut game = headless(data);
    let mut paula = Paula::new(rate);
    let mut want = Vec::new();
    for n in 1..=FRAMES {
        let out = game.step(script_input(n));
        paula.frame(&out.paula, &NoChip, &mut want);
    }
    assert!(want.len() > (FRAMES as usize) * (rate as usize) / 51, "{} samples", want.len());
    assert_eq!(samples.len(), want.len());
    assert!(*samples == want, "the captured audio differs from sf-paula's");
}

#[test]
fn a_script_s_events_reach_the_game_as_its_input() {
    let (c, _, _) = shared_run();
    assert_eq!(c.inputs, "10 04 \n", "a tap on ArrowLeft at 10 is joystick left for step 10");
}

#[test]
fn losing_focus_lets_go_of_every_held_key() {
    // macOS sends no key-up for a key held while the window loses focus (winit 0.30 resets only
    // the modifiers), so the app lets go of everything held, or the frog runs on after Cmd-Tab.
    let mut app = App::launch(LaunchOpts {
        max_frames: 30,
        ..Default::default()
    });
    let c = app.run_script("10: down ArrowLeft\n10: down KeyP\n20: blur\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let want: String = (10..20).map(|n| format!("{n} 04 19\n")).collect();
    assert_eq!(c.inputs, want, "held from step 10, released by the focus loss before step 20");
}

#[test]
fn max_frames_ends_the_run_by_itself_with_status_0_at_the_pal_rate() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 100,
        ..Default::default()
    });
    let t = Instant::now();
    let c = app.run_script("");
    let took = t.elapsed();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert_eq!(c.frames.len(), 101);
    // 100 PAL frames are 2.003 s; the pacer never runs ahead of its clock.
    assert!(took >= Duration::from_millis(2000), "100 frames in {took:?}");
}

#[test]
fn timeout_kills_a_stuck_run_and_everything_in_its_process_group() {
    let app = App::launch(LaunchOpts::default());
    let script = app.home.join("hang.script");
    std::fs::write(&script, "5: hang\n").unwrap();
    let t = Instant::now();
    let mut child = Command::new(binary())
        .arg("--home")
        .arg(&app.home)
        .arg("--script")
        .arg(&script)
        .args(["--max-frames", "100000", "--timeout", "2"])
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .unwrap();
    let pgid = child.id();
    // A child of the run: another process in its group, as a helper the app started would be.
    let mut helper = Command::new("sleep")
        .arg("60")
        .process_group(pgid as i32)
        .spawn()
        .unwrap();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        assert!(t.elapsed() < Duration::from_secs(30), "--timeout 2 did not stop the run");
        std::thread::sleep(Duration::from_millis(20));
    };
    let took = t.elapsed();
    let helper_status = helper.wait().unwrap();
    let mut err = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err).unwrap();
    assert!(!status.success(), "{status}");
    assert!(took >= Duration::from_secs(2) && took < Duration::from_secs(15), "{took:?}");
    assert!(err.contains("hangs at frame 5"), "{err}");
    assert!(err.contains("--timeout 2 s reached"), "{err}");
    assert!(!helper_status.success(), "the helper in the run's group survived: {helper_status}");
    assert!(!sf_e2e::group_alive(pgid), "something in the run's group survived");
}

#[test]
fn a_script_without_a_frame_cap_or_with_fullscreen_past_300_frames_is_refused() {
    let app = App::launch(LaunchOpts::default());
    let script = app.home.join("empty.script");
    std::fs::write(&script, "").unwrap();
    for (args, says) in [
        (vec![], "--max-frames"),
        (vec!["--max-frames", "301", "--allow-fullscreen"], "--allow-fullscreen"),
    ] {
        let out = Command::new(binary())
            .arg("--home")
            .arg(&app.home)
            .arg("--script")
            .arg(&script)
            .args(&args)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {err}");
        assert!(err.contains(says), "{args:?}: {err}");
    }
}

/// Every file under `dir`, with its size and modification time; empty when `dir` is absent.
fn listing(dir: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let m = e.metadata().unwrap();
            if m.is_dir() {
                todo.push(e.path());
            }
            out.push((e.path(), m.len(), m.modified().unwrap()));
        }
    }
    out.sort();
    out
}

#[test]
fn home_is_honoured_and_nothing_is_written_to_the_real_app_data_folder() {
    let real = PathBuf::from(std::env::var_os("HOME").unwrap())
        .join("Library/Application Support/Superfrog-RE");
    let before = listing(&real);
    let mut app = App::launch(LaunchOpts {
        max_frames: 10,
        ..Default::default()
    });
    // A home with no cache: the app imports into it.
    std::fs::remove_dir_all(app.home.join("cache")).unwrap();
    let disks = sf_conform::disks::disk_dir().unwrap();
    app.opts.extra_args = vec!["--cache-from-disks".into(), disks.to_string_lossy().into()];
    let c = app.run_script("");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(app.home.join(format!("cache/assets-v{}.bin", sf_assets::CACHE_FORMAT)).is_file());
    assert_eq!(listing(&real), before, "the run wrote into {}", real.display());
}

#[test]
fn the_disks_are_read_only_and_nothing_is_written_beside_them() {
    let src = sf_conform::disks::disk_dir().unwrap();
    let dir = std::env::temp_dir().join(format!("sf-e2e-disks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(&src).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().to_lowercase();
        if name.ends_with(".zip") || name.ends_with(".adf") {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    let set_mode = |mode: u32| {
        use std::os::unix::fs::PermissionsExt;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            std::fs::set_permissions(e.path(), std::fs::Permissions::from_mode(mode & 0o666)).unwrap();
        }
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    set_mode(0o555);
    let before = listing(&dir);
    let home = dir.with_extension("home");
    let out = Command::new(binary())
        .arg("--home")
        .arg(&home)
        .arg("--cache-from-disks")
        .arg(&dir)
        .args(["--max-frames", "0", "--timeout", "120"])
        .output()
        .unwrap();
    let after = listing(&dir);
    set_mode(0o755);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&home);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(after, before);
}

/// The OS layer runs only when asked (DESIGN §5): it posts real events and brings a window up.
fn require_os_layer() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests post real events to the screen; run them with SF_E2E_OS=1"
    );
    os_permissions().unwrap_or_else(|e| panic!("{e}"));
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test smoke -- --ignored"]
fn os_layer_a_real_key_reaches_the_game_and_the_window_shows_the_game_area() {
    require_os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 400,
        os: true,
        ..Default::default()
    });
    let (_, bounds) = app.os_window();
    app.os_activate();
    std::thread::sleep(Duration::from_millis(500));
    app.os_key(Key::Left);
    let shot = app.os_capture();
    let c = app.finish();
    assert!(c.exit.success(), "{}", c.stderr);
    // The key reached the game: some step saw joystick left.
    assert!(
        c.inputs.lines().any(|l| l.split(' ').nth(1) == Some("04")),
        "no step saw joystick left: {:?}",
        c.inputs
    );
    // The window holds the 1× game area. macOS 26 reports window bounds and captures 3 points in
    // from each side of the frame (winit's outer size is 320×288 where the window server says
    // 314×282), so the game area may lose that much at its edges.
    const INSET: i32 = 3;
    assert!(
        bounds[2] >= 320 - 2 * INSET && bounds[3] >= 256 - 2 * INSET,
        "window bounds {bounds:?}"
    );
    let px = (shot.width as f64 / bounds[2] as f64).round() as u32; // pixels a point
    assert!(px >= 1, "capture {}×{} for bounds {bounds:?}", shot.width, shot.height);
    // The bottom of the window, inside the inset, is the game's black power-on frame.
    let (gw, gh) = ((320 - 2 * INSET as u32) * px, (256 - 2 * INSET as u32) * px);
    let x0 = (shot.width - gw) / 2;
    let y0 = shot.height - gh - INSET as u32 * px;
    let black = (y0..y0 + gh)
        .step_by(7)
        .all(|y| (x0..x0 + gw).step_by(7).all(|x| shot.rgb(x, y) == [0, 0, 0]));
    assert!(black, "the window's game area is not the black power-on frame");
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test smoke -- --ignored"]
fn os_layer_a_menu_click_reaches_the_app() {
    require_os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 2000,
        os: true,
        ..Default::default()
    });
    app.os_window();
    let t = Instant::now();
    app.os_menu(&["superfrog", "Quit Superfrog"]);
    let c = app.finish();
    assert!(
        t.elapsed() < Duration::from_secs(10),
        "Quit took {:?}: the run went on to its frame cap",
        t.elapsed()
    );
    assert!(c.exit.success(), "{} {}", c.exit, c.stderr);
}

/// T193's script: fire skips the title's intro, opens its menu and picks START GAME. The loading
/// picture's walk and the progress map then take the original's time, so W1-1 plays from about
/// frame 1 900 (engine-only, seed 0 0: the loading picture from 363, the map from 1 529, the
/// level scene from 1 885).
const CHAIN_SCRIPT: &str = "100: press Space\n200: press Space\n250: press Space\n";
const CHAIN_FRAMES: u64 = 2000;

/// The input [`CHAIN_SCRIPT`] gives step `n`.
fn chain_input(n: u64) -> FrameInput {
    if [100, 200, 250].contains(&n) { FrameInput::joy(Joy::FIRE) } else { FrameInput::NONE }
}

/// `world` and `level` (h0+$40da, h0+$40de) as the engine holds them.
fn world_level(game: &Game) -> (u32, u32) {
    let long = |v: VarId| {
        let at = sf_core::LAYOUT.iter().find(|d| d.id == v).unwrap().addr;
        u32::from_be_bytes(game.peek(at, 4).try_into().unwrap())
    };
    (long(VarId::World), long(VarId::Level))
}

/// The engine's picture with the frog's two sprites hidden: their copper MOVEs to SPRxPOS and
/// SPRxCTL (the record's copper words, `frog::anim::spr::COPPER`, and its attached partner's)
/// set to a start and stop of line 0, so the hardware never shows them.
fn without_frog(game: &Game) -> sf_core::Picture {
    use sf_core::level::camera::{FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT};
    let mut g = game.clone();
    for rec in [FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT] {
        let at = rec + sf_core::frog::anim::spr::COPPER;
        let cop = u32::from_be_bytes(g.peek(at, 4).try_into().unwrap());
        for value in [0xa, 0xe, 0x1a, 0x1e] {
            g.poke(cop + value, &[0, 0]);
        }
    }
    g.render(View::default())
}

#[test]
fn power_on_fire_on_the_title_and_start_game_plays_w1_1() {
    let mut app = App::launch(LaunchOpts {
        max_frames: CHAIN_FRAMES,
        timeout_s: 150,
        ..Default::default()
    });
    let data = cache_data(&app.home);
    let c = app.run_script_unread(CHAIN_SCRIPT);
    assert!(c.exit.success(), "the run failed ({}): {}", c.exit, c.stderr);
    assert_eq!(c.inputs, "100 10 \n200 10 \n250 10 \n", "fire reached the game at 100, 200, 250");

    // The engine alone, headless, from the same start and input: the pictures the app must show.
    let mut game = headless(&data);
    let mut level_from = None;
    let mut want = std::collections::BTreeMap::new();
    for n in 1..=CHAIN_FRAMES {
        let pic = game.step(chain_input(n)).picture;
        if level_from.is_none() && game.scene() == sf_core::Scene::Level {
            level_from = Some(n);
        }
        if n % 100 == 0 || n > CHAIN_FRAMES - 50 {
            want.insert(n, pic);
        }
    }
    let level_from = level_from.expect("the engine never reached a level");
    assert_eq!(world_level(&game), (1, 1), "the level played is W1-1");
    assert!(level_from < CHAIN_FRAMES - 50, "W1-1 plays from frame {level_from}");

    // Every sampled frame is the engine's; the walk, the map and W1-1's are not black (the fades
    // between them are).
    for (&n, pic) in &want {
        let shot = c.read_frame(n);
        assert_eq!(shot, Image::of_picture(pic), "frame {n} is not the engine's");
        if n == 1000 || n == 1700 || n > CHAIN_FRAMES - 50 {
            assert!(shot.rgba.chunks(4).any(|p| p[..3] != [0, 0, 0]), "frame {n} is black");
        }
    }

    // The frog's sprite is in the last frame: hiding it changes what the app showed, and only
    // around the frog.
    let last = c.read_frame(CHAIN_FRAMES);
    let hidden = Image::of_picture(&without_frog(&game));
    let differ: Vec<(u32, u32)> = (0..last.height)
        .flat_map(|y| (0..last.width).map(move |x| (x, y)))
        .filter(|&(x, y)| last.rgb(x, y) != hidden.rgb(x, y))
        .collect();
    assert!(differ.len() >= 100, "the frog's sprite shows {} pixels", differ.len());
    let (xs, ys) = (differ.iter().map(|p| p.0), differ.iter().map(|p| p.1));
    let w = xs.clone().max().unwrap() - xs.min().unwrap();
    let h = ys.clone().max().unwrap() - ys.min().unwrap();
    assert!(w < 32 && h < 48, "the hidden sprite changed a {w}×{h} area, not the frog's");
}
