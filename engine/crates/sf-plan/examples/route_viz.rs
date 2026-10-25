//! Route viewer data: plays an app-timed route (`# timing: app`, `# seed: N`) on the engine from
//! its app start and writes one JSON file for `tools/route-viz/`: the level's tiles and palette,
//! every object marker on the map, the frog's position each frame, the live objects every other
//! frame, and the events along the way (a marker taken or killed, a heart or life lost, take-offs,
//! landings, shots).
//!
//! cargo run --release -p sf-plan --example route_viz -- W1-4 route.txt out.json

use std::fmt::Write as _;

use sf_conform::InputScript;
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{h0, h1};
use sf_core::level::standard_dispatch;
use sf_core::{LevelId, State};
use sf_plan::app::{app_start, route_seed};

const COLS: u32 = 300;
const ROWS: u32 = 84;
/// The live map's row 0 and a row's bytes (sf-plan `core/facts.rs`).
const ROW0: u32 = h1(0x218e0 + 0x4c0);
const ROW_BYTES: u32 = 0x260;
const OBJ_SLOTS: u32 = h0(0x1ba6c);
const SLOT_SIZE: u32 = 0x7c;
const SLOTS: u32 = 20;
const HEARTS: u32 = h0(0x830);
const SCORE: u32 = h0(0x834);
const LEVEL_STATE: u32 = h0(0xa06);
const OBJ_WORLD_OFFSET: u32 = h0(0x9fa);
const PICKUP_TABLES: u32 = 0xf382;

fn cell_addr(c: u32, r: u32) -> u32 {
    ROW0 + r * ROW_BYTES + 2 * c
}

fn cell_of(addr: u32) -> Option<(u32, u32)> {
    let off = addr.checked_sub(ROW0)?;
    let (r, c) = (off / ROW_BYTES, off % ROW_BYTES / 2);
    (r < ROWS && c < COLS).then_some((c, r))
}

fn markers(st: &State) -> Vec<u8> {
    (0..ROWS)
        .flat_map(|r| (0..COLS).map(move |c| (c, r)))
        .map(|(c, r)| (st.w(cell_addr(c, r)) & 0x3f) as u8)
        .collect()
}

/// World 1's object markers and the templates their stubs spawn (sf-core `enemies/w1`), and the
/// ones every world shares (`items`).
fn marker_template(world: u32, m: u8) -> Option<u16> {
    let shared = match m {
        8 | 9 => Some(0),
        29 => Some(5),
        45 => Some(4),
        _ => None,
    };
    if shared.is_some() || world != 0 {
        return shared;
    }
    Some(match m {
        1 => 8,
        2 => 12,
        3 => 13,
        10 => 33,
        11 => 23,
        12 => 22,
        13 => 20,
        14 => 19,
        15 => 21,
        16 => 78,
        17 => 77,
        18 => 76,
        _ => return None,
    })
}

/// What the frog itself reacts to (docs/09, `level/collision.rs`).
fn frog_marker(m: u8) -> Option<(&'static str, &'static str)> {
    Some(match m {
        6 => ("hazard", "spikes"),
        38 => ("secret", "secret passage"),
        39..=43 => ("spring", "spring"),
        46 | 47 => ("surface", "conveyor"),
        56 => ("surface", "ice"),
        57 => ("surface", "push"),
        48..=51 => ("switch", "bump block / switch"),
        54 => ("exit", "level exit"),
        _ => return None,
    })
}

fn pickup_name(handler: u32) -> &'static str {
    match handler {
        0xed92 => "checkpoint",
        0xede0 => "tile coin",
        0xee08 => "spud",
        0xee38 => "wings",
        0xee6e => "bottle",
        0xeec6 => "speed up",
        0xef12 => "slow down",
        0xef40 => "extra life",
        0xef78 => "shield",
        0xefc2 | 0xefd0 | 0xefde | 0xefec | 0xeffa | 0xf008 | 0xf016 | 0xf024 | 0xf032
        | 0xf040 | 0xf04e => "fruit",
        _ => "pickup",
    }
}

/// The world's tile-pickup table: tile → handler offset in h0.
fn pickup_table(st: &State, rom: &sf_core::mem::Rom) -> Vec<(u32, u32)> {
    let l = |a: u32| -> Option<u32> {
        let o = a as usize;
        Some(u32::from_be_bytes(rom.h0.get(o..o + 4)?.try_into().ok()?))
    };
    let world = st.l(OBJ_WORLD_OFFSET) >> 8;
    let Some(mut at) = l(PICKUP_TABLES + 4 * world).and_then(|a| a.checked_sub(h0(0))) else {
        return Vec::new();
    };
    let mut v = Vec::new();
    while let Some(tile) = l(at).filter(|&t| (t as i32) >= 0) {
        if let Some(hd) = l(at + 4) {
            v.push((tile, hd.wrapping_sub(h0(0))));
        }
        at += 8;
    }
    v
}

