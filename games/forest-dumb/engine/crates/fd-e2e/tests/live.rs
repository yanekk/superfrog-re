//! Map page T10 (plans/forest-dumb-map-page DESIGN §2.5, §2.7, §2.9, §2.10): the real app, scripted,
//! writes the map page's `live.js` into `<home>/map` every 12th step from either engine, its last
//! frame where a headless run is at that step, its state hashes those of a headless run (which
//! writes nothing), a folder it cannot write costing play nothing, and the title not play.
//!
//! The hand engine's tests need the owner's disks; the translated program's also the generated
//! tree (`translate.py --build`), and are full.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use amiga_input::FrameInput;
use fd_conform::FrameEngine;
use fd_conform::translated_engine::TranslatedEngine;
use fd_core::live::{live_from_game, live_from_ram};
use fd_e2e::{App, Captured, LaunchOpts};
use fd_translated::TranslatedGame;

/// Chip RAM's size, where slow RAM starts in the headless engine's memory.
const CHIP: usize = 512 << 10;

/// The live file in a scripted run's home.
fn live_js(home: &Path) -> std::path::PathBuf {
    home.join("map").join("live.js")
}

/// The value after `"key":` in the live file's text, up to the next `,` or `}` outside brackets.
fn field<'a>(text: &'a str, key: &str) -> &'a str {
    let at = text
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("no {key} in {text}"))
        + key.len()
        + 3;
    let rest = &text[at..];
    let mut depth = 0;
    let end = rest
        .char_indices()
        .find(|&(_, ch)| match ch {
            '[' | '{' => {
                depth += 1;
                false
            }
            ']' | '}' if depth > 0 => {
                depth -= 1;
                false
            }
            ',' | '}' => depth == 0,
            _ => false,
        })
        .map_or(rest.len(), |(i, _)| i);
    &rest[..end]
}

/// The run's live file, read and checked to be one whole frame.
fn read_live(home: &Path) -> String {
    let path = live_js(home);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(
        text.starts_with("window.LIVE = {") && text.ends_with("};\n"),
        "one whole frame: {text}"
    );
    text
}

/// The route's first `steps` as the default keys, fast to the last 25.
fn route_script(steps: usize) -> String {
    let route = common::route();
    format!(
        "fast-until {}\n{}",
        steps - 25,
        common::key_lines(&route[..steps])
    )
}

/// The hand engine's headless game stepped on what the app was fed, its state equal to the app's
/// every step (the app writes the live file, the headless game nothing).
fn hand_follow(c: &Captured, steps: u64) -> fd_core::Game {
    let fed = common::inputs(c);
    let hashes = common::hashes(c);
    assert_eq!(hashes.len() as u64, steps + 1, "a hash a frame");
    let mut g = common::headless();
    assert_eq!(hashes[0], common::hash(&g), "the start");
    for n in 1..=steps {
        g.step(fed.get(&n).copied().unwrap_or(FrameInput::NONE));
        g.take_events();
        assert_eq!(hashes[n as usize], common::hash(&g), "the state after step {n}");
    }
    g
}

