//! bug-recorder T08: the Start Level window behind Game ▸ Start Level… (DESIGN §2.1). Driven twice:
//! from a script, whose `form …` lines answer the window through its model as clicks would (in the
//! test block), and by clicking its dropdowns and typing into it for real (the OS layer,
//! `SF_E2E_OS=1`). Each is checked by what the app says it started and what the game holds at the
//! level's first frame (`superfrog: start-level holds …`).

use std::time::Duration;

use sf_e2e::{App, Captured, Key, LaunchOpts, Mods};

fn launch(max_frames: u64) -> App {
    let mut app = App::launch(LaunchOpts {
        max_frames,
        timeout_s: 120,
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
    app
}

/// What the game held at each level start, as the app logged it.
fn holds(c: &Captured) -> Vec<String> {
    c.stderr
        .lines()
        .filter_map(|l| l.strip_prefix("superfrog: start-level holds "))
        .map(String::from)
        .collect()
}

fn refusals(c: &Captured) -> usize {
    c.stderr.matches("superfrog: start-level: Start is disabled").count()
}

#[test]
fn the_window_s_defaults_start_w1_1_as_a_new_game_with_the_recorder_on() {
    let mut app = launch(12);
    let c = app.run_script("5: menu start-level\nform start\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: started W1-1 after step 4, recording\n"), "{}", c.stderr);
    assert_eq!(
        holds(&c),
        ["frog_lives 2, score 0, frog_hearts 4, opt_game_mode 1, frog_spud_on 0, frog_flight 0"]
    );
    assert!(c.prompts.contains("start-level: -> W1-1 lives=3 points=0 energy=4 difficulty=normal record"), "{}", c.prompts);
}

#[test]
fn points_out_of_range_refuse_start_and_the_values_chosen_start_the_level() {
    let mut app = launch(12);
    let c = app.run_script(
        "5: menu start-level\nform level W1-2\nform lives 7\nform points 1000000\nform start\n\
         form points 450000\nform energy 2\nform difficulty easy\nform wings on\nform spud on\nform start\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(refusals(&c), 1, "Start refused once, with 1000000: {}", c.stderr);
    assert!(c.stderr.contains("superfrog: started W1-2 after step 4, recording\n"), "{}", c.stderr);
    assert_eq!(
        holds(&c),
        ["frog_lives 6, score 450000, frog_hearts 2, opt_game_mode 0, frog_spud_on 1, frog_flight 1"]
    );
}

#[test]
fn the_recorder_switched_off_starts_the_level_without_recording() {
    let mut app = launch(12);
    let c = app.run_script("5: menu start-level\nform level WB-1\nform record off\nform start\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: started WB-1 after step 4\n"), "{}", c.stderr);
}

#[test]
fn cancel_closes_the_window_and_the_game_carries_on_unchanged() {
    let mut app = launch(60);
    let plain = app.run_script("20: down ArrowRight\n40: up ArrowRight\n");
    let c = app.run_script(
        "10: menu start-level\nform level W4-3\nform lives 7\nform cancel\n20: down ArrowRight\n40: up ArrowRight\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(!c.stderr.contains("superfrog: started"), "{}", c.stderr);
    assert!(c.prompts.contains("start-level: -> cancel"), "{}", c.prompts);
    assert_eq!(c.frames.len(), plain.frames.len());
    for (n, (a, b)) in c.frames.iter().zip(&plain.frames).enumerate() {
        assert!(a == b, "frame {n} differs from a run without the window");
    }
}

// The OS layer: the real menu bar, the real window's dropdowns, field, checkboxes and keys.

fn os_layer() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests click and type into the real window; run them with SF_E2E_OS=1"
    );
    sf_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
}

fn until(what: &str, mut f: impl FnMut() -> bool) {
    let t = std::time::Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(20), "{what}: not within 20 s");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The app launched for the OS layer at `scale` (`None`: fullscreen), with its window up.
