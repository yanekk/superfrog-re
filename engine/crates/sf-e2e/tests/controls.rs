//! T25: remappable keys and gamepads (DESIGN §2.6, §2.9). The Controls window is driven twice:
//! from a script (`menu controls`, `controls fire keys`, a key pressed while it is open; in the
//! test block), and by clicking it and typing into it for real (the OS layer, `SF_E2E_OS=1`). The
//! pads come from a script's `pad …` lines, the fake gilrs source a scripted run reads instead of
//! real pads. Each is checked by what the game was fed (`inputs.txt`), what the window said (the
//! app logs every line it shows as `superfrog: controls: …`) and what `settings.toml` remembers.

mod common;

use std::collections::BTreeMap;
use std::time::Duration;

use sf_e2e::{App, Captured, Key, LaunchOpts, Mods};

const UP: u8 = 1;
const DOWN: u8 = 2;
const LEFT: u8 = 4;
const RIGHT: u8 = 8;
const FIRE: u8 = 16;

/// What the game was fed at each step that had any input: the joystick bits and the Amiga keys
/// (hex codes, comma separated).
fn inputs(c: &Captured) -> BTreeMap<u64, (u8, String)> {
    c.inputs
        .lines()
        .map(|l| {
            let mut w = l.split(' ');
            let n = w.next().unwrap().parse().unwrap();
            let joy = u8::from_str_radix(w.next().unwrap(), 16).unwrap();
            (n, (joy, w.next().unwrap_or("").to_string()))
        })
        .collect()
}

/// The joystick at each step from `from` to `to`, inclusive (0 where nothing was fed).
fn joy(c: &Captured, from: u64, to: u64) -> Vec<u8> {
    let i = inputs(c);
    (from..=to).map(|n| i.get(&n).map_or(0, |e| e.0)).collect()
}

/// Every line the Controls window showed under its rows, in order.
fn said(c: &Captured) -> Vec<String> {
    c.stderr
        .lines()
        .filter_map(|l| l.strip_prefix("superfrog: controls: "))
        .map(String::from)
        .collect()
}

fn settings(app: &App) -> String {
    std::fs::read_to_string(app.home.join("settings.toml")).expect("settings.toml")
}

fn launch(max_frames: u64) -> App {
    App::launch(LaunchOpts {
        max_frames,
        ..Default::default()
    })
}

