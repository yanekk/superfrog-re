//! `core::facts` against the disks' levels, and `core::graph::shortest` on a hand-built graph.

use sf_core::frog::{FROG, f};
use sf_core::{Game, LevelId, Start};
use sf_plan::core::*;

/// The 24 normal levels, W1-1 … W6-4 (no alternates, no bonus level, no witch).
fn normal_levels() -> Vec<LevelId> {
    let ids: Vec<LevelId> = sf_conform::game_data()
        .levels
        .keys()
        .copied()
        .filter(|id| !id.alt && (1..=6).any(|w| id.to_string().starts_with(&format!("W{w}-"))))
        .collect();
    assert_eq!(ids.len(), 24, "{ids:?}");
    ids
}

/// `id` entered as the original enters it, and its facts there (the map is in memory from frame 0).
fn at_start(id: LevelId) -> (Game, Facts) {
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let game = Game::with_display(display.clone(), Start::Level(id));
    let fa = facts(&game, &data.levels[&id], display.rom());
    (game, fa)
}

fn id(s: &str) -> LevelId {
    s.parse().unwrap()
}

#[test]
fn w1_1_needs_its_ninety_coins() {
    let (game, fa) = at_start(id("W1-1"));
    assert_eq!(fa.level, id("W1-1"));
    // rust-engine T106: W1-1's exit needs 90; at load the count is the marker-8 cells (T104).
    assert_eq!(fa.coins_needed, 90);
    let marker8 = fa.coins.iter().filter(|c| c.kind == CoinKind::Object(8)).count();
    assert_eq!(marker8, 90);
    let objects = fa.coins.iter().filter(|c| matches!(c.kind, CoinKind::Object(_))).count();
    assert!(objects >= fa.coins_needed as usize);
    // The frog starts standing on the map.
    assert!(landed(game.state()));
    assert!(fa.standable.contains(&frog_cell(game.state())), "{:?}", frog_cell(game.state()));
}

#[test]
fn every_level_has_an_exit_ground_and_gated_switches() {
    for id in normal_levels() {
        let (_, fa) = at_start(id);
        assert!(!fa.exits.is_empty(), "{id}: no exit");
        assert!(!fa.standable.is_empty(), "{id}: nothing standable");
        assert!(fa.coins.len() >= fa.coins_needed as usize, "{id}: fewer coins than needed");
        for s in &fa.switches {
            // The gate is a single cell of the switch's marker, apart from the pair.
            assert!(!s.cells.contains(&s.gate), "{id}: {s:?}");
            assert_eq!(s.cells[1], (s.cells[0].0, s.cells[0].1 + 1), "{id}: {s:?}");
            assert!((48..=51).contains(&s.marker), "{id}: {s:?}");
        }
        let hazards: std::collections::BTreeSet<Cell> = fa.hazards.iter().copied().collect();
        assert!(fa.standable.iter().all(|c| !hazards.contains(c)), "{id}: a hazard is standable");
    }
}

#[test]
fn w6_4_spikes_at_row_10_are_hazards() {
    let (_, fa) = at_start(id("W6-4"));
    // rust-engine FINDINGS T150: 24 spikes at row 10, crossed only with wings.
    let row10: Vec<Cell> = fa.hazards.iter().copied().filter(|c| c.1 == 10).collect();
    assert_eq!(row10.len(), 24, "{row10:?}");
    assert!(fa.standable.iter().all(|c| !fa.hazards.contains(c)));
    // Its marker 51 opens nothing (`marker_cells` 0) and is no switch.
    assert!(fa.switches.iter().all(|s| s.marker != 51), "{:?}", fa.switches);
}

#[test]
fn frog_cell_is_the_old_planners_formula() {
    let (mut game, _) = at_start(id("W1-1"));
    for (x, y) in [(0x00a0u16, 0x02a0u16), (0x1234, 0x0345), (0x0009, 0x0010), (0x12a7, 0x053f)] {
        game.poke(FROG + f::X, &x.to_be_bytes());
        game.poke(FROG + f::Y, &y.to_be_bytes());
        // plan_exit's `frog()`, then `/ 16`.
        let old = ((x as i16 as i32 + 0x17) / 16, (y as i16 as i32 - 0x10) / 16);
        let cell = frog_cell(game.state());
        assert_eq!((cell.0 as i32, cell.1 as i32), old, "x {x:#x} y {y:#x}");
    }
}

fn edge(from: u32, to: u32, frames: u32, needs_switches: u32, needs_wings: bool) -> Edge {
    Edge { from, to, inputs: vec![(0, frames as u16)], frames, coins: Vec::new(), needs_switches, needs_wings }
}

#[test]
fn shortest_honours_switches_and_wings() {
    let nodes = (0..5u16).map(|k| Node { cell: (k, 0), wings: false }).collect();
    // 0→1 10 frames; 1→4 behind switch bit 1 (5 frames); 0→4 the long way round, 100 frames;
    // 0→2 with wings, 3 frames; 2→3 1 frame; 3 has no other way in.
    let map = MoveMap::new(
        nodes,
        vec![
            edge(1, 4, 5, 0b10, false),
            edge(0, 1, 10, 0, false),
            edge(0, 4, 100, 0, false),
            edge(0, 2, 3, 0, true),
            edge(2, 3, 1, 0, false),
        ],
    );
    assert_eq!(map.out.len(), 5);
    assert!(map.edges.windows(2).all(|w| w[0].from <= w[1].from));
    let m = u32::MAX;
    assert_eq!(shortest(&map, 0, 0, false), vec![0, 10, m, m, 100]);
    assert_eq!(shortest(&map, 0, 0b10, false), vec![0, 10, m, m, 15]);
    // A switch set that lacks the bit the edge needs (bit 0 only) does not open it.
    assert_eq!(shortest(&map, 0, 0b01, false), vec![0, 10, m, m, 100]);
    assert_eq!(shortest(&map, 0, 0b11, true), vec![0, 10, 3, 4, 15]);
    // From a node with no way out, only itself.
    assert_eq!(shortest(&map, 3, 0b11, true), vec![m, m, m, 0, m]);
}
