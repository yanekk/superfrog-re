//! The spike's generators (`games/forest-dumb/spikes/muse-spark/judge/src/main.rs`, 2026-11-04),
//! carried over unchanged in what they set, named as in `re/symbols.toml` or by address. What each
//! routine reads is the spike's reading of the listing, not a measurement: a generator task that
//! takes one of these routines over checks it against the listing and its paths, and moves it out
//! of here: T37 took `char_sprite_position` and `h0+$9754` (now `h0_9754`) into `drawing.rs`.

use super::{Gen, Generator, Setup, h};

pub const ALL: &[(&str, Generator)] = &[
    (
        "h0+$24ce",
        Generator {
            base: None,
            case: cell_under_player,
        },
    ),
    (
        "h0+$2dd6",
        Generator {
            base: None,
            case: objects_off_screen,
        },
    ),
    (
        "h0+$2fae",
        Generator {
            base: None,
            case: objects_move,
        },
    ),
    (
        "h0+$774",
        Generator {
            base: Some(screens),
            case: tile_blit,
        },
    ),
    (
        "redraw_edge_a",
        Generator {
            base: Some(screens_and_map),
            case: map_edge,
        },
    ),
    (
        "redraw_edge_b",
        Generator {
            base: Some(screens_and_map),
            case: map_edge,
        },
    ),
    (
        "h0+$3adc",
        Generator {
            base: None,
            case: Setup::random,
        },
    ),
    (
        // `h0+$3b2a`, `char_hit_check`: the name is T38's generator (`objects.rs`).
        "h0+$3b2a",
        Generator {
            base: None,
            case: char_hit_check,
        },
    ),
    (
        "h4+$18870",
        Generator {
            base: None,
            case: music_channel_tick,
        },
    ),
];

/// h0: a byte map, 0x60 cells to a column.
const MAP: u32 = 0xcb38;

/// A coordinate: mostly in a level's range, sometimes anything.
fn coord(g: &mut Gen) -> u32 {
    match g.below(10) {
        0..=2 => g.below(0x200) as u32,
        3..=8 => g.below(0x1400) as u32,
        _ => g.next() as u32,
    }
}

fn map_cell(x: u32, y: u32) -> u32 {
    (((x >> 4) & 0xffff) * 0x60).wrapping_add(y >> 4)
}

fn cell_value(g: &mut Gen) -> u8 {
    match g.below(4) {
        0 => 0,
        1 => g.pick(&[0x10, 0x11, 0x12, 0x13]),
        _ => g.next() as u8,
    }
}

/// Five objects of 8 bytes at h0+$67ec, flagged in h0+$67da, around the camera.
fn objects(g: &mut Gen, s: &mut Setup, types: u64) {
    let cx = 0x20 + g.below(0x1000) as u16;
    let cy = 0x30 + g.below(0x800) as u16;
    s.w(h(0, 0x1e62), cx);
    s.w(h(0, 0x1e64), cy);
    s.l(h(0, 0x1e5a), cx as u32 + g.below(0x140) as u32);
    s.l(h(0, 0x1e5e), cy as u32 + g.below(0xe0) as u32);
    for i in 0..5u32 {
        let flag = if g.one_in(3) {
            0
        } else {
            g.pick(&[1, 1, 0xff, 7])
        };
        s.b(h(0, 0x67da) + i, flag);
        let o = h(0, 0x67ec) + i * 8;
        let x = (cx as i32 + g.below(0x200) as i32 - 0x60) as u16;
        let y = (cy as i32 + g.below(0x180) as i32 - 0x60) as u16;
        s.w(o, x);
        s.w(o + 2, y);
        let kind = if g.one_in(10) {
            g.next() as u16
        } else {
            g.below(types) as u16
        };
        s.w(o + 4, kind);
        s.b(o + 6, g.next() as u8);
        let (any, low) = (g.next() as u8, g.below(0x20) as u8);
        s.b(o + 7, g.pick(&[0, 0x1e, 0x1f, 0x20, any, low]));
        // The cells an object lands on and around.
        let c = map_cell(x as u32, y as u32);
        for d in [0u32, 1, 0x60, 0x61, 0xc0, 0xc1, 0x5f, 0xbf] {
            let at = h(0, MAP).wrapping_add(c).wrapping_add(d).wrapping_sub(0x60);
            s.b(at, cell_value(g));
        }
    }
}