#[test]
fn rebinding_fire_to_x_in_the_controls_window_makes_x_fire_space_not_and_it_persists() {
    let mut app = launch(80);
    let c = app.run_script(
        "10: menu controls\n11: controls fire keys\n12: press KeyX\n13: controls close\n\
         20: down KeyX\n30: up KeyX\n40: down Space\n50: up Space\n60: down ControlLeft\n70: up ControlLeft\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(
        said(&c),
        [
            "Click a binding, then press the key or pad button for it.",
            "Press the key for Fire. Esc cancels.",
            "Fire is now X; it no longer works as the Amiga's X key.",
        ]
    );
    let i = inputs(&c);
    assert!(i.range(..20).next().is_none(), "the X typed into the window never reached the game: {i:?}");
    assert_eq!(joy(&c, 20, 29), [FIRE; 10], "X fires");
    assert!((20..30).all(|n| i[&n].1.is_empty()), "X no longer types X: {i:?}");
    assert!(i.range(30..).next().is_none(), "Space and Ctrl no longer fire: {i:?}");
    let s = settings(&app);
    assert!(s.contains("fire = [\"KeyX\"]"), "{s}");
    assert!(s.contains("\"key X\" = []"), "{s}");

    // Relaunched, the binding is remembered.
    let c = app.run_script("20: down KeyX\n30: up KeyX\n40: press Space\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(joy(&c, 20, 29), [FIRE; 10]);
    assert_eq!(inputs(&c).len(), 10, "only X's ten steps: {}", c.inputs);
}

#[test]
fn a_key_bound_to_a_second_input_is_taken_from_the_first_and_the_window_says_so() {
    let mut app = launch(60);
    let c = app.run_script(
        "5: menu controls\n6: controls fire keys\n7: press ArrowUp\n8: controls close\n\
         20: down ArrowUp\n30: up ArrowUp\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(
        said(&c).last().map(String::as_str),
        Some("Fire is now ↑; it no longer works as Up. Up has no key now.")
    );
    assert_eq!(joy(&c, 20, 29), [FIRE; 10], "↑ fires and moves nothing");
    let s = settings(&app);
    assert!(s.contains("fire = [\"ArrowUp\"]") && s.contains("\nup = []\n"), "{s}");
}

#[test]
fn restore_defaults_restores_both_maps_and_esc_cancels_then_closes() {
    let mut app = launch(80);
    let c = app.run_script(
        "5: menu controls\n6: controls fire keys\n7: press KeyZ\n8: controls left pad\n9: pad press 1 North\n\
         10: controls right keys\n11: press Escape\n12: press Escape\n\
         20: down KeyZ\n21: pad down 1 North\n22: up KeyZ\n22: pad up 1 North\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: controls closed"), "Esc, not waiting, closes: {}", c.stderr);
    assert_eq!(said(&c)[6], "Nothing changed.", "{:?}", said(&c));
    assert_eq!(joy(&c, 20, 22), [FIRE, FIRE | LEFT, 0]);
    let s = settings(&app);
    assert!(s.contains("fire = [\"KeyZ\"]") && s.contains("left = [\"North\"]"), "{s}");

    let c = app.run_script(
        "5: menu controls\n6: controls defaults\n7: controls close\n\
         20: press Space\n21: pad press 1 South\n22: pad press 1 North\n23: press KeyZ\n24: pad press 1 DPadLeft\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(said(&c).last().unwrap(), "The keys and the pad are back to their defaults.");
    assert_eq!(joy(&c, 20, 24), [FIRE, FIRE, 0, 0, LEFT]);
    assert_eq!(inputs(&c)[&23].1, "31", "Z types Z again");
    let defaults = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    let fresh = {
        let mut a = launch(1);
        a.run_script("1: menu controls\n1: controls defaults\n");
        settings(&a)
    };
    assert_eq!(defaults, fresh, "both maps are the defaults a fresh home writes");
}

#[test]
fn the_pad_moves_and_fires_and_unplugging_mid_move_releases_only_that_pad() {
    let mut app = launch(60);
    // Pads 1 and 2; the stick's dead zone is half way.
    let c = app.run_script(
        "5: pad connect 1\n5: pad connect 2\n\
         10: pad stick 1 0.4 -0.5\n\
         12: pad stick 1 0.75 0.75\n\
         14: pad down 2 South\n\
         16: pad down 1 DPadDown\n\
         18: pad disconnect 1\n\
         20: pad up 2 South\n\
         22: pad connect 1\n\
         24: pad press 1 East\n\
         26: pad stick 1 -1 0\n\
         28: pad stick 1 0 0\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(
        joy(&c, 10, 29),
        [
            0, 0, // inside the dead zone
            UP | RIGHT, UP | RIGHT, // the diagonal
            UP | RIGHT | FIRE, UP | RIGHT | FIRE, // pad 2's A
            UP | RIGHT | FIRE | DOWN, UP | RIGHT | FIRE | DOWN,
            FIRE, FIRE, // pad 1 unplugged mid-move: its stick and d-pad let go, pad 2's A holds
            0, 0, 0, 0,
            FIRE, 0, // a tap inside one frame is seen once
            LEFT, LEFT, 0, 0,
        ]
    );
    for l in ["pad 1 connected", "pad 2 connected", "pad 1 disconnected"] {
        assert!(c.stderr.contains(&format!("superfrog: {l}")), "{l}: {}", c.stderr);
    }
}

#[test]
fn a_pad_button_rebound_in_the_controls_window_fires_and_the_press_that_bound_it_does_not() {
    let mut app = launch(40);
    let c = app.run_script(
        "5: menu controls\n6: controls fire pad\n7: pad down 1 West\n8: pad up 1 West\n\
         10: pad down 1 West\n12: pad up 1 West\n14: pad press 1 South\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(said(&c).last().unwrap(), "Fire is now X.");
    assert_eq!(joy(&c, 5, 15), [0, 0, 0, 0, 0, FIRE, FIRE, 0, 0, 0, 0]);
    assert!(settings(&app).contains("fire = [\"West\"]"));
}

// The OS layer: the real menu bar, the real window's buttons, real keys.

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

/// Hold `key` down in the game window for about a second, then quit with ⌘Q.
fn hold_then_quit(app: &mut App, key: Option<Key>) -> Captured {
    app.os_window();
    std::thread::sleep(Duration::from_millis(800));
    app.os_activate();
    std::thread::sleep(Duration::from_millis(400));
    if let Some(k) = key {
        app.os_key_down(k);
        std::thread::sleep(Duration::from_millis(1000));
        app.os_key_up(k);
        std::thread::sleep(Duration::from_millis(300));
    }
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    c
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test controls -- --ignored --test-threads 1"]
fn os_layer_rebind_fire_to_x_in_the_controls_window_and_it_persists_across_relaunch() {
    os_layer();
    let mut app = App::launch(LaunchOpts {
        max_frames: 3000,
        timeout_s: 120,
        ..Default::default()
    });
    app.start_os();
    app.os_window();
    assert!(app.os_menu_items(&["Game"]).iter().any(|i| i.title == "Controls…" && i.enabled));
    app.os_menu(&["Game", "Controls…"]);
    until("the Controls window", || app.os_windows().len() == 2);
    assert_eq!(app.os_text_id("fire-keys"), "Space, Left Ctrl, Right Ctrl");
    assert_eq!(app.os_text_id("fire-pad"), "A, B");
    app.os_press_id("fire-keys");
    until("waiting for a key", || app.os_text_id("fire-keys") == "Press a key…");
    assert_eq!(app.os_text_id("status"), "Press the key for Fire. Esc cancels.");
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key(Key::Char('x'));
    until("X bound", || app.os_text_id("fire-keys") == "X");
    assert_eq!(app.os_text_id("status"), "Fire is now X; it no longer works as the Amiga's X key.");
    assert!(settings(&app).contains("fire = [\"KeyX\"]"), "remembered at once");
    // Esc, not waiting, closes the window; the game's window has the keyboard again.
    app.os_key(Key::Escape);
    until("the Controls window closed", || app.os_windows().len() == 1);
    let c = hold_then_quit(&mut app, Some(Key::Char('x')));
    let i = inputs(&c);
    assert!(i.values().any(|e| e.0 == FIRE), "X held in the game fired: {}", c.inputs);
    assert!(i.values().all(|e| e.0 == FIRE && e.1.is_empty()), "only fire, never the X key: {}", c.inputs);

    // Relaunched: Space no longer fires…
    app.start_os();
    let c = hold_then_quit(&mut app, Some(Key::Space));
    assert_eq!(c.inputs, "", "Space fed the game: {}", c.stderr);
    // …and X does.
    app.start_os();
    let c = hold_then_quit(&mut app, Some(Key::Char('x')));
    assert!(inputs(&c).values().any(|e| e.0 == FIRE), "X after relaunch: {}", c.inputs);

    // Restore Defaults through its button: Space fires again.
    app.start_os();
    app.os_window();
    app.os_menu(&["Game", "Controls…"]);
    until("the Controls window", || app.os_windows().len() == 2);
    app.os_press_id("defaults");
    until("the defaults", || app.os_text_id("fire-keys") == "Space, Left Ctrl, Right Ctrl");
    assert_eq!(app.os_text_id("status"), "The keys and the pad are back to their defaults.");
    app.os_activate();
    app.os_key(Key::Escape);
    until("the Controls window closed", || app.os_windows().len() == 1);
    let c = hold_then_quit(&mut app, Some(Key::Space));
    assert!(inputs(&c).values().any(|e| e.0 == FIRE), "Space after Restore Defaults: {}", c.inputs);
    let s = settings(&app);
    assert!(s.contains("fire = [\"Space\", \"ControlLeft\", \"ControlRight\"]"), "{s}");
}
