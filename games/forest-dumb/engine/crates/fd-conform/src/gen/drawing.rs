//! The `drawing` routines' generators (plans/forest-dumb-level-1 T37, DESIGN §4.3), one per
//! `subsystem = "drawing"` entry of `re/translate.toml`, named as there. Each case sets only what its
//! routine reads ([`READS`], checked by `tests/gen_drawing.rs`), and spreads it so every conditional
//! branch of the routine's unit goes both ways on some state, or is named there with why none can.
//! The branch list and what each state reaches are in `docs/11-translation.md`, "Generators: drawing".
//!
//! Every routine here blits or writes the copper list, so they run in hardware mode: the blitter
//! finishes at once (its waits never loop) and VHPOSR steps on each read.

use std::ops::Range;

use super::{Gen, Generator, Setup, h};

pub const ALL: &[(&str, Generator)] = &[
    (
        "h0_21c0",
        Generator {
            base: Some(tiles_and_map),
            case: trigger_cell_tile,
        },
    ),
    (
        "h0_29ca",
        Generator {
            base: Some(tiles_and_map),
            case: item_cell_redraw,
        },
    ),
    (
        "h0_2c7e",
        Generator {
            base: Some(tiles_and_map),
            case: restore_behind_objects,
        },
    ),
    (
        "h0_2d4c",
        Generator {
            base: Some(tiles_and_screen),
            case: cut_objects,
        },
    ),
    (
        "h0_3ab4",
        Generator {
            base: None,
            case: blitter_wait,
        },
    ),
    (
        "char_sprite_position",
        Generator {
            base: None,
            case: char_sprite_position,
        },
    ),
    (
        "h0_5820",
        Generator {
            base: Some(tiles_and_screen),
            case: cut_object,
        },
    ),
    (
        "h0_5e44",
        Generator {
            base: Some(tiles_and_map),
            case: restore_behind_object,
        },
    ),
    (
        "h0_62b6",
        Generator {
            base: Some(tiles_and_map),
            case: animated_cells,
        },
    ),
    (
        "h0_94ba",
        Generator {
            base: None,
            case: palette_fade,
        },
    ),
    (
        "h0_94fe",
        Generator {
            base: None,
            case: palette_fade_step,
        },
    ),
    (
        "h0_9550",
        Generator {
            base: None,
            case: palette_fade_colour,
        },
    ),
    (
        "h0_9754",
        Generator {
            base: None,
            case: camera_to_bitplanes,
        },
    ),
    (
        "char_sprite_copy",
        Generator {
            base: None,
            case: char_sprite_copy,
        },
    ),
];

/// What each routine reads, by the listing (its unit: own flow and the callees not split off): a
/// generator's pokes, hardware reads and base all fall inside these. Program data a routine reads
/// but no generator changes (the tile table h0+$3b988, the creature frame tables h0+$3818-$3a58,
/// the fade table h0+$9624, h4's frames) is left out, so nothing may write there.
pub const READS: &[(&str, &[Range<u32>])] = &[
    ("h0_21c0", &[h(0, 0x21b4)..h(0, 0x21c0), MAP_WORDS, TILES]),
    ("h0_29ca", &[h(0, 0x29c6)..h(0, 0x29ca), MAP_WORDS, TILES]),
    (
        "h0_2c7e",
        &[
            CAMERA_Y,
            OBJ_FLAGS,
            OBJ_SLOTS,
            h(0, 0x5818)..h(0, 0x581e),
            MAP_WORDS,
            TILES,
        ],
    ),
    (
        "h0_2d4c",
        &[
            OBJ_FLAGS,
            OBJ_SLOTS,
            h(0, 0x5818)..h(0, 0x581e),
            SCREEN,
            TILES,
        ],
    ),
    ("h0_3ab4", &[DMACONR]),
    ("char_sprite_position", &[h(0, 0x1e5a)..h(0, 0x1e66)]),
    (
        "h0_5820",
        &[OBJ_SLOTS, h(0, 0x5818)..h(0, 0x581e), SCREEN, TILES],
    ),
    (
        "h0_5e44",
        &[
            CAMERA_Y,
            OBJ_SLOTS,
            h(0, 0x5818)..h(0, 0x581e),
            MAP_WORDS,
            TILES,
        ],
    ),
    (
        "h0_62b6",
        &[
            h(0, 0x1e62)..h(0, 0x1e66),
            h(0, 0x62b2)..h(0, 0x62b4),
            h(0, 0x6342)..h(0, 0x6344),
            h(2, 0x1680)..h(2, 0x1830),
            MAP_BYTES,
            MAP_WORDS,
            TILES,
        ],
    ),
    (
        "h0_94ba",
        &[
            h(0, 0x94b4)..h(0, 0x94b5),
            h(0, 0x94b6)..h(0, 0x94ba),
            FADE_NOW,
            FADE_TO,
        ],
    ),
    ("h0_94fe", &[FADE_NOW, FADE_TO]),
    ("h0_9550", &[]),
    ("h0_9754", &[h(0, 0x1e62)..h(0, 0x1e66)]),
    (
        "char_sprite_copy",
        &[h(0, 0x4ed8)..h(0, 0x4edc), h(0, 0x9b04)..h(0, 0x9b05)],
    ),
];

