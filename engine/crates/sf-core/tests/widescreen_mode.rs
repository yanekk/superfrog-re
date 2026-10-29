//! real-widescreen T01 (DESIGN §2.1, §3.4): the widescreen mode reaches every level step, is
//! read there each step, and is not saved.

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

/// The mode is read each step, not latched: a game switched on and back off before it steps is
/// the 4:3 game, state, picture and sound, frame for frame.
#[test]
fn widescreen_switched_off_again_is_the_original() {
    let (mut off, mut back) = (w1_1(), w1_1());
    back.set_widescreen(true);
    back.set_widescreen(false);
    for f in 0..500 {
        let (a, b) = (off.step(input(f)), back.step(input(f)));
        assert!(a.picture == b.picture, "frame {f}: the pictures differ");
        assert!(a.paula == b.paula, "frame {f}: the Paula writes differ");
        assert!(off.state() == back.state(), "frame {f}: the states differ");
    }
    assert!(off.scene().is_level(), "the run stayed in the level");
}

/// From T02 the mode reaches the spawn and despawn edges (DESIGN §2.3): a W1-1 run with it on
/// wakes something the 4:3 run has not, so the states part, and the game plays on in the level.
#[test]
fn widescreen_on_changes_the_game_from_t02() {
    let (mut off, mut on) = (w1_1(), w1_1());
    on.set_widescreen(true);
    let mut parted = None;
    for f in 0..500 {
        off.step(input(f));
        on.step(input(f));
        if parted.is_none() && off.state() != on.state() {
            parted = Some(f);
        }
    }
    assert!(parted.is_some(), "widescreen changed nothing in 500 frames");
    assert!(
        on.scene().is_level(),
        "the widescreen run stayed in the level"
    );
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
