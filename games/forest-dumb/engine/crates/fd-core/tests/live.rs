//! `fd_core::live` on made-up RAM (plans/forest-dumb-map-page T06): play by `irq_count` moving, the
//! world, the character's spot, BCD items left, the active creature slots, the cells that differ
//! from the level file, and the ranges that cover every byte the reader takes.

use fd_core::live::{Live, live_from_ram, live_ranges};
use fd_core::{HUNK_BASES, layout_var};

const SLOW_BASE: usize = 0xC0_0000;
const MAP: u32 = 0xcb38;
const MAP_BYTES: usize = 400 * 96;

struct Ram {
    chip: Vec<u8>,
    slow: Vec<u8>,
}

impl Ram {
    fn new() -> Ram {
        Ram {
            chip: vec![0; 512 << 10],
            slow: vec![0; 512 << 10],
        }
    }

    fn byte(&mut self, a: usize, b: u8) {
        match a.checked_sub(SLOW_BASE) {
            Some(i) => self.slow[i] = b,
            None => self.chip[a] = b,
        }
    }

    fn put(&mut self, name: &str, value: u32) {
        let v = layout_var(name).unwrap_or_else(|| panic!("{name}"));
        let at = (HUNK_BASES[v.hunk as usize] + v.offset) as usize;
        let n = v.width.bytes();
        for k in 0..n {
            self.byte(at + k, (value >> (8 * (n - 1 - k))) as u8);
        }
    }

    /// The object map's cell (`col`, `row`).
    fn cell(&mut self, col: usize, row: usize, b: u8) {
        self.byte((HUNK_BASES[0] + MAP) as usize + col * 96 + row, b);
    }

    fn map(&mut self, cells: &[u8]) {
        for (i, &b) in cells.iter().enumerate() {
            self.cell(i / 96, i % 96, b);
        }
    }

    fn live(&self, prev: Option<u16>, start: &dyn Fn(u8) -> Option<Vec<u8>>) -> Live {
        live_from_ram(&self.chip, &self.slow, prev, start)
    }
}

/// A level file's map with a coin at 3,4 and a creature (value 5) at 10,20.
fn level_file() -> Vec<u8> {
    let mut m = vec![0; MAP_BYTES];
    m[3 * 96 + 4] = 0x20;
    m[10 * 96 + 20] = 5;
    m
}

/// World 3 in play: the character at (`$70`, `$70`), 272 items left, slots 1 and 3 active, the coin
/// taken and the creature written back one cell to the right.
fn world_3() -> Ram {
    let mut r = Ram::new();
    r.put("irq_count", 41);
    r.put("world_index", 2);
    r.put("char_x", 0x70);
    r.put("char_y", 0x70);
    r.put("items_left", 0x272);
    r.put("creature_active+$1", 0xff);
    r.put("creature_slots+$8", 200 << 16 | 336);
    r.put("creature_slots+$c", 4 << 16 | 0x01_00);
    r.put("creature_active+$3", 0xff);
    r.put("creature_slots+$18", 0xfff0 << 16 | 48);
    r.put("creature_slots+$1c", 7 << 16);
    // Slot 2 holds a stale creature but is free: not shown.
    r.put("creature_slots+$10", 1 << 16 | 1);
    let mut m = level_file();
    m[3 * 96 + 4] = 0;
    m[10 * 96 + 20] = 0;
    m[11 * 96 + 20] = 5;
    r.map(&m);
    r
}

#[test]
fn nothing_is_played_before_irq_count_moves() {
    let none = |_: u8| -> Option<Vec<u8>> { panic!("no map is read when not playing") };
    let r = world_3();
    let first = r.live(None, &none);
    assert_eq!(
        first,
        Live {
            irq: 41,
            ..Live::default()
        },
        "the first sample is not play"
    );
    // Standing still, not at 0: the game over and SCENA screens once a level was played.
    assert!(!r.live(Some(41), &none).playing);
    // All zero RAM, as before the title.
    let z = Ram::new().live(Some(0), &none);
    assert!(!z.playing && z.world.is_none() && z.char_xy.is_none() && z.items_left.is_none());
}

#[test]
fn world_3_in_play_reads_whole() {
    let r = world_3();
    let asked = std::cell::Cell::new(0);
    let start = |w: u8| {
        asked.set(w);
        Some(level_file())
    };
    let l = r.live(Some(40), &start);
    assert_eq!(
        asked.get(),
        3,
        "the level file asked for is world_index + 1's"
    );
    assert_eq!(
        l,
        Live {
            playing: true,
            world: Some(3),
            char_xy: Some((0x70 + 0xe, 0x70 + 9)),
            items_left: Some(272),
            creatures: vec![(200, 336, 4), (-16, 48, 7)],
            changed: vec![(3, 4, 0), (10, 20, 0), (11, 20, 5)],
            irq: 41,
        }
    );
    // `irq_count` wrapped past `$ffff` still moved.
    let mut w = world_3();
    w.put("irq_count", 0);
    assert!(w.live(Some(0xffff), &start).playing);
}

#[test]
fn a_world_with_no_level_file_has_no_changed_cells() {
    let l = world_3().live(Some(40), &|_| None);
    assert!(l.playing && l.changed.is_empty() && l.char_xy.is_some());
}

#[test]
fn a_nonsense_world_index_names_no_real_world() {
    let mut r = world_3();
    r.put("world_index", 0x1_0000);
    assert_eq!(r.live(Some(40), &|_| None).world, Some(255));
}

#[test]
fn items_left_decodes_bcd() {
    let mut r = world_3();
    for (bcd, n) in [(0x282, 282), (0x0, 0), (0x9, 9), (0x100, 100)] {
        r.put("items_left", bcd);
        assert_eq!(r.live(Some(40), &|_| None).items_left, Some(n), "{bcd:x}");
    }
}

/// Every byte the reader takes lies in `live_ranges`: RAM with only those bytes copied reads the
/// same, and filling everything outside them with noise changes nothing.
#[test]
fn the_ranges_cover_every_byte_read() {
    let full = world_3();
    let start = |_: u8| Some(level_file());
    let want = full.live(Some(40), &start);
    let mut copy = Ram {
        chip: vec![0xa5; 512 << 10],
        slow: vec![0x5a; 512 << 10],
    };
    let ranges = live_ranges();
    for &(a, b) in &ranges {
        assert!(a < b, "{a:x}..{b:x}");
        for at in a..b {
            let at = at as usize;
            let v = match at.checked_sub(SLOW_BASE) {
                Some(i) => full.slow[i],
                None => full.chip[at],
            };
            copy.byte(at, v);
        }
    }
    assert_eq!(copy.live(Some(40), &start), want);
    // Sorted and apart, so the app's copy is a few slices; the map is one of them.
    assert!(ranges.windows(2).all(|w| w[0].1 < w[1].0), "{ranges:x?}");
    let map = HUNK_BASES[0] + MAP;
    assert!(
        ranges
            .iter()
            .any(|&(a, b)| a <= map && map + MAP_BYTES as u32 <= b)
    );
    let bytes: u32 = ranges.iter().map(|(a, b)| b - a).sum();
    assert!(bytes < 40_000, "{bytes} bytes: the map and a few variables");
}