/// `camera_y`, a word.
const CAMERA_Y: Range<u32> = h(0, 0x1e64)..h(0, 0x1e66);
/// The five creature slots' flags, a byte each.
const OBJ_FLAGS: Range<u32> = h(0, 0x67da)..h(0, 0x67df);
/// The five creature slots, 8 bytes each: x, y, kind (words), facing (byte, sign), frame (byte).
const OBJ_SLOTS: Range<u32> = h(0, 0x67ec)..h(0, 0x6814);
/// The level's tile words, 96 to a column, read two columns of a 16-pixel cell apart.
const MAP_WORDS: Range<u32> = h(0, 0x1_6318)..h(0, 0x1_c318);
/// The cell bytes, 96 to a column (the spike's `MAP`): what h0_62b6 tests before animating a cell.
const MAP_BYTES: Range<u32> = h(0, 0xcb38)..h(0, 0x1_6318);
/// Hunk 3 from the tiles on, the blits' sources: tiles at h3+$13540, creature frames and masks at
/// h3+$20c18 and h3+$2bb18; a source can run a little past the hunk's end.
const TILES: Range<u32> = h(3, 0x1_3540)..h(3, 0x3_aa28);
/// The play screen at h3+$398, $e6 bytes a line, $e600 in all: h0_5820's blits read it as C.
const SCREEN: Range<u32> = h(3, 0x398)..h(3, 0xe998);
/// The copper list's 32 colour words now (h1+$13e) and the target palette (h1+$306), 4 bytes apart.
const FADE_NOW: Range<u32> = h(1, 0x13e)..h(1, 0x1be);
const FADE_TO: Range<u32> = h(1, 0x306)..h(1, 0x386);
/// DMACONR, whose bit 6 (blitter busy) the waits test.
const DMACONR: Range<u32> = 0xdf_f002..0xdf_f004;

/// Hunk 3's blit sources and the screen filled with seeded noise, so a blit's source is never all
/// zero and a wrong source address shows.
fn tiles_and_screen(mem: &mut [u8]) {
    noise(mem, TILES, 0x5EED_0037);
    noise(mem, SCREEN, 0x5EED_0038);
}

/// Hunk 3's blit sources as noise, and the map's words: tile numbers, mostly below $200, some up to
/// $3ff; all within the 1 024 entries of the tile table.
fn tiles_and_map(mem: &mut [u8]) {
    noise(mem, TILES, 0x5EED_0037);
    let mut g = Gen::new(0x5EED_0039);
    for (i, b) in mem[MAP_WORDS.start as usize..MAP_WORDS.end as usize]
        .iter_mut()
        .enumerate()
    {
        *b = if i % 2 == 0 {
            g.below(4) as u8 * g.one_in(4) as u8
        } else {
            g.next() as u8
        };
    }
}

fn noise(mem: &mut [u8], r: Range<u32>, seed: u64) {
    let mut g = Gen::new(seed);
    for b in &mut mem[r.start as usize..r.end as usize] {
        *b = g.next() as u8;
    }
}

/// A level x: mostly inside the map's 128 columns (`$800` pixels), sometimes anywhere.
fn level_x(g: &mut Gen) -> u16 {
    match g.below(10) {
        0 => g.next() as u16,
        _ => g.below(0x800) as u16,
    }
}

