//! Translator T37 (DESIGN §2.10, §2.14): the app built with no translated program says which
//! command generates it and exits 1, before it reads the disks or opens a window. Needs no disks
//! and no tree: smoke.
//!
//! Built with a tree (the owner's machine after `translate.py --build`), the same launch plays the
//! translated program instead, which is checked here too, so either build tests something.

use fd_e2e::{App, LaunchOpts};
use fd_translated::TranslatedGame;

#[test]
fn with_no_translation_built_in_the_app_names_the_command_and_exits_1() {
    // A folder without the zips: the message must come before the disks are looked for.
    let empty = std::env::temp_dir().join(format!("fd-e2e-no-tree-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    let mut app = App::launch_translated_bare(LaunchOpts {
        max_frames: 10,
        extra_args: vec!["--disks".into(), empty.display().to_string()],
        ..Default::default()
    });
    let c = app.run_script("");
    std::fs::remove_dir_all(&empty).unwrap();
    assert!(!c.stderr.contains("panicked"), "{}", c.stderr);
    if TranslatedGame::generated() {
        // The tree is in: the app goes on to the disks, which this folder lacks.
        assert_eq!(c.exit.code(), Some(1), "{}", c.stderr);
        assert!(c.stderr.contains("zips not found"), "{}", c.stderr);
        assert!(!c.stderr.contains(fd_app::NOT_GENERATED), "{}", c.stderr);
    } else {
        assert_eq!(c.exit.code(), Some(1), "{}", c.stderr);
        assert!(
            c.stderr.contains("python3 -I games/forest-dumb/tools/translate.py --build"),
            "names the command: {}",
            c.stderr
        );
        assert!(!c.stderr.contains("zips not found"), "before the disks: {}", c.stderr);
    }
}

#[test]
fn the_hand_engine_needs_no_translation() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 5,
        ..Default::default()
    });
    let c = app.run_script("");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: playing W1-1"), "{}", c.stderr);
    assert!(!c.stderr.contains(fd_app::NOT_GENERATED), "{}", c.stderr);
}
