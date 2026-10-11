//! Tiles and palettes against `tools/render_levels.py`: the same level drawn by both is the same
//! picture, compared as the SHA-1 of its RGB bytes (4800 × 1344 × 3).
//!
//! Python is given the level, the world's `LnBM` and the chunk it takes the palette from, and runs
//! its own `decode_tiles`, `base_palette` and `render_level`. For W1–W5 that palette chunk is the
//! one its `associations()` picks (the world's `LnLP`). For W6 and the bonus and final levels its
//! `associations()` picks wrongly (W6: the L2ET picture, read two bytes out of line; LB and LW: W6's
//! tiles), so those worlds are compared on their tiles alone, with a test palette.

mod common;

use common::{chunk, data, named};
use sf_assets::*;
use sf_conform::reference::python_render_levels;
use sf_disk::sha1_hex;

/// The level drawn as `render_levels.py` draws it: each cell's tile (`cell & 0x3ff`) in the
/// palette's colours (nibble × 17), black where the index is past the tileset.
fn render(t: &T7mp, tiles: &Tileset, palette: &Palette) -> (u32, u32, String) {
    let (w, h) = (t.width as usize * 16, t.height as usize * 16);
    let mut rgb = vec![0u8; w * h * 3];
    for row in 0..t.height as usize {
        for col in 0..t.width as usize {
            let idx = (t.body[row * t.width as usize + col] & 0x3ff) as usize;
            let Some(tile) = tiles.tiles.get(idx) else {
                continue;
            };
            for y in 0..16 {
                for x in 0..16 {
                    let o = ((row * 16 + y) * w + col * 16 + x) * 3;
                    rgb[o..o + 3].copy_from_slice(&palette.rgb888(tile[y * 16 + x]));
                }
            }
        }
    }
    (w as u32, h as u32, sha1_hex(&rgb))
}

#[test]
fn a_level_of_each_world_1_to_6_renders_as_the_python_tool_renders_it() {
    let d = data();
    // One level per world draws with all it has: its tiles and its palette. The other levels'
    // maps are proven by levels.rs (they round-trip), so drawing them too proves nothing more.
    let ids: Vec<LevelId> = World::ALL[..6]
        .iter()
        .map(|&world| LevelId {
            world,
            level: 2,
            alt: false,
        })
        .collect();
    let lp = |w: World| chunk(named(&format!("L{}LP", w.code()), 0));
    let bm = |w: World| chunk(named(&format!("L{}BM", w.code()), 0));
    let jobs: Vec<(&[u8], &[u8], &[u8])> = ids
        .iter()
        .map(|id| (chunk(d.levels[id].chunk), bm(id.world), lp(id.world)))
        .collect();
    let python = python_render_levels(&jobs).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(python.len(), ids.len());
    for (id, py) in ids.iter().zip(&python) {
        let w = d.world(id.world);
        assert_eq!(
            render(&d.levels[id].t7mp, &w.tiles, &w.palette),
            *py,
            "{id}"
        );
    }
}

/// Every tile of every tileset, in order, on one map, in a palette with 32 different colours
/// (black at 0 and the grey ramp at 26..=29, which `base_palette` looks for).
#[test]
fn every_tileset_decodes_as_the_python_tool_decodes_it() {
    let d = data();
    let mut colours = [0u16; 32];
    for (k, c) in colours.iter_mut().enumerate() {
        *c = (k as u16 * 0x0137 + 0x0f0) & 0xfff;
    }
    colours[0] = 0;
    colours[26..30].copy_from_slice(&[0x111, 0x222, 0x444, 0xaaa]);
    let palette = Palette(colours);
    let mut distinct = colours.to_vec();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 32);
    let pal_src: Vec<u8> = colours.iter().flat_map(|c| c.to_be_bytes()).collect();

    let mut maps = Vec::new();
    for w in &d.worlds {
        let mut t = d.levels.values().next().unwrap().t7mp.clone();
        let n = w.tiles.tiles.len() as u16;
        for (k, cell) in t.body.iter_mut().enumerate() {
            *cell = k as u16 % n;
        }
        maps.push((w.world, t7mp_encode(&t), t));
    }
    let jobs: Vec<(&[u8], &[u8], &[u8])> = maps
        .iter()
        .map(|(w, bytes, _)| {
            (
                &bytes[..],
                chunk(named(&format!("L{}BM", w.code()), 0)),
                &pal_src[..],
            )
        })
        .collect();
    let python = python_render_levels(&jobs).unwrap_or_else(|e| panic!("{e}"));
    for ((w, _, t), py) in maps.iter().zip(&python) {
        assert_eq!(render(t, &d.world(*w).tiles, &palette), *py, "{w:?} tiles");
    }
    assert_eq!(python.len(), 8);
}

/// The tileset's own encoding: decoding then encoding `LnBM` gives its bytes back.
#[test]
fn tilesets_encode_back() {
    for w in &data().worlds {
        assert_eq!(
            w.tiles.encode(),
            chunk(named(&format!("L{}BM", w.world.code()), 0)),
            "{:?}",
            w.world
        );
    }
}
