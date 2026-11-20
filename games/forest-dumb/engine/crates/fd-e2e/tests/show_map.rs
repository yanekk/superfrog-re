//! Map page T11 (plans/forest-dumb-map-page DESIGN §2.8, §2.9, §2.10): Game ▸ Show Map in the real
//! app. A script's `menu show-map` takes the menu's one command path: the page is written fresh
//! into `<home>/map` on a thread and, the run being scripted, nothing is opened, the app saying
//! `show-map: would open <path>` instead; play is unchanged meanwhile. A folder it cannot write is
//! said on stderr and the run goes on to its frame cap. The OS layer reads the real menu bar:
//! `FD_E2E_OS=1 cargo test --release -p fd-e2e --test show_map -- --ignored --test-threads 1`.
//! It never clicks Show Map: an OS-layer run is not scripted, so that would open the owner's Chrome.
//!
//! Every test needs the owner's disks.

mod common;

use std::os::unix::fs::PermissionsExt;

use amiga_input::FrameInput;
use fd_e2e::{App, Captured, LaunchOpts};

/// The hand engine's headless game stepped on what the app was fed, its state equal to the app's
/// every step: Show Map's thread costs play nothing.
fn same_as_headless(c: &Captured, steps: u64) {
    let fed = common::inputs(c);
    let hashes = common::hashes(c);
    assert_eq!(hashes.len() as u64, steps + 1, "a hash a frame: the run reached its cap");
    let mut g = common::headless();
    assert_eq!(hashes[0], common::hash(&g), "the start");
    for n in 1..=steps {
        g.step(fed.get(&n).copied().unwrap_or(FrameInput::NONE));
        g.take_events();
        assert_eq!(hashes[n as usize], common::hash(&g), "the state after step {n}");
    }
}

/// The step the app quit after, from its `quit after step N` line.
fn quit_step(c: &Captured) -> u64 {
    c.stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: quit after step "))
        .unwrap_or_else(|| panic!("no quit line: {}", c.stderr))
        .parse()
        .unwrap()
}

/// Smoke: `100: menu show-map` on the hand engine writes the six levels' page into `<home>/map`,
/// says it would open it, opens nothing, and the state is the headless game's every step.
#[test]
fn a_scripted_show_map_writes_the_page_into_its_home_and_opens_nothing() {
    const STEPS: u64 = 160;
    let mut app = App::launch(LaunchOpts {
        max_frames: STEPS,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let c = app.run_script_unread("fast-until 150\n10: down ArrowRight\n100: menu show-map\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let map = app.home.join("map");
    let page = map.join("index.html");
    let would = format!("fd-app: show-map: would open {}", page.display());
    assert_eq!(
        c.stderr.lines().filter(|l| l.contains("show-map:")).collect::<Vec<_>>(),
        [would.as_str()],
        "{}",
        c.stderr
    );
    assert!(c.stderr.contains("fd-app: menu show-map after step 99"), "{}", c.stderr);
    let html = std::fs::read_to_string(&page).unwrap();
    let mut data: Vec<String> = std::fs::read_dir(map.join("data"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    data.sort();
    assert_eq!(data, ["W1-1.js", "W2-1.js", "W3-1.js", "W4-1.js", "W5-1.js", "W6-1.js"]);
    for id in ["W1-1", "W6-1"] {
        assert!(html.contains(id), "the page lists {id}");
    }
    assert_eq!(quit_step(&c), STEPS);
    same_as_headless(&c, STEPS);
}

/// `<home>/map` read-only: Show Map says on stderr which folder it could not write, nothing is
/// written, and the run goes on to its frame cap, its state the headless game's.
#[test]
fn a_map_folder_it_cannot_write_is_named_and_the_run_goes_on() {
    const STEPS: u64 = 160;
    let mut app = App::launch(LaunchOpts {
        max_frames: STEPS,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let map = app.home.join("map");
    std::fs::create_dir_all(&map).unwrap();
    std::fs::set_permissions(&map, std::fs::Permissions::from_mode(0o555)).unwrap();
    let c = app.run_script_unread("fast-until 150\n100: menu show-map\n");
    // The home is removed on drop: its folder writable again first.
    std::fs::set_permissions(&map, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(c.exit.success(), "{}", c.stderr);
    let said: Vec<&str> = c.stderr.lines().filter(|l| l.contains("show-map:")).collect();
    assert_eq!(said.len(), 1, "{}", c.stderr);
    let want = format!("could not be written to {}", map.display());
    assert!(said[0].contains(&want) && said[0].ends_with("the game carries on"), "{}", said[0]);
    assert!(!map.join("index.html").exists());
    assert_eq!(quit_step(&c), STEPS);
    same_as_headless(&c, STEPS);
}

fn os_layer() {
    assert!(
        std::env::var_os("FD_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests read the real menu bar; run them with FD_E2E_OS=1"
    );
    fd_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
}

/// The real menu bar lists Show Map under Game, enabled, between Stop Recording and Controls….
#[test]
#[ignore = "OS layer, a real window: FD_E2E_OS=1 cargo test --release -p fd-e2e --test show_map -- --ignored --test-threads 1"]
fn os_layer_the_game_menu_lists_show_map() {
    os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 3000,
        timeout_s: 60,
        ..Default::default()
    });
    app.start_os();
    app.os_window();
    let items = app.os_menu_items(&["Game"]);
    let titles: Vec<&str> = items
        .iter()
        .map(|i| i.title.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(titles, ["Start Level…", "Stop Recording", "Show Map", "Controls…"]);
    assert!(app.os_menu_item(&["Game"], "Show Map").enabled);
    app.os_menu(&["fd-app", "Quit Forest Dumb"]);
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert!(!c.stderr.contains("show-map"), "nothing clicked it: {}", c.stderr);
}
