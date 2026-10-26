//! real-widescreen T01 (DESIGN §2.1, §3.4): the widescreen mode reaches every level step, and
//! until a site reads it, switching it on changes nothing the game does or draws.

use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, GameData, LevelId, Start};

const BUILD: &str = "superfrog test-build";

fn data() -> &'static GameData {
    sf_conform::game_data()
}

/// Frame `f`'s input: a walk right with jumps and fire, so the camera scrolls and objects spawn.
fn input(f: u32) -> FrameInput {
    let mut joy = Joy::NONE;
    if f % 90 < 60 {
        joy = joy | Joy::RIGHT;
    } else if f % 90 < 75 {
        joy = joy | Joy::LEFT;
    }
    if f % 37 < 4 {
        joy = joy | Joy::UP;
    }
    if f % 53 < 2 {
        joy = joy | Joy::FIRE;
    }
    FrameInput::joy(joy)
}

fn w1_1() -> Game {
    Game::new(data(), Start::Level("W1-1".parse::<LevelId>().unwrap()))
}

#[test]
fn the_mode_is_off_by_default_and_set_by_the_app() {
    let mut g = w1_1();
    assert!(!g.widescreen());
    g.set_widescreen(true);
    assert!(g.widescreen());
    g.set_widescreen(false);
    assert!(!g.widescreen());
}

/// No site reads the mode yet (T02–T05 do): a W1-1 run with it on is the run with it off, state,
/// picture and sound, frame for frame.
#[test]
fn widescreen_on_changes_no_state_picture_or_sound_until_a_site_reads_it() {
    let (mut off, mut on) = (w1_1(), w1_1());
    on.set_widescreen(true);
    for f in 0..500 {
        let (a, b) = (off.step(input(f)), on.step(input(f)));
        assert!(a.picture == b.picture, "frame {f}: the pictures differ");
        assert!(a.paula == b.paula, "frame {f}: the Paula writes differ");
        assert!(off.state() == on.state(), "frame {f}: the states differ");
    }
    assert!(off.scene().is_level(), "the run stayed in the level");
}

/// The mode is not in a save state (DESIGN §3.4): the app sets it, as it sets the view.
#[test]
fn the_mode_is_not_saved() {
    let hash = sf_assets::cache_hash(data());
    let mut g = w1_1();
    for f in 0..50 {
        g.step(input(f));
    }
    let narrow = g.save(BUILD, &hash);
    g.set_widescreen(true);
    assert_eq!(g.save(BUILD, &hash), narrow);
    let loaded = Game::load(data(), &narrow, BUILD, &hash).unwrap();
    assert!(!loaded.widescreen());
}
