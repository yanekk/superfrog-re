//! Every tile pickup (marker 55, named by its world table's handler), hidden coin spring (45),
//! switch (48-51), pad (4, 5) and launcher (7) in each level's map, and its start marker ($35),
//! with up to eight `col,row` each: where to aim an `items-*` case.
//!
//! `cargo run --example items_map -- [level]` (every level, or the one named, e.g. `W2-2`).
#[path = "common/mod.rs"]
mod common;
use sf_core::layout::h0;

/// The world pickup tables' table (h0).
const PICKUP_TABLES: u32 = 0xf382;
fn main() {
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let rom = &display.rom().h0;
    let l = |a: u32| {
        let o = (a - h0(0)) as usize;
        u32::from_be_bytes([rom[o], rom[o + 1], rom[o + 2], rom[o + 3]])
    };
    let name = |h: u32| common::pickup(h - h0(0));
    let only: Option<String> = std::env::args().nth(1);
    for (id, lv) in &data.levels {
        let ids = id.to_string();
        if only.as_ref().is_some_and(|o| *o != ids) {
            continue;
        }
        let world = match ids.as_bytes()[1] {
            b'B' => 6,
            b'W' => 5,
            c => (c - b'1') as u32,
        };
        let table = l(h0(PICKUP_TABLES) + 4 * world);
        let mut pick = std::collections::BTreeMap::new();
        let mut at = table;
        while (l(at) as i32) >= 0 {
            pick.insert(l(at), l(at + 4));
            at += 8;
        }
        let t = &lv.t7mp;
        let mut found: std::collections::BTreeMap<String, Vec<(u16, u16)>> = Default::default();
        for y in 0..t.height {
            for x in 0..t.width {
                let v = t.body[(y * t.width + x) as usize];
                let (tile, m) = ((v & 0x3ff) as u32, v >> 10);
                let k = match m {
                    55 => pick
                        .get(&tile)
                        .map(|&h| name(h))
                        .unwrap_or(format!("tile{tile}?")),
                    45 => "SPRING".into(),
                    48..=51 => format!("SWITCH{m}"),
                    4 | 5 => format!("PAD{m}"),
                    7 => "LAUNCHER".into(),
                    0x35 => "START".into(),
                    _ => continue,
                };
                found.entry(k).or_default().push((x, y));
            }
        }
        println!("{ids}:");
        for (k, v) in &found {
            let shown: Vec<String> = v.iter().take(8).map(|(x, y)| format!("{x},{y}")).collect();
            println!("  {k:14} {:3}: {}", v.len(), shown.join(" "));
        }
    }
}
