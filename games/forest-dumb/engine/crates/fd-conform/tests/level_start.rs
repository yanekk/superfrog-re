//! A game started on its own (`Game::new(Start::Level1)`, plans/forest-dumb-level-1 T65) holds what
//! `level_init` drew before play, as the original holds it at the level sync point: `collision_map`
//! byte for byte over what `CollisionPlane` keeps, and the play screen as the first view's tiles
//! leave it. Also what else is set there: the creature slots and animated cells
//! (docs/12-level-start-draw.md).

use fd_conform::Live;
use fd_conform::sync::reach_level_1;
use fd_core::level::collision::{BEFORE, CollisionPlane, KEPT, MAP};
use fd_core::level::redraw::Blit;
use fd_core::{Game, Mem, Start};

fn at_sync() -> amiga_oracle::Amiga {
    reach_level_1(1).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_new_games_collision_map_is_the_originals_at_the_sync_point() {
    let am = at_sync();
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let g = Game::new(data, Start::Level1);
    let theirs = Live(&am).read(3, MAP - BEFORE, (BEFORE + KEPT) as usize);
    let differ: Vec<u32> = CollisionPlane::kept_offsets()
        .zip(&theirs)
        .filter(|&(at, &b)| g.plane().kept_byte(at) != b)
        .map(|(at, _)| at)
        .collect();
    assert!(
        differ.is_empty(),
        "{} byte(s) differ, first at collision_map{:+#x}",
        differ.len(),
        differ.first().map_or(0, |&a| a as i32)
    );
    // Not a comparison of two empty planes: the first view has ground.
    assert!(theirs.iter().filter(|&&b| b != 0).count() > 100);
}

#[test]
fn the_first_views_tiles_are_the_play_screen_at_the_sync_point() {
    // Each listed tile copied as `view_tile` copies it, from the original's own tiles, onto a screen
    // of zeros, must give the original's play screen's 16 bands.
    let am = at_sync();
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let g = Game::new(data, Start::Level1);
    let live = Live(&am);
    let theirs = live.read(3, 0x398, 16 * 0xe60);
    let mut ours = vec![0u8; theirs.len()];
    assert_eq!(g.blits().len(), 23 * 16);
    for b in g.blits() {
        let Blit::Tile { tile, dst } = *b else {
            panic!("a bob in the first view: {b:?}")
        };
        let src = live.read(3, 0x1_3540 + tile as u32 * 0xa0, 0xa0);
        for (k, w) in src.chunks(2).enumerate() {
            let (line, plane) = (k as u32 / 5, k as u32 % 5);
            let at = (dst + line * 0xe6 + plane * 0x2e) as usize;
            ours[at..at + 2].copy_from_slice(w);
        }
    }
    let differ = ours.iter().zip(&theirs).filter(|(a, b)| a != b).count();
    assert_eq!(differ, 0, "{differ} screen byte(s) differ");
}

#[test]
fn no_creature_or_animated_cell_is_set_at_the_sync_point() {
    // Measured 2026-11-06 (docs/12-level-start-draw.md): `level_init` starts none, and no edge
    // redraw has run before the first pass, so the engine's empty tables are the original's.
    let am = at_sync();
    let live = Live(&am);
    let slots = live.read(0, 0x67da, 5);
    let cells = live.read(2, 0x1680, 48);
    println!("creature_active {slots:x?}\nanim_cell_used {cells:x?}");
    assert!(slots.iter().all(|&b| b == 0), "creature_active {slots:x?}");
    assert!(cells.iter().all(|&b| b == 0), "anim_cell_used {cells:x?}");
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let g = Game::new(data, Start::Level1);
    assert_eq!(g.state().edges.creatures.active, [0; 5]);
    assert_eq!(g.state().edges.cells.used, [[0; 12]; 4]);
}