/// A level y: mostly inside the map's 96 rows (`$600` pixels), sometimes anywhere. A low byte of
/// $e0-$ff (a creature cut at the screen's foot, h0_5820) and the 16th line of a cell (the screen
/// wrap, h0_21c0, h0_5e44) come up often enough to be reached.
fn level_y(g: &mut Gen) -> u16 {
    let y = match g.below(10) {
        0 => g.next() as u16,
        _ => g.below(0x600) as u16,
    };
    match g.below(8) {
        0 => (y & 0xff00) | 0xe0,
        1 => (y & 0xff00) | 0xe0 | g.below(0x20) as u16,
        2 => y | 0xf0,
        _ => y,
    }
}

/// `$dff002` with the blitter-busy bit clear and the rest anything: a set bit 6 would wait for ever,
/// since the routine oracle's blitter never runs on.
fn blitter_wait(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    if !g.one_in(4) {
        let v = g.next() as u16 & !0x4000;
        s.hw_reads.push((0xdf_f002, (v >> 8) as u8));
        s.hw_reads.push((0xdf_f003, v as u8));
    }
    s
}

/// h0+$5462: the character's position less the camera's, into the four sprite control words.
fn char_sprite_position(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let cx = level_x(g);
    let cy = level_y(g);
    // Near the camera mostly, so the sprite's height bits (bit 8 of y + $31, + $59) go both ways.
    let near = |g: &mut Gen, c: u16, span: u64| (c as u32).wrapping_add(g.below(span) as u32);
    let (px, py) = if g.one_in(5) {
        (g.next() as u32, g.next() as u32)
    } else {
        (near(g, cx, 0x160), near(g, cy, 0x140).wrapping_sub(0x30))
    };
    s.l(h(0, 0x1e5a), px);
    s.l(h(0, 0x1e5e), py);
    s.w(h(0, 0x1e62), cx);
    s.w(h(0, 0x1e64), cy);
    s
}

/// h0+$9754: the camera to the copper list's bitplane pointers and the split line.
fn camera_to_bitplanes(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.w(h(0, 0x1e62), level_x(g));
    s.w(h(0, 0x1e64), level_y(g));
    s
}

/// h0+$94ba: the fade's on flag h0+$94b4 and its step h0+$94b6, 0-$1f in play.
fn palette_fade(g: &mut Gen) -> Setup {
    let mut s = palette_fade_step(g);
    s.b(h(0, 0x94b4), {
        let any = g.next() as u8;
        g.pick(&[0, 0, 1, 0xff, any])
    });
    let step = match g.below(8) {
        0 => 0x1f,
        1 => g.next() as u32,
        _ => g.below(0x20) as u32,
    };
    s.l(h(0, 0x94b6), step);
    s
}

/// h0+$94fe: every colour of the copper list one step toward the target, d6 the step (0-15).
fn palette_fade_step(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[6] = fade_step(g);
    for i in 0..32 {
        let to = g.next() as u16 & 0xfff;
        s.w(FADE_TO.start + i * 4, to);
        s.w(FADE_NOW.start + i * 4, near_colour(g, to));
    }
    s
}

/// h0+$9550: one colour d1 one step toward d0, a2 the fade table, d6 the step.
fn palette_fade_colour(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[6] = fade_step(g);
    let to = g.next() as u16 & 0xfff;
    s.d[0] = (s.d[0] & 0xffff_0000) | to as u32;
    s.d[1] = (s.d[1] & 0xffff_0000) | near_colour(g, to) as u32;
    s.a[2] = h(0, 0x9624);
    s
}

fn fade_step(g: &mut Gen) -> u32 {
    if g.one_in(10) {
        g.next() as u32
    } else {
        g.next() as u32 & !0xffff | g.below(0x10) as u32
    }
}

/// A colour some of whose nibbles equal `to`'s, so each nibble's equal test goes both ways.
fn near_colour(g: &mut Gen, to: u16) -> u16 {
    let mut c = g.next() as u16 & 0xfff;
    for k in 0..3 {
        if g.one_in(3) {
            let m = 0xf << (4 * k);
            c = (c & !m) | (to & m);
        }
    }
    c
}

/// h0+$9b06: the character's frame (640 bytes at `h0+$4ed8`'s pointer) into its four sprites, two
/// frames in four of h0+$9b04's count blanked. The pointer goes into hunk 4's data; the frame
/// itself is never written.
fn char_sprite_copy(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let src = if g.one_in(40) {
        g.next() as u32 & 0xff_fffe
    } else {
        h(4, g.below(0x1_c000) as u32 & !1)
    };
    s.l(h(0, 0x4ed8), src);
    s.b(h(0, 0x9b04), {
        let any = g.next() as u8;
        g.pick(&[0, 0, 1, 2, 3, 0xff, any])
    });
    s
}

