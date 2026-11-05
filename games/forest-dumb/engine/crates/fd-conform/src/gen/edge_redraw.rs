//! The edge redraws' generators (plans/forest-dumb-level-1 T56, DESIGN §4.3): one per `gen_task =
//! "T56"` entry of `re/translate.toml`, named as there. `redraw_edge_a..d` run from `main_loop` when
//! the camera crosses a 16-pixel boundary: each draws one row (a, b) or column (c, d) of tiles onto the
//! play screen (`h0_774`), the same row or column of collision shapes into `collision_map`
//! (`h0_7cc`), then scans the object map's cells just outside that edge, 20 across or 13 down. A cell
//! of 0 is skipped, `$ff` ends the scan, 1-6 and `$87`-`$fe` start a creature in a free slot
//! (`h0_681c`, `h0_6852`) and clear the cell, `$20`-`$3f` add an animated cell (`h0_6882`), `$40`-`$86`
//! blit a bob (`h0_a06`, `h0_ce4`, `h0_104c`, `h0_1390`, one per edge), 7-`$1f` are skipped.
//! `h0_3ac2` waits for the raster to leave lines `$e7`-`$66` and for the blitter.
//!
//! [`READS`] is what each routine reads when it runs, callees included (checked by
//! `tests/gen_edge_redraw.rs`); every generator writes inside it only. The program's own tables
//! (`h0+$1496` screen-band pointers, `h0+$18d6` collision-row pointers, the bobs at `h4+$c80` and
//! `h4+$2580`) are left as the program has them. The branch list and what the states reach are in
//! `docs/11-translation.md`, "Generators: edge redraws".
//!
//! `h0+$1e66`/`$1e68` are the position the redraws draw at (x, y, words), not `camera_x`/`camera_y`
//! (`h0+$1e62`/`$1e64`); the spike read them as the camera's scroll position.

use std::ops::Range;

use super::{Gen, Generator, Setup, h};

pub const ALL: &[(&str, Generator)] = &[
    ("h0_774", entry(Some(tiles), tile_blit)),
    ("h0_7cc", entry(Some(shapes), shape_copy)),
    ("redraw_edge_a", entry(Some(level), edge_a)),
    ("h0_a06", entry(Some(screen), bob_a)),
    ("redraw_edge_b", entry(Some(level), edge_b)),
    ("h0_ce4", entry(Some(screen), bob_b)),
    ("redraw_edge_c", entry(Some(level), edge_c)),
    ("h0_104c", entry(Some(screen), bob_c)),
    ("redraw_edge_d", entry(Some(level), edge_d)),
    ("h0_1390", entry(Some(screen), bob_d)),
    ("h0_3ac2", entry(None, blitter_wait)),
    ("h0_681c", entry(None, free_slot)),
    ("h0_6852", entry(None, fill_slot)),
    ("h0_6882", entry(None, add_animated_cell)),
];

const fn entry(base: Option<fn(&mut [u8])>, case: fn(&mut Gen) -> Setup) -> Generator {
    Generator { base, case }
}

/// The edge redraws' x and y, words.
const POS: Range<u32> = h(0, 0x1e66)..h(0, 0x1e6a);
const POS_X: u32 = h(0, 0x1e66);
const POS_Y: u32 = h(0, 0x1e68);
/// Hunk 3 from the tiles on: h0_774's blit source (tile `d5` at h3+$13540 + `d5` * $a0).
const TILES: Range<u32> = h(3, 0x1_3540)..h(3, 0x3_aa28);
/// The play screen at h3+$398, $e6 bytes a line, 256 lines: the bob blits read it as C.
const SCREEN: Range<u32> = h(3, 0x398)..h(3, 0xe998);
/// `_MapaN-1`, the tile words, 96 to a column; W6's 61 440 bytes.
const TILE_MAP: Range<u32> = h(0, 0x1_6318)..h(0, 0x2_5318);
/// `_MapaN-1.Mapa`, the collision shape bytes, 96 to a column; W6's 30 720 bytes.
const SHAPE_MAP: Range<u32> = h(0, 0x2_8f18)..h(0, 0x3_0718);
/// `_GenN-1.Mapa`, 32 bytes a shape. h0_7cc indexes it by a byte, so it reads up to h0+$dda0, past
/// the 108 shapes that fit before the object map into the object map itself.
const SHAPES: Range<u32> = h(0, 0xbda0)..h(0, 0xdda0);
/// The part of [`SHAPES`] before the object map, which a base fills.
const SHAPES_OWN: Range<u32> = h(0, 0xbda0)..h(0, 0xcb38);
/// `_MapaBobowN`, the object map: a byte a cell, 96 to a column.
const OBJ_MAP: Range<u32> = h(0, 0xcb38)..h(0, 0x1_6318);
/// The five creature slots' flags, a byte each (0 free).
const OBJ_FLAGS: Range<u32> = h(0, 0x67da)..h(0, 0x67df);
/// The animated cells' four groups of 12 flags (h2+$1680); h0_6882 fills the emptiest group.
const CELL_FLAGS: Range<u32> = h(2, 0x1680)..h(2, 0x16b0);
/// DMACONR, whose bit 6 (blitter busy) h0_3ac2 tests.
const DMACONR: Range<u32> = 0xdf_f002..0xdf_f004;

