//! Diagnostic: a level's start, exits, switches and extents. `facts_dump <level>...`
use sf_core::input::FrameInput;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::{facts, frog_cell, landed};

fn main() {
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    for a in std::env::args().skip(1) {
        let id: LevelId = a.parse().unwrap();
        let mut g = Game::with_display(display.clone(), Start::Level(id));
        while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
            g.step_unseen(FrameInput::default());
        }
        let fa = facts(&g, &data.levels[&id], display.rom());
        println!("{id}: start {:?} frame {}", frog_cell(g.state()), g.frame());
        println!("  exits {:?}", fa.exits);
        println!("  switches {:?}", fa.switches);
        let xs: Vec<_> = fa.standable.iter().map(|c| c.0).collect();
        println!(
            "  hazards {} standable {} cols {}..{} coins {} need {}",
            fa.hazards.len(),
            fa.standable.len(),
            xs.iter().min().unwrap(),
            xs.iter().max().unwrap(),
            fa.coins.len(),
            fa.coins_needed
        );
    }
}
