//! `Game::step_unseen` (T104), the planners' step without a level frame's picture, leaves the game
//! exactly as `Game::step` does: the whole `Game` (state, scene, every field) and its `probe()`
//! equal on every frame of a W6 route. Here rather than in sf-core: the route needs the disks.

use sf_conform::{InputScript, display_data};
use sf_core::{Game, LevelId, Start};

fn same_both_ways(level: &str, script: &str, frames: u32) {
    let id: LevelId = level.parse().unwrap();
    let script = InputScript::load(&sf_conform::case::script_path(script)).unwrap();
    let mut seen = Game::with_display(display_data(), Start::Level(id));
    let mut unseen = seen.clone();
    for fr in 0..frames {
        let x = script.input_at(fr);
        seen.step(x);
        unseen.step_unseen(x);
        assert_eq!(seen.probe(), unseen.probe(), "{level} frame {fr}: probe");
        assert!(seen == unseen, "{level} frame {fr}: the game differs");
    }
}

/// W6-1's enemy route: every type the map has, a stomp, a hit, a switch's gate.
#[test]
fn step_unseen_leaves_the_game_as_step_does() {
    same_both_ways("W6-1", "enemies-w6-1", 2150);
}
