//! `Game::step_unseen` (T104), the planners' step without a level frame's picture, leaves the game
//! exactly as `Game::step` does: the whole `Game` (state, scene, every field) and its `probe()`
//! equal on every frame of a route. Here rather than in sf-core: the routes need the disks.
//! Since smart-planner T19 it skips `Scene::step`'s `level_as_drawn` too, save on the frame the
//! level's loop ends (the game keeps that picture for the front end), which the last test covers.

use sf_conform::{InputScript, display_data};
use sf_core::{Game, LevelId, Scene, Start};

/// Plays `script` both ways; returns how many times the level's loop ended (the scene left it).
fn same_both_ways(level: &str, script: &str, frames: u32) -> u32 {
    let id: LevelId = level.parse().unwrap();
    let script = InputScript::load(&sf_conform::case::script_path(script)).unwrap();
    let mut seen = Game::with_display(display_data(), Start::Level(id));
    let mut unseen = seen.clone();
    let mut ended = 0;
    for fr in 0..frames {
        let x = script.input_at(fr);
        let was = seen.scene();
        seen.step(x);
        unseen.step_unseen(x);
        assert_eq!(seen.probe(), unseen.probe(), "{level} frame {fr}: probe");
        assert!(seen == unseen, "{level} frame {fr}: the game differs");
        ended += (was == Scene::Level && seen.scene() == Scene::Front) as u32;
    }
    ended
}

/// W6-1's enemy route: every type the map has, a stomp, a hit, a switch's gate.
#[test]
fn step_unseen_leaves_the_game_as_step_does() {
    same_both_ways("W6-1", "enemies-w6-1", 2150);
}

/// W1-4, the level the smart planner's spike profiled (T00 Q3).
#[test]
fn step_unseen_leaves_the_game_as_step_does_w1_4() {
    same_both_ways("W1-4", "enemies-w1-4", 2000);
}

/// W6-3, the other spike level.
#[test]
fn step_unseen_leaves_the_game_as_step_does_w6_3() {
    same_both_ways("W6-3", "enemies-w6-3", 2000);
}

/// W6-1's completed route, played from `Start::Level` (not the case's sync), loses a life at
/// frames 1545 and 5040: each time the level's loop ends, and the frame it ends in is drawn
/// unseen too, the front end keeping that picture. Skipping it differs at 1545.
#[test]
fn step_unseen_leaves_the_game_as_step_does_where_the_loop_ends() {
    assert_eq!(same_both_ways("W6-1", "enemies-w6-1-complete", 5100), 2);
}
