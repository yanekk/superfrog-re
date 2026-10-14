//! Every marker of a level's world that spawns an enemy or scenery (anything but the items every
//! world shares: coins 8, 9, the exit lock 29, the hidden coin spring 45), with its count and up
//! to eight `col,row` each; then the start (`$35`) and the exit (54). Where to aim an `enemies-*`
//! case.
//!
//! `cargo run --example enemies_map -- [level]` (every level, or the one named, e.g. `W2-1`).
fn main() {
    let data = sf_conform::game_data();
    let only: Option<String> = std::env::args().nth(1);
    let d = sf_core::level::standard_dispatch();
    for (id, lv) in &data.levels {
        let ids = id.to_string();
        if only.as_ref().is_some_and(|o| *o != ids) {
            continue;
        }
        let world = match ids.as_bytes()[1] {
            b'B' => 6,
            b'W' => 5,
            c => c - b'1',
        };
        let t = &lv.t7mp;
        let mut found: std::collections::BTreeMap<u16, Vec<(u16, u16)>> = Default::default();
        for y in 0..t.height {
            for x in 0..t.width {
                let m = t.body[(y * t.width + x) as usize] >> 10;
                let shown = match m {
                    8 | 9 | 29 | 45 => false,
                    0x35 | 54 => true,
                    _ => d.marker(world, m as u8).is_some() && m != 0,
                };
                if shown {
                    found.entry(m).or_default().push((x, y));
                }
            }
        }
        println!("{ids}:");
        for (m, v) in &found {
            let shown: Vec<String> = v.iter().take(8).map(|(x, y)| format!("{x},{y}")).collect();
            let what = match m {
                0x35 => "START".to_string(),
                54 => "EXIT".to_string(),
                m => format!("marker {m}"),
            };
            println!("  {what:10} {:3}: {}", v.len(), shown.join(" "));
        }
    }
}