fn os_app(scale: Option<u32>) -> App {
    let mut app = App::launch(LaunchOpts {
        max_frames: 20_000,
        timeout_s: 180,
        scale: scale.or(Some(1)),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args.extend(["--bugs-dir".to_string(), bugs.display().to_string()]);
    app.start_os();
    app.os_window();
    std::thread::sleep(Duration::from_millis(500));
    if scale.is_none() {
        app.os_menu(&["View", "Fullscreen"]);
        // macOS's fullscreen animation, then the window fills the display.
        std::thread::sleep(Duration::from_millis(2000));
    }
    app
}

/// Game ▸ Start Level…, waiting for its window.
fn open_window(app: &mut App) {
    assert!(app.os_menu_items(&["Game"]).iter().any(|i| i.title == "Start Level…" && i.enabled));
    app.os_menu(&["Game", "Start Level…"]);
    until("the Start Level window", || is_open(app));
}

/// Is the Start Level window up? It sits at AppKit's modal-panel level, which the window list
/// of ordinary windows ([`App::os_windows`]) leaves out, so its buttons are looked for instead.
fn is_open(app: &App) -> bool {
    let b = app.os_buttons();
    b.iter().any(|t| t == "Start") && b.iter().any(|t| t == "Cancel")
}

/// Type `text` into the Points field, replacing what it held.
fn type_points(app: &mut App, text: &str) {
    app.os_focus_id("points");
    std::thread::sleep(Duration::from_millis(200));
    app.os_key_with(Mods::COMMAND, Key::Char('a'));
    for ch in text.chars() {
        app.os_key(Key::Char(ch));
    }
}

fn clipboard() -> String {
    let out = std::process::Command::new("pbpaste").output().expect("pbpaste");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn set_clipboard(text: &str) {
    use std::io::Write;
    let mut p = std::process::Command::new("pbcopy").stdin(std::process::Stdio::piped()).spawn().expect("pbcopy");
    p.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    p.wait().unwrap();
}

/// Quit with ⌘Q.
fn quit(app: &mut App) -> Captured {
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    c
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_start_level -- --ignored --test-threads 1"]
fn os_layer_the_window_opens_on_the_defaults_and_each_dropdown_offers_exactly_its_values() {
    os_layer();
    let mut app = os_app(Some(2));
    open_window(&mut app);
    assert_eq!(app.os_text_id("level"), "W1-1");
    assert_eq!(app.os_text_id("lives"), "3");
    assert_eq!(app.os_text_id("points"), "0");
    assert_eq!(app.os_text_id("energy"), "4");
    assert_eq!(app.os_text_id("difficulty"), "Normal");
    assert!(!app.os_checked_id("spud") && !app.os_checked_id("wings") && app.os_checked_id("record"));
    assert!(app.os_enabled_id("start"));
    let levels: Vec<String> = sf_core::recording::levels().iter().map(ToString::to_string).collect();
    assert_eq!(levels.len(), 26);
    assert_eq!(app.os_popup_titles("level"), levels);
    assert_eq!(app.os_popup_titles("lives"), ["3", "5", "7"]);
    assert_eq!(app.os_popup_titles("energy"), ["1", "2", "3", "4"]);
    assert_eq!(app.os_popup_titles("difficulty"), ["Easy", "Normal"]);
    // Points out of range disable Start and show the range; in range, Start is back.
    type_points(&mut app, "1000000");
    until("Start disabled", || !app.os_enabled_id("start"));
    assert_eq!(app.os_text_id("points-range"), "0–999 999");
    type_points(&mut app, "450000");
    until("Start enabled", || app.os_enabled_id("start"));
    assert_eq!(app.os_text_id("points"), "450000");
    assert_eq!(app.os_text_id("points-range"), "");
    // Pasted, as a burst: ⌘V works though the app has no Edit menu. The clipboard is put back.
    let kept = clipboard();
    set_clipboard("1000000");
    app.os_focus_id("points");
    app.os_key_with(Mods::COMMAND, Key::Char('a'));
    app.os_key_with(Mods::COMMAND, Key::Char('v'));
    let pasted = (0..40).any(|_| {
        std::thread::sleep(Duration::from_millis(50));
        app.os_text_id("points") == "1000000"
    });
    set_clipboard(&kept);
    assert!(pasted, "⌘V pasted: {:?}", app.os_text_id("points"));
    until("Start disabled by the paste", || !app.os_enabled_id("start"));
    type_points(&mut app, "0");
    until("Start enabled", || app.os_enabled_id("start"));
    // Esc closes it: nothing starts and the game carries on.
    app.os_key(Key::Escape);
    until("the window closed", || !is_open(&app));
    std::thread::sleep(Duration::from_millis(500));
    let c = quit(&mut app);
    assert!(!c.stderr.contains("superfrog: started"), "{}", c.stderr);
    let after: Vec<u64> = c
        .stderr
        .lines()
        .filter_map(|l| l.strip_prefix("superfrog: menu start-level after step "))
        .map(|n| n.parse().unwrap())
        .collect();
    let quit_at: u64 = c
        .stderr
        .lines()
        .find_map(|l| l.strip_prefix("superfrog: menu quit after step "))
        .expect("quit")
        .parse()
        .unwrap();
    assert_eq!(after.len(), 1, "{}", c.stderr);
    assert!(quit_at > after[0] + 10, "the game ran on after Esc: {} then {quit_at}", after[0]);
}

/// Start W1-2, 7 lives, easy, wings, recorder on, through the real window at `scale`.
fn start_w1_2(scale: Option<u32>) {
    os_layer();
    let mut app = os_app(scale);
    open_window(&mut app);
    app.os_choose_id("level", "W1-2");
    app.os_choose_id("lives", "7");
    app.os_choose_id("difficulty", "Easy");
    app.os_press_id("wings");
    until("wings ticked", || app.os_checked_id("wings"));
    assert_eq!(app.os_text_id("level"), "W1-2");
    assert_eq!(app.os_text_id("lives"), "7");
    assert_eq!(app.os_text_id("difficulty"), "Easy");
    assert!(app.os_checked_id("record"));
    // Return is Start.
    app.os_activate();
    app.os_key(Key::Return);
    until("the window closed", || !is_open(&app));
    std::thread::sleep(Duration::from_millis(1500));
    // Quit while recording asks first (T10's "Recording in progress"); Discard quits.
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    app.os_press_button("Discard");
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: recording discarded"), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: started W1-2 after step"), "{}", c.stderr);
    assert!(c.stderr.contains(", recording\n"), "{}", c.stderr);
    let held = c.stderr.lines().find_map(|l| l.strip_prefix("superfrog: start-level holds ")).expect("holds");
    assert_eq!(held, "frog_lives 6, score 0, frog_hearts 4, opt_game_mode 0, frog_spud_on 0, frog_flight 1");
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_start_level -- --ignored --test-threads 1"]
fn os_layer_start_with_w1_2_seven_lives_easy_wings_and_the_recorder_at_1x() {
    start_w1_2(Some(1));
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_start_level -- --ignored --test-threads 1"]
fn os_layer_start_with_w1_2_seven_lives_easy_wings_and_the_recorder_at_3x() {
    start_w1_2(Some(3));
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_start_level -- --ignored --test-threads 1"]
fn os_layer_start_with_w1_2_seven_lives_easy_wings_and_the_recorder_in_fullscreen() {
    start_w1_2(None);
}