/// What each routine reads when it runs, callees included: a generator's base, pokes and hardware
/// reads all fall inside these. VHPOSR is read too, but steps by itself and is never set.
pub const READS: &[(&str, &[Range<u32>])] = &[
    ("h0_774", &[TILES, DMACONR]),
    ("h0_7cc", &[SHAPES]),
    ("redraw_edge_a", EDGE_READS),
    ("h0_a06", BOB_READS),
    ("redraw_edge_b", EDGE_READS),
    ("h0_ce4", BOB_READS),
    ("redraw_edge_c", EDGE_READS),
    ("h0_104c", BOB_READS),
    ("redraw_edge_d", EDGE_READS),
    ("h0_1390", BOB_READS),
    ("h0_3ac2", &[DMACONR]),
    ("h0_681c", &[OBJ_FLAGS]),
    ("h0_6852", &[]),
    ("h0_6882", &[CELL_FLAGS, OBJ_MAP]),
];

const EDGE_READS: &[Range<u32>] = &[
    POS, TILES, SCREEN, TILE_MAP, SHAPE_MAP, SHAPES, OBJ_MAP, OBJ_FLAGS, CELL_FLAGS, DMACONR,
];
const BOB_READS: &[Range<u32>] = &[POS, OBJ_MAP, SCREEN, DMACONR];

fn noise(mem: &mut [u8], r: Range<u32>, seed: u64) {
    let mut g = Gen::new(seed);
    for b in &mut mem[r.start as usize..r.end as usize] {
        *b = g.next() as u8;
    }
}

/// The tiles as noise, so a wrong source address shows.
fn tiles(mem: &mut [u8]) {
    noise(mem, TILES, 0x5EED_0056);
}

/// The shapes as noise, so a wrong shape number shows.
fn shapes(mem: &mut [u8]) {
    noise(mem, SHAPES_OWN, 0x5EED_0057);
}

/// The screen as noise: the bob blits cut over it.
fn screen(mem: &mut [u8]) {
    noise(mem, SCREEN, 0x5EED_0058);
}

/// A level for the redraws: tiles, shapes and screen as noise; tile words mostly below $3ed (the
/// tiles hunk 3 holds), some up to $3ff; shape bytes mostly below $6c (the shapes before the object
/// map), some anything. The object map is set per state, only at the cells a redraw scans.
fn level(mem: &mut [u8]) {
    tiles(mem);
    shapes(mem);
    screen(mem);
    let mut g = Gen::new(0x5EED_0059);
    let words = &mut mem[TILE_MAP.start as usize..TILE_MAP.end as usize];
    for w in words.as_chunks_mut::<2>().0 {
        let t = if g.one_in(16) {
            g.below(0x400)
        } else {
            g.below(0x3ed)
        } as u16;
        w.copy_from_slice(&t.to_be_bytes());
    }
    for b in &mut mem[SHAPE_MAP.start as usize..SHAPE_MAP.end as usize] {
        *b = if g.one_in(8) {
            g.next() as u8
        } else {
            g.below(0x6c) as u8
        };
    }
}

/// The redraws' x: mostly inside a level (W6 is $1400 pixels wide; `camera_x` clamps at $17cf),
/// sometimes 0 and the first cell (h0_df6's `x >= 1` guard), sometimes $8000 (the only x that passes
/// that guard and fails its `tst.w`, its N flag set either way), sometimes anything.
fn pos_x(g: &mut Gen) -> u16 {
    match g.below(20) {
        0 => 0,
        1 => g.below(0x20) as u16,
        2 => 0x8000,
        3 => g.next() as u16,
        _ => g.below(0x17d0) as u16,
    }
}

