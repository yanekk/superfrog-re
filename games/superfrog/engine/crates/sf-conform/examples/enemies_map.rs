//! Every marker a world's row of `obj_flag_jumptab` spawns an object for, in each of the world's
//! levels' maps, with up to ten `col,row` each, and the start marker (`$35`), the exit (54) and
//! the coins (8, 9): where to aim an `enemies-*` case.
//!
//! `cargo run --example enemies_map -- <level>...` (e.g. `W4-1 W4-2`).
fn main() {
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let rom = &display.rom().h0;
    let w = |o: u32| u16::from_be_bytes([rom[o as usize], rom[o as usize + 1]]);
    let only: Vec<String> = std::env::args().skip(1).collect();
    for (id, lv) in &data.levels {
        let ids = id.to_string();
        if !only.is_empty() && !only.contains(&ids) {
            continue;
        }
        let world = match ids.as_bytes()[1] {
            b'B' => 6,
            b'W' => 5,
            c => (c - b'1') as u32,
        };
        // A marker spawns when its stub is not the empty one (h0+$17fec).
        let table = 0x178ec + 0x100 * world;
        let spawns = |m: u16| {
            let at = table + 4 * m as u32;
            at + 2 + w(at + 2) as i16 as i32 as u32 != 0x17fec
        };
        let t = &lv.t7mp;
        let mut found: std::collections::BTreeMap<u16, Vec<(u16, u16)>> = Default::default();
        for y in 0..t.height {
            for x in 0..t.width {
                let m = t.cell(x, y) >> 10;
                if spawns(m) || matches!(m, 0x35 | 54) {
                    found.entry(m).or_default().push((x, y));
                }
            }
        }
        println!("{ids}:");
        for (m, v) in &found {
            let shown: Vec<String> = v.iter().take(10).map(|(x, y)| format!("{x},{y}")).collect();
            println!("  marker {m:2} {:3}: {}", v.len(), shown.join(" "));
        }
    }
}