/// h0+$29ca: the tile under d0, d1 (level pixels) blitted back to the screen; h0+$29c6 returned.
fn item_cell_redraw(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[0] = s.d[0] & 0xffff_0000 | level_x(g) as u32;
    s.d[1] = s.d[1] & 0xffff_0000 | level_y(g) as u32;
    s.l(h(0, 0x29c6), g.next() as u32);
    s
}

/// h0+$21c0: the trigger cell's count h0+$21bc (0 idle, 1 starts the sound, 2-5 counting, 6 and on
/// redraws the cell's four tiles and stops), the cell at h0+$21b4/$21b8, and h0+$21be choosing
/// which of the two tiles is drawn over it.
fn trigger_cell_tile(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let n = match g.below(10) {
        0 | 1 => 0,
        2 | 3 => 1,
        4 => 2 + g.below(4) as u16,
        5 | 6 => 6,
        7 => 7 + g.below(0x10) as u16,
        _ => g.next() as u16,
    };
    s.w(h(0, 0x21bc), n);
    s.l(h(0, 0x21b4), {
        let v = level_x(g);
        cell_coord(g, v)
    });
    s.l(h(0, 0x21b8), {
        let v = level_y(g);
        cell_coord(g, v)
    });
    s.w(h(0, 0x21be), {
        let any = g.next() as u16;
        g.pick(&[0, 0, 1, any])
    });
    s
}

/// A level coordinate as a long, its high word clear mostly.
fn cell_coord(g: &mut Gen, v: u16) -> u32 {
    if g.one_in(10) {
        (g.next() as u32 & 0xffff_0000) | v as u32
    } else {
        v as u32
    }
}

/// The camera and five creature slots around it: flags (0 empty), x, y, kind (0-7, mostly; 0, 3
/// and 7 are the kinds h0_5e44 redraws more of, 7 the one h0_5820 cuts from hunk 4), facing (the
/// sign of +6) and frame (+7, its bits 2-4 pick the frame).
fn camera_and_slots(g: &mut Gen, s: &mut Setup) -> u16 {
    let cy = level_y(g);
    s.w(CAMERA_Y.start, cy);
    for i in 0..5u32 {
        let flag = if g.one_in(3) {
            0
        } else {
            {
                let any = g.next() as u8;
                g.pick(&[1, 0xff, any | 1])
            }
        };
        s.b(OBJ_FLAGS.start + i, flag);
        let o = OBJ_SLOTS.start + i * 8;
        let (x, y) = object_at(g, cy);
        s.w(o, x);
        s.w(o + 2, y);
        s.w(o + 4, object_kind(g));
        s.b(o + 6, g.next() as u8);
        s.b(o + 7, g.next() as u8);
    }
    cy
}

fn object_kind(g: &mut Gen) -> u16 {
    match g.below(20) {
        0 => g.below(0x10) as u16,
        1..=3 => 0,
        4..=6 => 3,
        7..=9 => 7,
        _ => g.below(8) as u16,
    }
}

/// A creature's x and y: around the camera's y so h0_5e44's tests against `camera_y + $d0` go
/// both ways, with y's low byte $e0-$ff and its 16 line cell boundary each common.
fn object_at(g: &mut Gen, cy: u16) -> (u16, u16) {
    let x = level_x(g);
    let mut y = if g.one_in(6) {
        level_y(g)
    } else {
        cy.wrapping_add(0x80).wrapping_add(g.below(0xa0) as u16)
    };
    match g.below(6) {
        0 => y &= !0xf,
        1 => y = (y & 0xff00) | 0xe0 | g.below(0x20) as u16,
        _ => {}
    }
    (x, y)
}

/// h0+$5e44: one creature's slot h0+$581c, its x and y in h0+$5818/$581a; restores the tiles
/// behind it, more of them for kinds 0, 3 and 7 when it is not on a cell line.
fn restore_behind_object(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let cy = camera_and_slots(g, &mut s);
    // The flags are not read here: h0_2c7e tests them before the call.
    unset(&mut s, &OBJ_FLAGS);
    let (x, y) = object_at(g, cy);
    s.w(h(0, 0x5818), x);
    s.w(h(0, 0x581a), y);
    s.w(h(0, 0x581c), slot(g));
    s
}