/// The route's first 600 steps on the hand engine: `<home>/map/live.js` holds step 600's frame,
/// the character's cell and items left the headless game's, and play is unchanged by the writing.
#[test]
fn a_scripted_hand_run_writes_its_last_frame_where_the_headless_game_is() {
    const STEPS: u64 = 600;
    let mut app = App::launch(LaunchOpts {
        max_frames: STEPS,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let c = app.run_script_unread(&route_script(STEPS as usize));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(!c.stderr.contains("live map"), "{}", c.stderr);
    let text = read_live(&app.home);
    let g = hand_follow(&c, STEPS);
    let at = g.location();
    let want = live_from_game(&g, &common::data().level.objects.cells);
    assert_eq!(field(&text, "playing"), "true", "{text}");
    assert_eq!(field(&text, "level"), "\"W1-1\"", "{text}");
    assert_eq!(field(&text, "cell"), format!("[{},{}]", at.col, at.row), "{text}");
    assert_eq!(
        field(&text, "items_left"),
        want.items_left.unwrap().to_string(),
        "{text}"
    );
    assert_eq!(
        field(&text, "cells"),
        format!(
            "[{}]",
            want.changed
                .iter()
                .map(|(c, r, v)| format!("[{c},{r},{v}]"))
                .collect::<Vec<_>>()
                .join(",")
        ),
        "the cells taken by step {STEPS}"
    );
    assert!(!want.changed.is_empty(), "the route takes items by step {STEPS}");
}

/// Smoke: 120 steps of the route on the hand engine write `<home>/map/live.js`, never the owner's
/// page, its last frame step 120's, where the headless game is.
#[test]
fn a_short_hand_run_writes_live_js_into_its_home() {
    const STEPS: u64 = 120;
    let mut app = App::launch(LaunchOpts {
        max_frames: STEPS,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let c = app.run_script_unread(&route_script(STEPS as usize));
    assert!(c.exit.success(), "{}", c.stderr);
    let text = read_live(&app.home);
    let at = hand_follow(&c, STEPS).location();
    assert_eq!(field(&text, "playing"), "true", "{text}");
    assert_eq!(field(&text, "level"), "\"W1-1\"", "{text}");
    assert_eq!(field(&text, "cell"), format!("[{},{}]", at.col, at.row), "{text}");
}

/// The map folder read-only: the run finishes, its state the headless game's every step, one
/// stderr line naming the failure, no live file.
#[test]
fn a_map_folder_it_cannot_write_costs_play_nothing() {
    const STEPS: u64 = 120;
    let mut app = App::launch(LaunchOpts {
        max_frames: STEPS,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let map = app.home.join("map");
    std::fs::create_dir_all(&map).unwrap();
    std::fs::set_permissions(&map, std::fs::Permissions::from_mode(0o555)).unwrap();
    let c = app.run_script_unread("fast-until 100\n10: down ArrowRight\n");
    // The home is removed on drop: its folder writable again first.
    std::fs::set_permissions(&map, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(c.exit.success(), "{}", c.stderr);
    let said: Vec<&str> = c.stderr.lines().filter(|l| l.contains("live map")).collect();
    assert_eq!(said.len(), 1, "said once, though every sample failed: {}", c.stderr);
    assert!(
        said[0].contains(&live_js(&app.home).display().to_string()),
        "names the file: {}",
        said[0]
    );
    assert!(!live_js(&app.home).exists());
    hand_follow(&c, STEPS);
}

fn require_tree() {
    assert!(
        TranslatedGame::generated(),
        "no translated program built in: run `python3 -I games/forest-dumb/tools/translate.py --build`"
    );
}

/// The conformance runner's translated program from power-on, level 1 mounted, as the app runs it.
fn headless_translated() -> TranslatedEngine {
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut e = TranslatedEngine::power_on(&disks).unwrap_or_else(|e| panic!("{e}"));
    e.mount_level_1(common::data());
    e
}

fn memory_hash(e: &TranslatedEngine) -> String {
    let m = e.ram().expect("a frame has run");
    format!("{:016x}", fd_app::memory_hash(&m[..CHIP], &m[CHIP..]))
}

/// On the translated program's disk 2 prompt, before any play: the app writes `playing: false`.
#[test]
fn on_the_title_it_writes_not_playing() {
    require_tree();
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: 48,
        timeout_s: 120,
        ..Default::default()
    });
    let c = app.run_script_unread("");
    assert!(c.exit.success(), "{}", c.stderr);
    let text = read_live(&app.home);
    assert_eq!(field(&text, "playing"), "false", "{text}");
    assert_eq!(field(&text, "level"), "null", "{text}");
    assert_eq!(field(&text, "cell"), "null", "{text}");
}

/// `--level 1` and the route's first 240 steps on the translated program: the last frame's cell and
/// items left are where a headless run's RAM says (`location_in_ram`), and the state is the
/// headless run's every step.
#[test]
fn a_scripted_translated_run_in_w1_1_writes_where_its_ram_says() {
    require_tree();
    const STEPS: u64 = 240;
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: STEPS,
        timeout_s: 180,
        extra_args: vec![
            "--level".into(),
            "1".into(),
            "--capture-hashes".into(),
        ],
        ..Default::default()
    });
    let c = app.run_script_unread(&route_script(STEPS as usize));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(!c.stderr.contains("live map"), "{}", c.stderr);
    let text = read_live(&app.home);
    let frames: u64 = c
        .stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: level 1 reached headless after "))
        .and_then(|l| l.split(' ').next())
        .unwrap_or_else(|| panic!("{}", c.stderr))
        .parse()
        .unwrap();
    let mut e = headless_translated();
    for n in 0..frames {
        e.frame(fd_app::to_level_1_input(n));
    }
    let hashes = common::hashes(&c);
    assert_eq!(memory_hash(&e), hashes[0], "level 1's first frame");
    let fed = common::inputs(&c);
    // The headless RAM sampled on the app's steps, against W1-1's object file, is what the app's
    // copied ranges and its world-to-file lookup must reproduce.
    let file = common::data().level.objects.cells.clone();
    let start = move |w: u8| (w == 1).then(|| file.clone());
    let (mut prev, mut want) = (None, None);
    for n in 1..=STEPS {
        e.frame(fed.get(&n).copied().unwrap_or_default());
        assert_eq!(memory_hash(&e), hashes[n as usize], "the state after step {n}");
        if n % 12 == 0 {
            let m = e.ram().unwrap();
            let l = live_from_ram(&m[..CHIP], &m[CHIP..], prev, &start);
            prev = Some(l.irq);
            want = Some(l);
        }
    }
    let want = want.unwrap();
    let m = e.ram().unwrap();
    let at = fd_app::location_in_ram(&m[..CHIP], &m[CHIP..]).expect("in level 1");
    assert_eq!(field(&text, "playing"), "true", "{text}");
    assert_eq!(field(&text, "level"), "\"W1-1\"", "{text}");
    assert_eq!(field(&text, "cell"), format!("[{},{}]", at.col, at.row), "{text}");
    // Any earlier `irq_count` marks it play; the sample's other fields do not depend on it.
    let items = live_from_ram(&m[..CHIP], &m[CHIP..], Some(0), &|_| None).items_left;
    assert_eq!(field(&text, "items_left"), items.unwrap().to_string(), "{text}");
    let cells = want.changed.iter().map(|(c, r, v)| format!("[{c},{r},{v}]"));
    assert_eq!(field(&text, "cells"), format!("[{}]", cells.collect::<Vec<_>>().join(",")));
    assert!(!want.changed.is_empty(), "the route takes items by step {STEPS}");
    let actors = want.creatures.iter().map(|(x, y, _)| format!("[{x},{y},\"enemy\"]"));
    assert_eq!(field(&text, "actors"), format!("[{}]", actors.collect::<Vec<_>>().join(",")));
}

