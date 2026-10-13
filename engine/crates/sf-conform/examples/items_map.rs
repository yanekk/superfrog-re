//! Scratch (T36): every tile pickup (marker 55, by its world table's handler), hidden coin spring
//! (45), switch (48-51), pad (4, 5) and launcher (7) in each level, with the start marker ($35).
use sf_core::layout::h0;
fn main() {
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let rom = &display.rom().h0;
    let l = |a: u32| { let o = (a - h0(0)) as usize; u32::from_be_bytes([rom[o], rom[o+1], rom[o+2], rom[o+3]]) };
    let name = |h: u32| match h - h0(0) {
        0xed92 => "checkpoint".to_string(), 0xede0 => "coin".into(), 0xee08 => "spud".into(), 0xee38 => "wings".into(),
        0xee6e => "bottle".into(), 0xeec6 => "speed_up".into(), 0xef12 => "slow_down".into(), 0xef40 => "extra_life".into(),
        0xef78 => "shield".into(), 0x7d4 => "nothing".into(), x => format!("fruit${x:x}"),
    };
    let only: Option<String> = std::env::args().nth(1);
    for (id, lv) in &data.levels {
        let ids = id.to_string();
        if only.as_ref().is_some_and(|o| *o != ids) { continue; }
        let world = match ids.as_bytes()[1] { b'B' => 6, b'W' => 5, c => (c - b'1') as u32 };
        let table = l(h0(0xf382) + 4 * world);
        let mut pick = std::collections::BTreeMap::new();
        let mut at = table;
        while (l(at) as i32) >= 0 { pick.insert(l(at), l(at + 4)); at += 8; }
        let t = &lv.t7mp;
        let mut found: std::collections::BTreeMap<String, Vec<(u16, u16)>> = Default::default();
        for y in 0..t.height { for x in 0..t.width {
            let v = t.body[(y * t.width + x) as usize];
            let (tile, m) = ((v & 0x3ff) as u32, v >> 10);
            let k = match m {
                55 => pick.get(&tile).map(|&h| name(h)).unwrap_or(format!("tile{tile}?")),
                45 => "SPRING".into(), 48..=51 => format!("SWITCH{m}"), 4 | 5 => format!("PAD{m}"), 7 => "LAUNCHER".into(),
                0x35 => "START".into(), _ => continue,
            };
            found.entry(k).or_default().push((x, y));
        }}
        println!("{ids}:");
        for (k, v) in &found {
            let shown: Vec<String> = v.iter().take(8).map(|(x, y)| format!("{x},{y}")).collect();
            println!("  {k:14} {:3}: {}", v.len(), shown.join(" "));
        }
    }
}