/// The redraws' y: mostly inside a level (`camera_y` clamps at $53f), and around each guard: y 0-8
/// (redraw_edge_b's `y >= 1` and `y - 8 >= 0`), $530-$54f (redraw_edge_a's `y < $53c`), $8000-$853f
/// (where `cmpi.w #$53c` and `cmpi.w #1` see the sign, not the size), sometimes anything.
fn pos_y(g: &mut Gen) -> u16 {
    match g.below(20) {
        0 | 1 => g.below(9) as u16,
        2 | 3 => 0x530 + g.below(0x20) as u16,
        4 => 0x8000 + g.below(0x540) as u16,
        5 => g.next() as u16,
        _ => g.below(0x540) as u16,
    }
}

/// One object-map cell's code, weighted over what the redraws tell apart: 0 (skip), $ff (stop),
/// 1 (a creature whose kind is 0, `y + 2`), 2-6 and $87-$fe (a creature), 7-$1f (skip), $20-$3f
/// (an animated cell), $40-$86 (a bob), with each edge of those ranges common.
fn cell_code(g: &mut Gen) -> u8 {
    match g.below(40) {
        0..=13 => 0,
        14 => 0xff,
        15 | 16 => 1,
        17..=19 => 2 + g.below(5) as u8,
        20 | 21 => 0x87 + g.below(0x78) as u8,
        22 => g.pick(&[7, 0x1f, 0x86, 0x87, 0xfe, 0x20, 0x3f, 0x40, 6]),
        23 | 24 => 7 + g.below(0x19) as u8,
        25..=29 => 0x20 + g.below(0x20) as u8,
        30..=35 => 0x40 + g.below(0x47) as u8,
        _ => g.next() as u8,
    }
}

/// The five slot flags: each free or taken, all taken one state in six (h0_681c's `d0 = -1`).
fn slot_flags(g: &mut Gen, s: &mut Setup) {
    let full = g.one_in(6);
    let free = g.below(4);
    for i in 0..5u32 {
        let f = if !full && g.below(4) < free {
            0
        } else {
            let any = g.next() as u8;
            g.pick(&[0xff, 1, any | 1])
        };
        s.b(OBJ_FLAGS.start + i, f);
    }
}

/// The four groups of 12 animated-cell flags: each group empty, sparse, nearly full or full, and
/// every group full one state in six (h0_6882 then finds no free entry and adds nothing).
fn cell_flags(g: &mut Gen, s: &mut Setup) {
    let full = g.one_in(6);
    for grp in 0..4u32 {
        let fill = if full { 12 } else { g.pick(&[0, 3, 8, 11, 12]) };
        for i in 0..12u32 {
            let v = if g.below(12) < fill {
                let any = g.next() as u8;
                g.pick(&[0xff, 1, any | 1])
            } else {
                0
            };
            s.b(CELL_FLAGS.start + grp * 12 + i, v);
        }
    }
}

/// The object-map byte at `cell` (an offset from the map's start, as the routine computes it, so a
/// wrapped one lands where the routine reads): a code when inside the map.
fn put_cell(g: &mut Gen, s: &mut Setup, cell: u32) {
    let at = OBJ_MAP.start.wrapping_add(cell);
    if OBJ_MAP.contains(&at) {
        s.b(at, cell_code(g));
    }
}

/// A redraw's state: its x and y, the slot and cell flags, and the object-map cells it scans,
/// given as cell offsets by `cells(x, y)`.
fn edge(g: &mut Gen, cells: fn(u16, u16) -> Vec<u32>) -> Setup {
    let mut s = Setup::random(g);
    let (x, y) = (pos_x(g), pos_y(g));
    s.w(POS_X, x);
    s.w(POS_Y, y);
    slot_flags(g, &mut s);
    cell_flags(g, &mut s);
    for c in cells(x, y) {
        put_cell(g, &mut s, c);
    }
    s
}

/// The cells redraw_edge_a and _b scan: 20 columns from x's, on row `row`.
fn across(x: u16, row: u32) -> Vec<u32> {
    let col = (x >> 4) as u32 * 0x60;
    (0..20)
        .map(|k| (col + k * 0x60).wrapping_add(row))
        .collect()
}

/// The cells redraw_edge_c and _d scan: 13 rows from y's, in column `col`.
fn down(col: u16, y: u16) -> Vec<u32> {
    let base = (col >> 4) as u32 * 0x60 + (y >> 4) as u32;
    (0..13).map(|k| base + k).collect()
}

