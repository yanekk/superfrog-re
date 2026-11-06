//! T13: Game → Controls… rebinds the joystick and the binding persists (DESIGN §2.7); settings
//! round-trip, and a damaged `settings.toml` gives the defaults, named, and is left alone until a
//! setting changes (§2.12). Driven through the script layer: `menu controls`, `controls up keys`
//! and a key pressed while the window has the keyboard, as the clicks and typing reach the app.

mod common;

use common::{FIRE, UP};
use fd_e2e::{App, LaunchOpts};

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
fn rebinding_up_to_w_makes_w_jump_space_not_and_it_persists() {
    let mut app = launch(60);
    let c = app.run_script(
        "5: down Space\n8: up Space\n10: menu controls\n11: controls up keys\n12: press KeyW\n13: controls close\n\
         20: down KeyW\n30: up KeyW\n40: down Space\n50: up Space\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(common::joy(&c, 5, 7), [UP; 3], "Space is Up by default");
    assert_eq!(
        common::said(&c),
        [
            "Click a binding, then press the key or pad button for it.",
            "Press the key for Up. Esc cancels.",
            "Up is now W; it no longer works as the Amiga's W key.",
        ]
    );
    assert!(
        common::inputs(&c).range(9..20).next().is_none(),
        "the W typed into the window never reached the game"
    );
    assert_eq!(common::joy(&c, 20, 29), [UP; 10], "W is Up");
    assert_eq!(common::joy(&c, 40, 49), [0; 10], "Space no longer is");
    assert!(
        settings(&app).contains("up = [\"KeyW\"]"),
        "{}",
        settings(&app)
    );

    // The next launch remembers it.
    let c = app.run_script("5: down KeyW\n10: up KeyW\n15: down Space\n20: up Space\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(common::joy(&c, 5, 9), [UP; 5]);
    assert_eq!(common::joy(&c, 15, 19), [0; 5]);
}

#[test]
fn fire_is_left_option_and_restore_defaults_brings_back_this_game_s_keys() {
    let mut app = launch(40);
    let c = app.run_script(
        "3: down AltLeft\n6: up AltLeft\n10: menu controls\n11: controls up keys\n12: press KeyK\n\
         13: controls defaults\n14: controls close\n20: down Space\n25: up Space\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(common::joy(&c, 3, 5), [FIRE; 3]);
    assert_eq!(common::joy(&c, 20, 24), [UP; 5], "Space is Up again");
    let s = settings(&app);
    assert!(
        s.contains("up = [\"Space\"]") && s.contains("fire = [\"AltLeft\"]"),
        "{s}"
    );
}

#[test]
fn a_damaged_settings_file_gives_the_defaults_named_and_is_not_overwritten() {
    let mut app = launch(20);
    let bad = "scale = 0\n[keys]\nup = [\"KeyW\"]\n";
    std::fs::write(app.home.join("settings.toml"), bad).unwrap();
    let c = app.run_script("5: down Space\n10: up Space\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr.contains("fd-app: Your settings could not be read")
            && c.stderr.contains("the defaults are used"),
        "{}",
        c.stderr
    );
    assert_eq!(common::joy(&c, 5, 9), [UP; 5], "the defaults: Space is Up");
    assert_eq!(settings(&app), bad, "left as it was");

    // Changing a setting writes a good file.
    let c = app
        .run_script("3: menu controls\n4: controls fire keys\n5: press KeyF\n6: controls close\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let s = settings(&app);
    assert!(
        s.contains("fire = [\"KeyF\"]") && s.contains("scale = \"auto\""),
        "{s}"
    );
}
