//! Scratch (T36): shared by the beam-calibration examples.
#![allow(dead_code)]
use sf_core::layout::h0;
use sf_core::level::beam::Cost;
use sf_oracle::LineSample;

pub fn syms() -> Vec<(u32, String)> {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/symbols.toml")).unwrap();
    let t: toml::Table = text.parse().unwrap();
    let mut v: Vec<(u32, String)> = t["sym"].as_array().unwrap().iter().filter_map(|s| {
        let s = s.as_table()?;
        if s.get("exe")?.as_str()? != "d1_c03" || s.get("hunk")?.as_integer()? != 0 { return None; }
        if s.get("kind")?.as_str()? != "func" { return None; }
        Some((h0(s["offset"].as_integer()? as u32), s["name"].as_str()?.to_string()))
    }).collect();
    for l in std::fs::read_to_string("/tmp/t36/engine_syms.txt").unwrap().lines() {
        let mut it = l.split(' ');
        let a = h0(u32::from_str_radix(it.next().unwrap(), 16).unwrap());
        let n = it.next().unwrap().to_string();
        if !v.iter().any(|s| s.0 == a) { v.push((a, n)); }
    }
    v.sort();
    v
}

/// (label, the original's routines (any), the engine's cost marking its start)
pub const MARKS: &[(&str, &[&str], Cost)] = &[
    ("tiles", &["frog_tiles"], Cost::FrogTiles),
    ("air", &["frog_air_anim"], Cost::FrogAir),
    ("anim", &["frog_anim_tick"], Cost::FrogAnim),
    ("walk", &["frog_walk_input"], Cost::FrogWalk),
    ("input", &["frog_input_anim"], Cost::FrogInput),
    ("sprites", &["place_frog_sprites"], Cost::FrogSprites),
    ("clock", &["level_clock_tick"], Cost::ClockTick),
    ("move", &["frog_move"], Cost::FrogMove),
    ("camera", &["camera_follow"], Cost::Camera),
    ("music", &["music_tick"], Cost::Music),
    ("scroll", &["scroller"], Cost::Scroller),
    ("scan", &["map_spawn_scan_edges", "scan_edges"], Cost::Scan),
    ("tail", &["music_dma", "sfx_tick"], Cost::VblTail),
    ("pass", &["spud_update"], Cost::Spud),
    ("objs", &["obj_run_all"], Cost::ObjSlot),
    ("hitb", &["obj_build_hitboxes"], Cost::HitboxSlot),
    ("coll", &["collide_all"], Cost::Collide),
    ("list", &["build_bob_draw_list"], Cost::DrawList),
    ("draw", &["bob_draw"], Cost::BobDraw),
    ("panel", &["panel_update"], Cost::Panel),
];

/// The first line each mark's routine is running (PC or on the stack), from a frame's samples.
pub fn original_marks(syms: &[(u32, String)], rom: &[u8], samples: &[LineSample]) -> Vec<Option<u16>> {
    let code = h0(0)..h0(0x4834c);
    let func = |pc: u32| -> Option<&(u32, String)> { let i = syms.partition_point(|s| s.0 <= pc); if i == 0 { None } else { Some(&syms[i - 1]) } };
    let w = |x: u32| -> u16 { let o = (x - h0(0)) as usize; u16::from_be_bytes([rom[o], rom[o + 1]]) };
    let is_ret = |r: u32| code.contains(&r) && r & 1 == 0 && r >= h0(8) && (w(r - 6) == 0x4eb9 || w(r - 4) == 0x6100 || (w(r - 2) & 0xff00 == 0x6100 && w(r - 2) & 0xff != 0) || (w(r - 2) & 0xfff8) == 0x4e90 || (w(r - 4) & 0xfff8) == 0x4ea8);
    let mut orig: Vec<Option<u16>> = vec![None; MARKS.len()];
    for s in samples {
        let mut fs: Vec<&str> = Vec::new();
        if let Some(x) = func(s.pc) { fs.push(&x.1); }
        for &r in s.stack.iter() { if is_ret(r) { if let Some(x) = func(r) { fs.push(&x.1); } } }
        for (k, m) in MARKS.iter().enumerate() {
            if orig[k].is_none() && fs.iter().any(|n| m.1.contains(n)) { orig[k] = Some(s.vpos); }
        }
    }
    orig
}
