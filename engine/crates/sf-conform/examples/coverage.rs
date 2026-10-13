//! What a script makes the frog take and set off, run on the engine with the enemies frozen (as
//! the item cases are): every map marker that changes (with the pickup a taken tile was), every
//! change to the frog's power-ups and lives, and every object slot whose routine changes.
//! For writing and checking `items-*` cases.
//!
//! `cargo run --example coverage -- <case.toml>` (the case's sync point, script and length), or
//! `cargo run --example coverage -- <level> <script> <frames>` (from the level's start).
#[path = "common/mod.rs"]
mod common;
use sf_conform::freeze::Freeze;
use sf_conform::{Case, InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::layout::{VarId, h0, h1};
use sf_core::{Game, Start};

/// The object slots: the first's address, their size and how many.
const SLOTS: u32 = 0x1ba6c;
const SLOT_SIZE: u32 = 0x7c;
const SLOT_COUNT: u32 = 20;
/// The map's row 0 (`map_rows[1]`, h1), and a row's length in bytes.
const ROW0: u32 = 0x218e0 + 0x4c0;
const ROW_BYTES: i64 = 0x260;
/// The camera (h0) and the world number (h0, its high byte).
const CAMERA_X: u32 = 0x16664;
const CAMERA_Y: u32 = 0x16660;
const WORLD: u32 = 0x9fa;
/// The world pickup tables' table (h0): a long per world, each a list of (tile, handler) ended
/// by a negative tile.
const PICKUP_TABLES: u32 = 0xf382;
/// The marker of a tile pickup.
const PICKUP: u8 = 55;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (sync_pt, script, frames) = match &args[..] {
        [case] if case.ends_with(".toml") => {
            let c = Case::load(std::path::Path::new(case)).expect("case");
            (c.sync, c.script, c.frames)
        }
        [level, script, frames] => {
            let s = InputScript::load(&sf_conform::case::script_path(script)).expect("script");
            (
                SyncPoint::AtLevel(level.parse().expect("level id")),
                s,
                frames.parse().expect("frames"),
            )
        }
        _ => {
            eprintln!("usage: coverage <case.toml> | coverage <level> <script> <frames>");
            std::process::exit(2);
        }
    };
    let synced = sync::reach_frozen(&sync_pt, &script, frames + 10, Freeze::Enemies).expect("sync");
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Transplant(synced.memory));
    let map = sf_core::layout::def(VarId::LevelMap);
    let markers = |g: &Game| -> Vec<u8> {
        g.state()
            .var(VarId::LevelMap)
            .chunks(2)
            .map(|c| c[1] & 0x3f)
            .collect()
    };
    let mut prev = markers(&g);
    let w = |g: &Game, o: u32| g.state().w(FROG + o);
    let watch = [
        ("spud", f::SPUD_ON),
        ("flight", f::FLIGHT),
        ("shield", f::SHIELD),
        ("maxspeed", f::MAX_SPEED),
        ("lives", f::LIVES),
        ("launch", f::LAUNCHED),
        ("spring", f::SPRING),
    ];
    let mut pw: Vec<u16> = watch.iter().map(|x| w(&g, x.1)).collect();
    let kinds = |g: &Game| -> Vec<u32> {
        (0..SLOT_COUNT)
            .map(|k| {
                let s = h0(SLOTS) + k * SLOT_SIZE;
                if g.state().l(s + 0x40) != 0 {
                    g.state().l(s + 4) - h0(0)
                } else {
                    0
                }
            })
            .collect()
    };
    let mut pk = kinds(&g);
    for fr in 0..frames {
        g.step(script.input_at(fr));
        let m = markers(&g);
        for (k, (a, b)) in prev.iter().zip(&m).enumerate() {
            if a != b {
                let addr = map.addr + 2 * k as u32;
                let d = addr as i64 - h1(ROW0) as i64;
                let tile = g.state().w(addr) >> 6;
                let what = if *a == PICKUP {
                    kind(&g, tile.wrapping_sub(1) as u32)
                } else {
                    String::new()
                };
                println!(
                    "{fr}: marker {a} -> {b} at {},{} {what}",
                    (d % ROW_BYTES) / 2,
                    d / ROW_BYTES
                );
            }
        }
        let cw: Vec<u16> = watch.iter().map(|x| w(&g, x.1)).collect();
        for (i, (a, b)) in pw.iter().zip(&cw).enumerate() {
            if a != b {
                println!("{fr}: {} {a:#x} -> {b:#x}", watch[i].0);
            }
        }
        let ck = kinds(&g);
        for (s, (a, b)) in pk.iter().zip(&ck).enumerate() {
            if a != b {
                println!("{fr}: slot {s}: ${a:x} -> ${b:x}");
            }
        }
        (prev, pw, pk) = (m, cw, ck);
    }
    println!(
        "frog at {},{} camera {},{}",
        w(&g, f::X),
        w(&g, f::Y),
        g.state().l(h0(CAMERA_X)),
        g.state().l(h0(CAMERA_Y))
    );
}

/// The pickup tile `tile` is in the level's world table.
fn kind(g: &Game, tile: u32) -> String {
    let d = sf_conform::display_data();
    let rom = &d.rom().h0;
    let l = |a: u32| {
        let o = (a - h0(0)) as usize;
        u32::from_be_bytes([rom[o], rom[o + 1], rom[o + 2], rom[o + 3]])
    };
    let world = g.state().l(h0(WORLD)) >> 8;
    let mut at = l(h0(PICKUP_TABLES) + 4 * world);
    while (l(at) as i32) >= 0 {
        if l(at) == tile {
            return common::pickup(l(at + 4) - h0(0));
        }
        at += 8;
    }
    format!("tile{tile}?")
}