/// h0+$5820: one creature cut onto the screen from its frame, clipped at the screen's foot when
/// y's low byte is $e0 or more (and nothing of the second half drawn at exactly $e0).
fn cut_object(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let cy = camera_and_slots(g, &mut s);
    // Neither camera_y nor the flags are read here.
    unset(&mut s, &CAMERA_Y);
    unset(&mut s, &OBJ_FLAGS);
    let (x, y) = object_at(g, cy);
    s.w(h(0, 0x5818), x);
    s.w(h(0, 0x581a), y);
    s.w(h(0, 0x581c), slot(g));
    s
}

/// A slot number: 0-4, rarely anything.
fn slot(g: &mut Gen) -> u16 {
    if g.one_in(50) {
        g.below(0x40) as u16
    } else {
        g.below(5) as u16
    }
}

/// h0+$2c7e: h0_5e44 for each flagged slot.
fn restore_behind_objects(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    camera_and_slots(g, &mut s);
    s
}

/// h0+$2d4c: the blitter's modulos set, then h0_5820 for each flagged slot.
fn cut_objects(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    camera_and_slots(g, &mut s);
    unset(&mut s, &CAMERA_Y);
    s
}

/// Drop the pokes inside `r`: what [`camera_and_slots`] sets that a routine does not read.
fn unset(s: &mut Setup, r: &Range<u32>) {
    s.pokes.retain(|(a, _)| !r.contains(a));
}

/// h0+$62b6: the animated cells. A 2-bit phase h0+$62b3 (stepping h0+$62b2 when it wraps) picks
/// one of four groups of 12 flags at h2+$1680; each flagged entry at h2+$16b0 (x, y, code; 8
/// bytes) is dropped when its map byte is 0 or it is off the camera, else drawn by its code: $20-$27
/// and anything from $80 an animated tile, $28 and $2a two-frame ones ($28 slower when h0+$6342 is
/// set), $30-$33 a bridge that also fills or clears `collision_map`, $34 a two-tile one; any other
/// code ends the routine.
fn animated_cells(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (cx, cy) = (level_x(g), level_y(g));
    s.w(h(0, 0x1e62), cx);
    s.w(h(0, 0x1e64), cy);
    s.b(h(0, 0x62b3), g.next() as u8);
    s.b(h(0, 0x62b2), g.next() as u8);
    s.w(h(0, 0x6342), {
        let any = g.next() as u16;
        g.pick(&[0, 1, any])
    });
    for i in 0..48u32 {
        let on = if g.one_in(3) {
            0
        } else {
            {
                let any = g.next() as u8;
                g.pick(&[1, 0xff, any])
            }
        };
        s.b(h(2, 0x1680) + i, on);
        let e = h(2, 0x16b0) + i * 8;
        // On the camera mostly; off it by each edge sometimes.
        let x = match g.below(8) {
            0 => cx.wrapping_sub(g.below(0x40) as u16),
            1 => cx.wrapping_add(0x110 + g.below(0x40) as u16),
            _ => cx.wrapping_add(g.below(0x120) as u16),
        };
        let y = match g.below(8) {
            0 => cy.wrapping_sub(0x10 + g.below(0x40) as u16),
            1 => cy.wrapping_add(0xb0 + g.below(0x40) as u16),
            _ => cy.wrapping_sub(0x10).wrapping_add(g.below(0xd0) as u16),
        };
        s.w(e, x);
        s.w(e + 2, y);
        let code = match g.below(16) {
            0..=3 => 0x20 + g.below(8) as u8,
            4 | 5 => 0x28,
            6 => 0x2a,
            7 => g.pick(&[0x29, 0x2b, 0x2f, 0x1f]),
            8..=10 => 0x30 + g.below(4) as u8,
            11 | 12 => 0x34,
            13 => g.pick(&[0x35, 0x40, 0x7f]),
            _ => g.next() as u8,
        };
        s.b(e + 4, code);
        // Its map byte, nonzero mostly. A wrapped x or y lands it outside the map, as in play.
        let cell = ((x as u32 >> 4) * 0x60 + (y as u32 >> 4)) & 0xffff;
        let at = MAP_BYTES.start + cell;
        if MAP_BYTES.contains(&at) {
            s.b(at, if g.one_in(5) { 0 } else { g.next() as u8 | 1 });
        }
    }
    s
}
