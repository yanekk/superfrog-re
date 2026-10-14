//! W6's enemy markers (12–20), the witch's (1) and the start (`$35`) and exit (54) in each W6 and
//! final level's map, with up to twelve `col,row` each: where to aim an `enemies-w6-*` case.
//!
//! `cargo run --release --example w6_map -- [level]`
fn main() {
    let data = sf_conform::game_data();
    let only: Option<String> = std::env::args().nth(1);
    for (id, lv) in &data.levels {
        let ids = id.to_string();
        if !(ids.starts_with("W6") || ids == "WW-1")
            || only.as_ref().is_some_and(|o| *o != ids)
        {
            continue;
        }
        let t = &lv.t7mp;
        let mut found: std::collections::BTreeMap<u16, Vec<(u16, u16)>> = Default::default();
        for y in 0..t.height {
            for x in 0..t.width {
                let m = t.body[(y * t.width + x) as usize] >> 10;
                if matches!(m, 1 | 12..=20 | 48..=51 | 0x35 | 54 | 0x39) {
                    found.entry(m).or_default().push((x, y));
                }
            }
        }
        println!("{ids}:");
        for (k, v) in &found {
            let shown: Vec<String> = v.iter().take(12).map(|(x, y)| format!("{x},{y}")).collect();
            println!("  {k:3} {:3}: {}", v.len(), shown.join(" "));
        }
    }
}
