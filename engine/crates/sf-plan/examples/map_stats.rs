//! Builds the movement map of each level named and prints its size, cost and reach (T04's FINDINGS
//! row). `cargo run --release -p sf-plan --example map_stats -- [--threads N] W1-1 W1-2 ...`
//! (no level: all 24 normal levels).

use std::time::Instant;

use sf_core::input::FrameInput;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::map::{node_of, reach};
use sf_plan::core::{CoinKind, default_moves, facts, landed};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut threads = 10;
    if let Some(k) = args.iter().position(|a| a == "--threads") {
        threads = args[k + 1].parse().unwrap();
        args.drain(k..k + 2);
    }
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let levels: Vec<LevelId> = if args.is_empty() {
        data.levels
            .keys()
            .copied()
            .filter(|id| !id.alt && (1..=6).any(|w| id.to_string().starts_with(&format!("W{w}-"))))
            .collect()
    } else {
        args.iter().map(|a| a.parse().unwrap()).collect()
    };
    for id in levels {
        let mut g = Game::with_display(display.clone(), Start::Level(id));
        while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
            g.step_unseen(FrameInput::default());
        }
        let fa = facts(&g, &data.levels[&id], display.rom());
        let t0 = Instant::now();
        let (map, stats) = sf_plan::threads::build_threaded(&g, &fa, &default_moves(true), threads);
        let secs = t0.elapsed().as_secs_f64();
        let start = node_of(&map, g.state()).unwrap();
        let all = (1u32 << fa.switches.len()) - 1;
        let r = reach(&map, &fa, start, all, true);
        let r0 = reach(&map, &fa, start, 0, true);
        let objects = r
            .coins
            .iter()
            .filter(|&&k| matches!(fa.coins[k as usize].kind, CoinKind::Object(_)))
            .count();
        let total_obj = fa.coins.iter().filter(|c| matches!(c.kind, CoinKind::Object(_))).count();
        println!(
            "{id}: {secs:.1}s threads {threads}, nodes {} keys {} edges {} layers {} frames {} rejected {:?}; switches {} \
             reach nodes {} object coins {objects}/{total_obj} need {} exit {} (no switch: exit {} coins {})",
            map.nodes.len(),
            stats.keys,
            map.edges.len(),
            stats.layers,
            stats.frames,
            stats.rejected,
            fa.switches.len(),
            r.nodes,
            fa.coins_needed,
            r.exit,
            r0.exit,
            r0.coins.len(),
        );
        println!("  keys per switch set {:?}", stats.per_switch_set);
    }
}
