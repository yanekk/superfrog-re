//! Shared by the beam examples (`profile`, `fit`) and the item-case examples (`coverage`,
//! `items_map`): the original's routine names, the landmarks the beam costs are fitted to, and
//! the names of a world table's pickup handlers.
#![allow(dead_code)]
use sf_core::layout::h0;
use sf_core::level::beam::Cost;
use sf_oracle::LineSample;

/// End of the main executable's first hunk's code (h0).
pub const CODE_END: u32 = 0x4834c;

/// The original's routines by address, sorted: the functions `re/symbols.toml` names in
/// `d1_c03`'s first hunk, then any further name the engine's own sources give one, written
/// `` `name` (h0+$addr) `` in `sf-core`'s doc comments.
pub fn syms() -> Vec<(u32, String)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let text =
        std::fs::read_to_string(format!("{root}/../re/symbols.toml")).expect("re/symbols.toml");
    let t: toml::Table = text.parse().expect("symbols.toml parses");
    let mut v: Vec<(u32, String)> = t["sym"]
        .as_array()
        .expect("[[sym]]")
        .iter()
        .filter_map(|s| {
            let s = s.as_table()?;
            if s.get("exe")?.as_str()? != "d1_c03" || s.get("hunk")?.as_integer()? != 0 {
                return None;
            }
            if s.get("kind")?.as_str()? != "func" {
                return None;
            }
            Some((
                h0(s["offset"].as_integer()? as u32),
                s["name"].as_str()?.to_string(),
            ))
        })
        .collect();
    for (a, n) in engine_syms(std::path::Path::new(&format!("{root}/crates/sf-core/src"))) {
        if !v.iter().any(|s| s.0 == a) {
            v.push((a, n));
        }
    }
    v.sort();
    v.dedup_by_key(|s| s.0);
    v
}

/// Every `` `name` (h0+$addr) `` in the `.rs` files under `dir`.
fn engine_syms(dir: &std::path::Path) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).expect("sf-core's sources").flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(engine_syms(&p));
            continue;
        }
        if p.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        let src = std::fs::read_to_string(&p).unwrap_or_default();
        for (i, _) in src.match_indices("` (h0+$") {
            let Some(open) = src[..i].rfind('`') else {
                continue;
            };
            let name = &src[open + 1..i];
            if name.is_empty() || !name.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric()) {
                continue;
            }
            let hex: String = src[i + 7..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .collect();
            if let Ok(a) = u32::from_str_radix(&hex, 16) {
                out.push((h0(a), name.to_string()));
            }
        }
    }
    out
}

/// The routine `pc` is in, if it is in the code.
pub fn func(syms: &[(u32, String)], pc: u32) -> Option<&(u32, String)> {
    let i = syms.partition_point(|s| s.0 <= pc);
    if i == 0 || !(h0(0)..h0(CODE_END)).contains(&pc) {
        return None;
    }
    Some(&syms[i - 1])
}

/// `pc` as `routine+offset`.
pub fn name(syms: &[(u32, String)], pc: u32) -> String {
    match func(syms, pc) {
        Some((a, n)) => format!("{n}+{:x}", pc - a),
        None => format!("${pc:x}"),
    }
}

/// Whether `r`, a long on the stack, is a return address: the word before it ends a `jsr` or
/// `bsr`.
pub fn is_return(rom: &[u8], r: u32) -> bool {
    let w = |x: u32| -> u16 {
        let o = (x - h0(0)) as usize;
        u16::from_be_bytes([rom[o], rom[o + 1]])
    };
    (h0(8)..h0(CODE_END)).contains(&r)
        && r & 1 == 0
        && (w(r - 6) == 0x4eb9
            || w(r - 4) == 0x6100
            || (w(r - 2) & 0xff00 == 0x6100 && w(r - 2) & 0xff != 0)
            || (w(r - 2) & 0xfff8) == 0x4e90
            || (w(r - 4) & 0xfff8) == 0x4ea8
            || w(r - 4) == 0x4eba)
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
pub fn original_marks(
    syms: &[(u32, String)],
    rom: &[u8],
    samples: &[LineSample],
) -> Vec<Option<u16>> {
    let mut orig: Vec<Option<u16>> = vec![None; MARKS.len()];
    for s in samples {
        let mut fs: Vec<&str> = Vec::new();
        if let Some(x) = func(syms, s.pc) {
            fs.push(&x.1);
        }
        for &r in s.stack.iter().filter(|&&r| is_return(rom, r)) {
            if let Some(x) = func(syms, r) {
                fs.push(&x.1);
            }
        }
        for (k, m) in MARKS.iter().enumerate() {
            if orig[k].is_none() && fs.iter().any(|n| m.1.contains(n)) {
                orig[k] = Some(s.vpos);
            }
        }
    }
    orig
}

/// A world pickup table's handler (h0 offset) by name.
pub fn pickup(handler: u32) -> String {
    match handler {
        0xed92 => "checkpoint".into(),
        0xede0 => "coin".into(),
        0xee08 => "spud".into(),
        0xee38 => "wings".into(),
        0xee6e => "bottle".into(),
        0xeec6 => "speed_up".into(),
        0xef12 => "slow_down".into(),
        0xef40 => "extra_life".into(),
        0xef78 => "shield".into(),
        0x7d4 => "nothing".into(),
        x => format!("fruit${x:x}"),
    }
}
