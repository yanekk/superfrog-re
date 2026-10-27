//! What `tests/guide.rs` and `tests/repair.rs` share: the explorer's level start (as
//! `tests/explore.rs` makes it) and T08's engine-timed W1-1 fixture.
#![allow(dead_code)]

use std::path::Path;

use sf_core::input::FrameInput;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::{Facts, facts, landed};

pub fn id(s: &str) -> LevelId {
    s.parse().unwrap()
}

/// `id` entered as the original enters it, idled until the frog stands (frame 151), and its facts.
pub fn at_start(id: LevelId) -> (Game, Facts) {
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Level(id));
    while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
        g.step_unseen(FrameInput::default());
    }
    let fa = facts(&g, &data.levels[&id], display.rom());
    (g, fa)
}

pub fn threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get())
}

/// The engine-timed W1-1 fixture's inputs, from `at_start`.
pub fn fixture() -> Vec<FrameInput> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/w1-1-engine.txt");
    sf_plan::trace::script_inputs(&sf_conform::InputScript::load(&p).unwrap())
}
