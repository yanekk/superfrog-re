//! `level_init`'s first view (plans/forest-dumb-level-1 T65), written by hand: `view_tiles`
//! (`h0+$62e`) copies the 23 by 16 cells from the map's top-left onto the play screen, a cell at a time
//! by `view_tile` (`h0+$672`, row `d6`, column `d7`); `view_shapes` (`h0+$6c4`) copies the same cells'
//! shapes into `collision_map` by `view_shape` (`h0+$708`). None has a branch but its loop's, and
//! callers pass rows 0 to 15, columns 0 to 22 and their own bases in `a3` to `a5` only, so a state
//! varies what the cells hold: the tile words (mostly the tiles hunk 3 holds, some any up to
//! [`TILE_LIMIT`]) and the shape bytes (mostly the 108 shapes before the object map, some anything,
//! which read on into it).
//!
//! The tiles, the shapes, the screen and `collision_map` are noise in the base, so a wrong source or
//! destination shows.

use std::ops::Range;

use super::{Gen, Generator, Setup, h};

pub const ALL: &[(&str, Generator)] = &[
    ("view_tiles", entry(tiles_all)),
    ("view_tile", entry(tile_one)),
    ("view_shapes", entry(shapes_all)),
    ("view_shape", entry(shape_one)),
];

const fn entry(case: fn(&mut Gen) -> Setup) -> Generator {
    Generator {
        base: Some(base),
        case,
    }
}

/// The tiles, `$a0` bytes each from h3+$13540, as far as the largest tile word reaches.
const TILES: Range<u32> = h(3, 0x1_3540)..h(3, 0x3_aa28);
/// The play screen, `$e6` bytes a line.
const SCREEN: Range<u32> = h(3, 0x398)..h(3, 0xe998);
/// `collision_map` and the slack after it, to the tiles.
const COLLISION: Range<u32> = h(3, 0xfa50)..h(3, 0x1_3540);
/// `_GenN-1.Mapa`'s shapes before the object map.
const SHAPES: Range<u32> = h(0, 0xbda0)..h(0, 0xcb38);
/// The tile map (`_MapaN-1`, a word a cell, `$c0` bytes a column) and the shape map (`_MapaN-1.Mapa`,
/// a byte a cell, `$60` a column).
const TILE_MAP: u32 = h(0, 0x1_6318);
const SHAPE_MAP: u32 = h(0, 0x2_8f18);

/// Tiles from `$90e` on are read from `$80000` up, past the oracle's 512 KB of chip RAM, which there
/// reads back the chip RAM from 0: a later cell's tile can then read the screen an earlier cell drew,
/// which the rig's flat memory does not model (measured, T65: 4 bytes of one state in 10 000). The
/// level's tiles stop at `$3ed`.
const TILE_LIMIT: u32 = (0x8_0000 - TILES.start) / 0xa0;

const COLUMNS: u32 = 0x17;
const ROWS: u32 = 0x10;

fn noise(mem: &mut [u8], r: Range<u32>, seed: u64) {
    let mut g = Gen::new(seed);
    for b in &mut mem[r.start as usize..r.end as usize] {
        *b = g.next() as u8;
    }
}

fn base(mem: &mut [u8]) {
    noise(mem, TILES, 0x5EED_0650);
    noise(mem, SCREEN, 0x5EED_0651);
    noise(mem, COLLISION, 0x5EED_0652);
    noise(mem, SHAPES, 0x5EED_0653);
}

/// A tile word: mostly below `$3ed` (the tiles hunk 3 holds), sometimes to `$3ff`, sometimes any up
/// to [`TILE_LIMIT`].
fn tile(g: &mut Gen) -> u16 {
    match g.below(20) {
        0 => g.below(TILE_LIMIT as u64) as u16,
        1 | 2 => g.below(0x400) as u16,
        _ => g.below(0x3ed) as u16,
    }
}

/// A shape byte: mostly below `$6c`, sometimes any.
fn shape(g: &mut Gen) -> u8 {
    if g.one_in(8) {
        g.next() as u8
    } else {
        g.below(0x6c) as u8
    }
}

/// A cell, row `d6` and column `d7`, anywhere in the view, its corners common.
fn cell(g: &mut Gen) -> (u32, u32) {
    if g.one_in(4) {
        (g.pick(&[0, ROWS - 1]), g.pick(&[0, COLUMNS - 1]))
    } else {
        (g.below(ROWS as u64) as u32, g.below(COLUMNS as u64) as u32)
    }
}

/// The tile words of every cell in the view, a column (16 words) a poke.
fn tiles_all(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    for c in 0..COLUMNS {
        let words: Vec<u8> = (0..ROWS).flat_map(|_| tile(g).to_be_bytes()).collect();
        s.put(TILE_MAP + c * 0xc0, words);
    }
    s
}

/// One cell's tile word, `d6` and `d7` its row and column; `a3` to `a5` the screen, the tile map and
/// the tiles, as `view_tiles` passes them.
fn tile_one(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (r, c) = cell(g);
    s.d[6] = r;
    s.d[7] = c;
    s.a[3] = SCREEN.start;
    s.a[4] = TILE_MAP;
    s.a[5] = TILES.start;
    s.w(TILE_MAP + c * 0xc0 + r * 2, tile(g));
    s
}

/// The shape bytes of every cell in the view, a column a poke.
fn shapes_all(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    for c in 0..COLUMNS {
        let bytes: Vec<u8> = (0..ROWS).map(|_| shape(g)).collect();
        s.put(SHAPE_MAP + c * 0x60, bytes);
    }
    s
}

/// One cell's shape byte; `a3` to `a5` `collision_map`, the shape map and the shapes, as
/// `view_shapes` passes them.
fn shape_one(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (r, c) = cell(g);
    s.d[6] = r;
    s.d[7] = c;
    s.a[3] = COLLISION.start;
    s.a[4] = SHAPE_MAP;
    s.a[5] = SHAPES.start;
    s.b(SHAPE_MAP + c * 0x60 + r, shape(g));
    s
}
