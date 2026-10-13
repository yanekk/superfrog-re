//! Scratch (T36): what a script makes the frog take and set off, run on the engine from a case's
//! sync point (or a level code: `coverage <case.toml|W2-1> [script] [frames]`).
use sf_conform::{Case, SyncPoint, sync, InputScript};
use sf_conform::freeze::Freeze;
use sf_core::layout::{h0, LAYOUT, VarId};
use sf_core::frog::{FROG, f};
use sf_core::{Game, Start};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (sync_pt, script, frames) = if args[0].ends_with(".toml") {
        let c = Case::load(std::path::Path::new(&args[0])).unwrap();
        (c.sync, c.script, c.frames)
    } else {
        let s = InputScript::load(&sf_conform::case::script_path(&args[1])).unwrap();
        (SyncPoint::AtLevel(args[0].parse().unwrap()), s, args[2].parse().unwrap())
    };
    let synced = sync::reach_frozen(&sync_pt, &script, frames + 10, Freeze::Enemies).unwrap();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Transplant(synced.memory));
    let map = sf_core::layout::def(VarId::LevelMap);
    let rows = |g: &Game| g.state().l(h0(0x17334));
    let markers = |g: &Game| -> Vec<u8> { g.state().var(VarId::LevelMap).chunks(2).map(|c| c[1] & 0x3f).collect() };
    let mut prev = markers(&g);
    let w = |g: &Game, o: u32| g.state().w(FROG + o);
    let watch = [("spud", f::SPUD_ON), ("flight", f::FLIGHT), ("shield", f::SHIELD), ("maxspeed", f::MAX_SPEED), ("lives", f::LIVES), ("launch", f::LAUNCHED), ("spring", f::SPRING)];
    let mut pw: Vec<u16> = watch.iter().map(|x| w(&g, x.1)).collect();
    let kinds = |g: &Game| -> Vec<u32> { (0..20).map(|k| { let s = h0(0x1ba6c) + k * 0x7c; if g.state().l(s + 0x40) != 0 { g.state().l(s + 4) - h0(0) } else { 0 } }).collect() };
    let mut pk = kinds(&g);
    let base = rows(&g);
    let _ = (map, LAYOUT);
    for fr in 0..frames {
        g.step(script.input_at(fr));
        let m = markers(&g);
        let first_row = h0(0) ; let _ = first_row;
        for (k, (a, b)) in prev.iter().zip(&m).enumerate() {
            if a != b {
                // cell address → col,row relative to the map's row 0 (map_rows[1])
                let addr = map.addr + 2 * k as u32;
                let row0 = sf_core::layout::h1(0x218e0 + 0x4c0);
                let d = addr as i64 - row0 as i64;
                let tile = g.state().w(addr) >> 6;
                println!("{fr}: marker {a} -> {b} at {},{} {}", (d % 0x260) / 2, d / 0x260, if *a == 55 { kind(&g, tile.wrapping_sub(1) as u32) } else { String::new() });
            }
        }
        let cw: Vec<u16> = watch.iter().map(|x| w(&g, x.1)).collect();
        for (i, (a, b)) in pw.iter().zip(&cw).enumerate() { if a != b { println!("{fr}: {} {a:#x} -> {b:#x}", watch[i].0); } }
        let ck = kinds(&g);
        for (s, (a, b)) in pk.iter().zip(&ck).enumerate() { if a != b { println!("{fr}: slot {s}: ${a:x} -> ${b:x}"); } }
        prev = m; pw = cw; pk = ck;
    }
    let _ = base;
    println!("frog at {},{} camera {},{}", w(&g, f::X), w(&g, f::Y), g.state().l(h0(0x16664)), g.state().l(h0(0x16660)));
}

/// The pickup kind of tile `tile` in the level's world table (h0+$f382).
fn kind(g: &Game, tile: u32) -> String {
    let st = g.state();
    let d = sf_conform::display_data();
    let rom = &d.rom().h0;
    let l = |a: u32| { let o = (a - h0(0)) as usize; u32::from_be_bytes([rom[o], rom[o+1], rom[o+2], rom[o+3]]) };
    let world = st.l(h0(0x9fa)) >> 8;
    let mut at = l(h0(0xf382) + 4 * world);
    while (l(at) as i32) >= 0 {
        if l(at) == tile {
            return match l(at + 4) - h0(0) {
                0xed92 => "checkpoint".into(), 0xede0 => "coin".into(), 0xee08 => "spud".into(), 0xee38 => "wings".into(),
                0xee6e => "bottle".into(), 0xeec6 => "speed_up".into(), 0xef12 => "slow_down".into(), 0xef40 => "extra_life".into(),
                0xef78 => "shield".into(), x => format!("fruit${x:x}"),
            };
        }
        at += 8;
    }
    format!("tile{tile}?")
}