/// h0+$844: the row below the view, y + $c0.
fn edge_a(g: &mut Gen) -> Setup {
    edge(g, |x, y| across(x, (y.wrapping_add(0xc0) >> 4) as u32))
}

/// h0+$b20: the row above the view, y - 8.
fn edge_b(g: &mut Gen) -> Setup {
    edge(g, |x, y| across(x, (y.wrapping_sub(8) >> 4) as u32))
}

/// h0+$df6: the column at x.
fn edge_c(g: &mut Gen) -> Setup {
    edge(g, down)
}

/// h0+$114a: the column at x + $130.
fn edge_d(g: &mut Gen) -> Setup {
    edge(g, |x, y| down(x.wrapping_add(0x130), y))
}

/// h0+$774: tile `d5` (a word: `mulu.w`) blitted to `a3`, a screen column at some band.
fn tile_blit(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let t = match g.below(10) {
        0 => g.next() as u32,
        1 => g.next() as u32 & !0xffff | g.below(0x400) as u32,
        _ => g.below(0x3ed) as u32,
    };
    s.d[5] = t;
    s.a[3] = SCREEN.start + (g.below((SCREEN.end - SCREEN.start - 0xe60) as u64) as u32 & !1);
    s
}

/// h0+$7cc: shape `d5` (a long, shifted by 5) copied as 16 words to `a3`, a `collision_map` column.
fn shape_copy(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[5] = match g.below(10) {
        0 => g.below(0x100) as u32,
        _ => g.below(0x6c) as u32,
    };
    s.a[3] = h(3, 0xfa50 + (g.below(0x2e00 - 0x2e0) as u32 & !1));
    s
}

/// A bob helper's state: x, y, `d4` the scan's counter (0-$13 across, 0-$c down; sometimes
/// anything), `a2` an object-map cell holding a bob code mostly ($40-$86), sometimes any code.
fn bob(g: &mut Gen, n: u64) -> Setup {
    let mut s = Setup::random(g);
    s.w(POS_X, pos_x(g));
    s.w(POS_Y, pos_y(g));
    s.d[4] = if g.one_in(10) {
        g.next() as u32
    } else {
        g.below(n) as u32
    };
    let cell = OBJ_MAP.start + g.below((OBJ_MAP.end - OBJ_MAP.start) as u64) as u32;
    s.a[2] = cell;
    let code = if g.one_in(5) {
        g.next() as u8
    } else {
        0x40 + g.below(0x47) as u8
    };
    s.b(cell, code);
    s
}

/// h0+$a06: a bob on the row below the view.
fn bob_a(g: &mut Gen) -> Setup {
    bob(g, 0x14)
}

/// h0+$ce4: a bob on the row above the view.
fn bob_b(g: &mut Gen) -> Setup {
    bob(g, 0x14)
}

/// h0+$104c: a bob in the column at x.
fn bob_c(g: &mut Gen) -> Setup {
    bob(g, 0xd)
}

/// h0+$1390: a bob in the column at x + $130.
fn bob_d(g: &mut Gen) -> Setup {
    bob(g, 0xd)
}

/// h0+$3ac2: `$dff002` with the blitter-busy bit clear and the rest anything (a set bit 6 would wait
/// for ever: the oracle's blitter never runs on). The raster half reads VHPOSR, which steps.
fn blitter_wait(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    if !g.one_in(4) {
        let v = g.next() as u16 & !0x4000;
        s.hw_reads.push((0xdf_f002, (v >> 8) as u8));
        s.hw_reads.push((0xdf_f003, v as u8));
    }
    s
}

/// h0+$681c: the first free slot of five takes d0-d3; none free returns -1.
fn free_slot(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    slot_flags(g, &mut s);
    s
}

/// h0+$6852: slot `d7` (0-4; rarely up to $3f, past the five) takes x d0, y d1, kind d2, facing d3.
fn fill_slot(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[7] = if g.one_in(50) {
        g.below(0x40) as u32
    } else {
        g.below(5) as u32
    };
    s
}

/// h0+$6882: an animated cell at d0, d1 with code `(a2)` added to the emptiest group's first free
/// entry.
fn add_animated_cell(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    cell_flags(g, &mut s);
    let cell = OBJ_MAP.start + g.below((OBJ_MAP.end - OBJ_MAP.start) as u64) as u32;
    s.a[2] = cell;
    let code = if g.one_in(4) {
        g.next() as u8
    } else {
        0x20 + g.below(0x20) as u8
    };
    s.b(cell, code);
    s
}