/// h0+$24ce: the map cell under the player, trigger codes $10-$13.
fn cell_under_player(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let v = if g.one_in(5) { 0 } else { g.next() as u8 | 1 };
    s.b(h(0, 0x4ed0), v);
    let v = if g.one_in(5) { g.next() as u16 | 1 } else { 0 };
    s.w(h(0, 0x4b62), v);
    let (x, y) = (coord(g), coord(g));
    s.l(h(0, 0x1e5a), x);
    s.l(h(0, 0x1e5e), y);
    let c = map_cell(x.wrapping_add(0xe), y.wrapping_add(0x23));
    s.b(h(0, MAP).wrapping_add(c), cell_value(g));
    for a in [0x4ecf, 0x41b6, 0x21b4, 0x21b8, 0x21bc, 0x21be] {
        s.b(h(0, a), g.next() as u8);
    }
    s
}

/// h0+$2dd6: five objects leaving the screen written into the map.
fn objects_off_screen(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    objects(g, &mut s, 8);
    s
}

/// h0+$2fae: five objects' per-type movement.
fn objects_move(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    objects(g, &mut s, 9);
    // The collision bytes the objects read, from h3+$fa50.
    for _ in 0..64 {
        let off = g.below(0x2000) as u32;
        let v = if g.one_in(2) { 0 } else { g.next() as u8 };
        s.b(h(3, 0xfa50) + off, v);
    }
    s
}

/// h0+$774: a tile blit, d5 the tile, a3 where it goes (a screen in hunk 3).
fn tile_blit(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[5] = if g.one_in(8) {
        g.next() as u32
    } else {
        g.below(0x300) as u32
    };
    s.a[3] = h(3, g.below(0x3_0000) as u32 & !1);
    s
}

/// redraw_edge_a, redraw_edge_b: the camera's scroll position in h0+$1e66/$1e68.
fn map_edge(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.w(h(0, 0x1e66), g.below(0x900) as u16);
    let x = if g.one_in(8) {
        g.next() as u16
    } else {
        g.below(0x600) as u16
    };
    s.w(h(0, 0x1e68), x);
    s
}

/// h0+$3b2a: the collision register, then the objects against the player.
fn char_hit_check(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let clx = g.next() as u16;
    s.hw_reads.push((0xdf_f00e, (clx >> 8) as u8));
    s.hw_reads.push((0xdf_f00f, clx as u8));
    objects(g, &mut s, 9);
    let v = if g.one_in(5) { 1 } else { 0 };
    s.w(h(0, 0x4b62), v);
    let v = if g.one_in(5) { 0 } else { 1 };
    s.b(h(0, 0x4ed0), v);
    for a in [0x3b26, 0x3b28, 0x4ecf] {
        s.b(h(0, a), g.next() as u8);
    }
    s
}

/// h4+$18870: one channel of the music player, a5 its Paula registers, a6 its $2c-byte state.
fn music_channel_tick(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let n = g.below(4) as u32;
    s.a[5] = 0xdf_f0a0 + n * 0x10;
    let st = h(4, 0x1_9498 + n * 0x2c);
    s.a[6] = st;
    let state: Vec<u8> = (0..0x2c).map(|_| g.next() as u8).collect();
    s.put(st, state);
    // The command word: a note's period bits, an effect 0-f, its parameter.
    let cmd = g.below(16) as u16;
    let note = if g.one_in(3) {
        0
    } else {
        g.below(0x1000) as u16
    };
    s.w(st + 2, (note & 0xf000) | cmd << 8 | g.below(256) as u16);
    if g.one_in(6) {
        s.w(st + 2, 0);
    }
    s.w(st + 0x10, 0x71 + g.below(0x2e8) as u16);
    let v = if g.one_in(2) { 0 } else { g.next() as u8 };
    s.b(st + 0x1f, v);
    s.l(st + 4, h(4, g.below(0x1_0000) as u32 & !1));
    s.l(st + 0xa, h(4, g.below(0x1_0000) as u32 & !1));
    s.b(h(4, 0x1_95c9), g.below(7) as u8);
    s.b(h(4, 0x1_95c8), 1 + g.below(6) as u8);
    s
}

/// Hunk 3 (chip BSS: the screens and tiles) filled with seeded noise, so a blit's source is never
/// all zero.
fn screens(mem: &mut [u8]) {
    let mut g = Gen::new(0x5EED);
    for b in &mut mem[h(3, 0) as usize..h(3, 0x3_aa28) as usize] {
        *b = g.next() as u8;
    }
}

/// [`screens`], and the map's words at h0+$16318: tile numbers, mostly below $100.
fn screens_and_map(mem: &mut [u8]) {
    screens(mem);
    let mut g = Gen::new(0x5EED_0001);
    for (i, b) in mem[h(0, 0x1_6318) as usize..h(0, 0x1_c318) as usize]
        .iter_mut()
        .enumerate()
    {
        *b = if i % 2 == 0 {
            g.one_in(8) as u8
        } else {
            g.below(0x400) as u8
        };
    }
}