/// The kind a live slot is, by its init routine.
fn slot_kind(init: u32) -> &'static str {
    let d = standard_dispatch();
    (0..160)
        .filter_map(|t| d.kind(t))
        .find(|k| k.routines.first().is_some_and(|r| r.0 == init))
        .map_or("object", |k| k.name)
}

fn category(name: &str) -> &'static str {
    match name {
        "coin" | "falling coin" | "hidden coin spring" => "coin",
        "exit lock" => "exit",
        "bounce pad" | "launcher" | "lever" | "bubble" => "fixture",
        _ => "enemy",
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn joy_text(x: FrameInput) -> String {
    let j = x.joy.0;
    let mut s = String::new();
    for (bit, ch) in [(Joy::UP.0, 'U'), (Joy::DOWN.0, 'D'), (Joy::LEFT.0, 'L'), (Joy::RIGHT.0, 'R'), (Joy::FIRE.0, 'F')] {
        if j & bit != 0 {
            s.push(ch);
        }
    }
    if s.is_empty() { "-".into() } else { s }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, level, route, out] = &args[..] else {
        eprintln!("usage: route_viz <level> <route.txt> <out.json>");
        std::process::exit(2);
    };
    let id: LevelId = level.parse().expect("level id");
    let text = std::fs::read_to_string(route).expect("route file");
    let seed = route_seed(&text).unwrap_or(1);
    let script = InputScript::parse(&text).expect("route parses");
    let n = script.lines.last().map_or(0, |l| l.0) + 600;

    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let lvl = &data.levels[&id];
    let t = &lvl.t7mp;
    let world = data.world(id.world);

    let mut g = app_start(id, seed);
    let st0 = g.state().clone();
    let table = pickup_table(&st0, display.rom());
    let world_ix = st0.l(OBJ_WORLD_OFFSET) >> 8;

    let mut j = String::new();
    write!(j, "{{\"level\":\"{id}\",\"seed\":{seed},\"cols\":{COLS},\"rows\":{ROWS},").unwrap();
    // `coins_left_for_exit` (h0+$832) at the start: the coins the exit lock wants.
    write!(j, "\"coins_needed\":{},", st0.w(h0(0x832))).unwrap();
    // The route's comment lines, for the page's header.
    let notes: Vec<String> = text.lines().take_while(|l| l.starts_with('#')).map(|l| format!("\"{}\"", esc(l))).collect();
    write!(j, "\"notes\":[{}],", notes.join(",")).unwrap();
    let pal: Vec<String> = (0..32).map(|i| {
        let [r, g, b] = world.palette.rgb888(i);
        format!("\"#{r:02x}{g:02x}{b:02x}\"")
    }).collect();
    write!(j, "\"palette\":[{}],", pal.join(",")).unwrap();
    // Tiles from the file's BODY (low 10 bits), the ones used only, as 512 hex digits each.
    let body: Vec<u16> = t.body.iter().map(|w| w & 0x3ff).collect();
    let mut used: Vec<u16> = body.clone();
    used.sort();
    used.dedup();
    let tiles: Vec<String> = used.iter().filter_map(|&ix| {
        let px = world.tiles.tiles.get(ix as usize)?;
        let hex: String = px.iter().map(|p| format!("{p:02x}")).collect();
        Some(format!("\"{ix}\":\"{hex}\""))
    }).collect();
    write!(j, "\"tiles\":{{{}}},", tiles.join(",")).unwrap();
    write!(j, "\"body\":[{}],", body.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")).unwrap();

    // Every marker on the live map at the level's start.
    let m0 = markers(&st0);
    let d = standard_dispatch();
    let mut objs = Vec::new();
    for r in 0..ROWS {
        for c in 0..COLS {
            let m = m0[(r * COLS + c) as usize];
            if m == 0 || m >= 0x3a {
                continue;
            }
            let (cat, name): (&str, String) = if m == 55 {
                let tile = (st0.w(cell_addr(c, r)) >> 6) as u32;
                match table.iter().find(|e| e.0 == tile) {
                    Some(&(_, h)) => {
                        let nm = pickup_name(h);
                        (if nm == "tile coin" { "coin" } else { "pickup" }, nm.to_string())
                    }
                    None => ("pickup", "pickup".into()),
                }
            } else if let Some(k) = marker_template(world_ix, m).and_then(|tp| d.kind(tp)) {
                (category(k.name), k.name.to_string())
            } else if let Some((cat, nm)) = frog_marker(m) {
                (cat, nm.to_string())
            } else if d.marker(world_ix as u8, m).is_some() {
                ("enemy", format!("object marker {m}"))
            } else {
                ("other", format!("marker {m}"))
            };
            objs.push(format!("[{c},{r},{m},\"{cat}\",\"{}\"]", esc(&name)));
        }
    }
    write!(j, "\"objects\":[{}],", objs.join(",")).unwrap();

    // The replay.
    let mut track = Vec::new();
    let mut inputs = Vec::new();
    let mut live = Vec::new();
    let mut events = Vec::new();
    let mut prev_m = m0;
    let w = |st: &State, o: u32| st.w(FROG + o) as i16 as i32;
    let landed = |st: &State| (st.w(FROG + f::ON_GROUND) != 0 || st.w(FROG + f::ON_PLATFORM) != 0) && st.w(FROG + f::AIR) == 0;
    let mut was_landed = landed(&st0);
    let (mut hearts, mut lives, mut score) = (st0.w(HEARTS), st0.w(FROG + f::LIVES), st0.l(SCORE));
    let mut prev_fire = false;
    let mut end = None;
    let mut ev = |k: u32, x: i32, y: i32, kind: &str, text: String| {
        events.push(format!("[{k},{x},{y},\"{kind}\",\"{}\"]", esc(&text)));
    };
    for k in 0..n {
        let x = script.input_at(k);
        g.step_unseen(x);
        let st = g.state();
        let (fx, fy) = (w(st, f::X), w(st, f::Y));
        track.push(format!("{fx},{fy}"));
        inputs.push(x.joy.0.to_string());
        let fire = x.joy.0 & Joy::FIRE.0 != 0;
        if fire && !prev_fire {
            ev(k, fx, fy, "fire", format!("fire ({})", joy_text(x)));
        }
        prev_fire = fire;
        let now_landed = landed(st);
        if was_landed && !now_landed && w(st, f::VY) < 0 {
            ev(k, fx, fy, "jump", format!("take-off ({})", joy_text(x)));
        }
        if !was_landed && now_landed {
            ev(k, fx, fy, "land", "landed".into());
        }
        was_landed = now_landed;
        let (h, l, sc) = (st.w(HEARTS), st.w(FROG + f::LIVES), st.l(SCORE));
        if h < hearts {
            ev(k, fx, fy, "hurt", format!("heart lost ({hearts} → {h})"));
        }
        if l < lives {
            ev(k, fx, fy, "death", format!("life lost ({lives} → {l})"));
        }
        let _ = sc > score;
        (hearts, lives, score) = (h, l, sc);
        // Markers that changed this frame.
        let mk = markers(st);
        for (i, (&a, &b)) in prev_m.iter().zip(&mk).enumerate() {
            if a != b && a != 0 && a < 0x3a {
                let (c, r) = (i as u32 % COLS, i as u32 / COLS);
                ev(k, (c * 16) as i32, (r * 16) as i32, "marker", format!("cell {c},{r}: marker {a} → {b}"));
            }
        }
        prev_m = mk;
        // Live slots every other frame: x, y, kind, the cell it came from.
        if k % 2 == 0 {
            let mut v = Vec::new();
            for s in 0..SLOTS {
                let slot = OBJ_SLOTS + s * SLOT_SIZE;
                let from = st.l(slot + 0x40);
                if from == 0 {
                    continue;
                }
                let (ox, oy) = (st.w(slot + 8) as i16, st.w(slot + 0xa) as i16);
                let name = slot_kind(st.l(slot));
                let cell = cell_of(from).map_or("null".into(), |(c, r)| format!("[{c},{r}]"));
                v.push(format!("[{ox},{oy},\"{}\",{cell}]", esc(name)));
            }
            live.push(format!("[{}]", v.join(",")));
        }
        if k >= 10 && st.l(LEVEL_STATE) != 0xffff_fffe && end.is_none() {
            end = Some(k + 1);
            ev(k, fx, fy, "end", format!("level ends (level_state {:#x})", st.l(LEVEL_STATE) as i32));
            break;
        }
    }
    write!(j, "\"end\":{},", end.map_or("null".into(), |e| e.to_string())).unwrap();
    write!(j, "\"track\":[{}],", track.iter().map(|p| format!("[{p}]")).collect::<Vec<_>>().join(",")).unwrap();
    write!(j, "\"inputs\":[{}],", inputs.join(",")).unwrap();
    write!(j, "\"live\":[{}],", live.join(",")).unwrap();
    write!(j, "\"events\":[{}]}}", events.join(",")).unwrap();
    std::fs::write(out, j).expect("write");
    eprintln!("{id}: {} frames, end {:?}, {} objects, {} events", track.len(), end, objs.len(), events.len());
}
