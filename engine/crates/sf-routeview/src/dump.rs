//! A route played on the engine from the app's start (`sf_plan::app::app_start`, its `# seed:`),
//! and everything the page draws: the level's tiles and palette, every object marker on the map,
//! the frog's position each frame, the live objects every other frame, and the events along the
//! way (a marker taken or killed, a heart or life lost, take-offs, landings, shots).

use sf_conform::InputScript;
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{h0, h1};
use sf_core::level::standard_dispatch;
use sf_core::mem::Rom;
use sf_core::{LevelId, State};
use sf_plan::app::{app_start, route_seed};

use crate::json::{Json, esc, list};

pub const COLS: u32 = 300;
pub const ROWS: u32 = 84;
/// The live map's row 0 and a row's bytes (sf-plan `core/facts.rs`).
const ROW0: u32 = h1(0x218e0 + 0x4c0);
const ROW_BYTES: u32 = 0x260;
/// `obj_slots` (docs/09): 20 slots of `$7c` bytes; `+$40` non-zero is a live slot.
const OBJ_SLOTS: u32 = h0(0x1ba6c);
const SLOT_SIZE: u32 = 0x7c;
const SLOTS: u32 = 20;
const HEARTS: u32 = h0(0x830);
const LEVEL_STATE: u32 = h0(0xa06);
const PLAYING: u32 = 0xffff_fffe;
const OBJ_WORLD_OFFSET: u32 = h0(0x9fa);
/// `coins_left_for_exit` (h0+$832): coins still needed before the exit lock goes.
const COINS_LEFT: u32 = h0(0x832);
/// h0+$f382: per world, its tile-pickup table of (tile, handler) longs.
const PICKUP_TABLES: u32 = 0xf382;
/// `obj_flag_jumptab` (h0+$178ec) and `obj_templates` (h0+$1c6e2, `$ae` bytes a template).
const JUMPTAB: u32 = 0x178ec;
const TEMPLATES: u32 = 0x1c6e2;
const TEMPLATE_SIZE: u32 = 0xae;
/// Frames the replay runs past a route's last line, for the level's end (the tally) to show.
const TAIL: u32 = 600;

/// The page's text for a route with no file (a level the planner failed and kept nothing of).
const NO_ROUTE: &str = "# timing: app\n# No route: the planner kept none for this level.\n0: -\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub c: u32,
    pub r: u32,
    pub marker: u8,
    /// `coin`, `enemy`, `pickup`, `fixture`, `exit`, `hazard`, `secret`, `spring`, `switch`,
    /// `surface` or `other`: the page's layer.
    pub cat: &'static str,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub k: u32,
    /// Level pixels: the frog's record x, y for frog events, the cell's top-left for markers.
    pub x: i32,
    pub y: i32,
    /// `marker`, `hurt`, `death`, `jump`, `land`, `fire`, `end`, `stall`.
    pub kind: &'static str,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Live {
    pub x: i16,
    pub y: i16,
    pub name: &'static str,
    pub cat: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dump {
    pub level: LevelId,
    /// The route's `# timing:` (`original` when it has none).
    pub timing: String,
    pub seed: u32,
    pub coins_needed: u16,
    /// The first frame `coins_left_for_exit` is 0: the exit is open from there.
    pub coins_met_at: Option<u32>,
    /// Coins still needed on the last frame played.
    pub coins_left_at_end: u16,
    pub notes: Vec<String>,
    pub palette: Vec<[u8; 3]>,
    /// The file's BODY, tile index per cell (low 10 bits), and the used tiles' 256 colour indices.
    pub body: Vec<u16>,
    pub tiles: Vec<(u16, [u8; 256])>,
    pub objects: Vec<Object>,
    /// The frog's record x, y after each frame played.
    pub track: Vec<(i16, i16)>,
    pub inputs: Vec<u8>,
    /// Every other frame.
    pub live: Vec<Vec<Live>>,
    pub events: Vec<Event>,
    /// The frame count at which the level ended (`level_state` left playing), if it did.
    pub end: Option<u32>,
    /// The route's last line's frame: past it the replay only idles.
    pub inputs_end: u32,
    /// A waypoint path drawn over the route (`--guide`), and its file's name.
    pub guide: Option<(String, Vec<crate::guide::Waypoint>)>,
}

fn cell_addr(c: u32, r: u32) -> u32 {
    ROW0 + r * ROW_BYTES + 2 * c
}

fn markers(st: &State) -> Vec<u8> {
    (0..ROWS)
        .flat_map(|r| (0..COLS).map(move |c| (c, r)))
        .map(|(c, r)| (st.w(cell_addr(c, r)) & 0x3f) as u8)
        .collect()
}

fn rom_w(rom: &Rom, o: u32) -> Option<u16> {
    let o = o as usize;
    Some(u16::from_be_bytes(rom.h0.get(o..o + 2)?.try_into().ok()?))
}

fn rom_l(rom: &Rom, o: u32) -> Option<u32> {
    Some((rom_w(rom, o)? as u32) << 16 | rom_w(rom, o + 2)? as u32)
}

/// The template a marker's stub spawns, read from the game: `obj_flag_jumptab` holds a `bra.w`
/// per (world, marker) to a stub that loads the template into a0 (`lea abs.l,a0` or
/// `lea d16(pc),a0`) before branching to a spawner (docs/09 "Placement"). Patroller stubs call a
/// check first (`bsr`/`jsr`), so only an unconditional branch, jump or return ends the search.
pub fn marker_template(rom: &Rom, world: u32, m: u8) -> Option<u16> {
    let entry = JUMPTAB + (world * 64 + m as u32) * 4;
    if rom_w(rom, entry)? != 0x6000 {
        return None;
    }
    let stub = (entry + 2).wrapping_add(rom_w(rom, entry + 2)? as i16 as i32 as u32);
    // A stub is a few instructions; past 48 bytes we are in the next one.
    for at in (stub..stub + 48).step_by(2) {
        let addr = match rom_w(rom, at)? {
            0x41f9 => rom_l(rom, at + 2)?.wrapping_sub(h0(0)),
            0x41fa => (at + 2).wrapping_add(rom_w(rom, at + 2)? as i16 as i32 as u32),
            0x6000 | 0x4ef9 | 0x4e75 => return None,
            _ => continue,
        };
        let off = addr.checked_sub(TEMPLATES)?;
        return (off % TEMPLATE_SIZE == 0 && off / TEMPLATE_SIZE < 160).then_some((off / TEMPLATE_SIZE) as u16);
    }
    None
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

/// A tile pickup's handler (h0 offset, `items/pickups.rs`) by what it does.
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
        0xefc2 | 0xefd0 | 0xefde | 0xefec | 0xeffa | 0xf008 | 0xf016 | 0xf024 | 0xf032 | 0xf040 | 0xf04e => "fruit",
        _ => "pickup",
    }
}

/// The world's tile-pickup table: (tile, handler offset in h0).
fn pickup_table(st: &State, rom: &Rom) -> Vec<(u32, u32)> {
    let world = st.l(OBJ_WORLD_OFFSET) >> 8;
    let Some(mut at) = rom_l(rom, PICKUP_TABLES + 4 * world).and_then(|a| a.checked_sub(h0(0))) else {
        return Vec::new();
    };
    let mut v = Vec::new();
    while let Some(tile) = rom_l(rom, at).filter(|&t| (t as i32) >= 0) {
        if let Some(hd) = rom_l(rom, at + 4) {
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

/// An object kind's layer by its name (sf-core's `ObjectKind::name`).
pub fn category(name: &str) -> &'static str {
    match name {
        "coin" | "falling coin" | "hidden coin spring" => "coin",
        "exit lock" => "exit",
        "bounce pad" | "launcher" | "lever" | "bubble" => "fixture",
        // Kinds that only animate, carry or block the frog (docs/09 "Other"); the rest hurt.
        n if ["scenery", "backdrop", "gate", "crumbl", "pad", "flow", "swing", "fountain"]
            .iter()
            .any(|w| n.contains(w))
            && !n.contains("drop") =>
        {
            "fixture"
        }
        _ => "enemy",
    }
}

fn joy_text(x: FrameInput) -> String {
    let j = x.joy.0;
    let s: String = [(Joy::UP.0, 'U'), (Joy::DOWN.0, 'D'), (Joy::LEFT.0, 'L'), (Joy::RIGHT.0, 'R'), (Joy::FIRE.0, 'F')]
        .iter()
        .filter(|(b, _)| j & b != 0)
        .map(|&(_, c)| c)
        .collect();
    if s.is_empty() { "-".into() } else { s }
}

/// Every object marker on the live map of `st`, named.
fn objects(st: &State, rom: &Rom) -> Vec<Object> {
    let table = pickup_table(st, rom);
    let world = st.l(OBJ_WORLD_OFFSET) >> 8;
    let d = standard_dispatch();
    let m0 = markers(st);
    let mut v = Vec::new();
    for r in 0..ROWS {
        for c in 0..COLS {
            let m = m0[(r * COLS + c) as usize];
            if m == 0 || m >= 0x3a {
                continue;
            }
            let (cat, name): (&'static str, String) = if m == 55 {
                let tile = (st.w(cell_addr(c, r)) >> 6) as u32;
                match table.iter().find(|e| e.0 == tile) {
                    Some(&(_, h)) => {
                        let nm = pickup_name(h);
                        (if nm == "tile coin" { "coin" } else { "pickup" }, nm.to_string())
                    }
                    None => ("pickup", "pickup".into()),
                }
            } else if let Some(k) = marker_template(rom, world, m).and_then(|t| d.kind(t)) {
                (category(k.name), k.name.to_string())
            } else if let Some((cat, nm)) = frog_marker(m) {
                (cat, nm.to_string())
            } else if d.marker(world as u8, m).is_some() {
                ("enemy", format!("object marker {m}"))
            } else {
                ("other", format!("marker {m}"))
            };
            v.push(Object { c, r, marker: m, cat, name });
        }
    }
    v
}

/// Plays `route` (a route file's text; `None` for no route) on `level` from the app's start with
/// the route's `# seed:` (1 without one), until the level ends or 600 frames past its last line.
pub fn replay(level: LevelId, route: Option<&str>) -> Result<Dump, String> {
    let text = route.unwrap_or(NO_ROUTE);
    let seed = route_seed(text).unwrap_or(1);
    // Only an app-timed route replays from the app's start; one paced to the original's lag
    // drifts here (sf-plan DESIGN §2.6). The verdict says so instead of passing it off.
    let timing = sf_plan::trace::timing(text).unwrap_or("original").to_string();
    let script = InputScript::parse(text).map_err(|e| format!("{e:?}"))?;
    let inputs_end = script.lines.last().map_or(0, |l| l.0);
    let n = if route.is_none() { 1 } else { inputs_end + TAIL };

    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let lvl = data.levels.get(&level).ok_or(format!("{level}: no such level on the disks"))?;
    let world = data.world(level.world);
    let body: Vec<u16> = lvl.t7mp.body.iter().map(|w| w & 0x3ff).collect();
    let mut used = body.clone();
    used.sort();
    used.dedup();
    let tiles = used
        .iter()
        .filter_map(|&ix| Some((ix, *world.tiles.tiles.get(ix as usize)?)))
        .collect();

    let mut g = app_start(level, seed);
    let st0 = g.state().clone();
    let objects = objects(&st0, display.rom());

    let w = |st: &State, o: u32| st.w(FROG + o) as i16;
    let landed = |st: &State| (st.w(FROG + f::ON_GROUND) != 0 || st.w(FROG + f::ON_PLATFORM) != 0) && st.w(FROG + f::AIR) == 0;
    let mut d = Dump {
        level,
        timing,
        seed,
        coins_needed: st0.w(COINS_LEFT),
        coins_met_at: (st0.w(COINS_LEFT) == 0).then_some(0),
        coins_left_at_end: st0.w(COINS_LEFT),
        notes: text.lines().take_while(|l| l.starts_with('#')).map(str::to_string).collect(),
        palette: (0..32).map(|i| world.palette.rgb888(i)).collect(),
        body,
        tiles,
        objects,
        track: Vec::new(),
        inputs: Vec::new(),
        live: Vec::new(),
        events: Vec::new(),
        end: None,
        inputs_end,
        guide: None,
    };
    let mut prev_m = markers(&st0);
    let mut was_landed = landed(&st0);
    let (mut hearts, mut lives) = (st0.w(HEARTS), st0.w(FROG + f::LIVES));
    let mut prev_fire = false;
    for k in 0..n {
        let x = script.input_at(k);
        g.step_unseen(x);
        let st = g.state();
        let (fx, fy) = (w(st, f::X), w(st, f::Y));
        d.track.push((fx, fy));
        d.inputs.push(x.joy.0);
        let mut ev = |kind, text: String| d.events.push(Event { k, x: fx as i32, y: fy as i32, kind, text });
        let fire = x.joy.0 & Joy::FIRE.0 != 0;
        if fire && !prev_fire {
            ev("fire", format!("fire ({})", joy_text(x)));
        }
        prev_fire = fire;
        let now_landed = landed(st);
        if was_landed && !now_landed && w(st, f::VY) < 0 {
            ev("jump", format!("take-off ({})", joy_text(x)));
        }
        if !was_landed && now_landed {
            ev("land", "landed".into());
        }
        was_landed = now_landed;
        let (h, l) = (st.w(HEARTS), st.w(FROG + f::LIVES));
        if h < hearts {
            ev("hurt", format!("heart lost ({hearts} → {h})"));
        }
        if l < lives {
            ev("death", format!("life lost ({lives} → {l})"));
        }
        (hearts, lives) = (h, l);
        let left = st.w(COINS_LEFT);
        d.coins_left_at_end = left;
        if left == 0 && d.coins_met_at.is_none() {
            d.coins_met_at = Some(k);
        }
        // Markers that changed this frame: a coin or pickup taken, an enemy killed, a wall opened.
        let mk = markers(st);
        for (i, (&a, &b)) in prev_m.iter().zip(&mk).enumerate() {
            if a != b && a != 0 && a < 0x3a {
                let (c, r) = (i as u32 % COLS, i as u32 / COLS);
                d.events.push(Event {
                    k,
                    x: (c * 16) as i32,
                    y: (r * 16) as i32,
                    kind: "marker",
                    text: format!("cell {c},{r}: marker {a} → {b}"),
                });
            }
        }
        prev_m = mk;
        if k % 2 == 0 {
            let v = (0..SLOTS)
                .map(|s| OBJ_SLOTS + s * SLOT_SIZE)
                .filter(|&slot| st.l(slot + 0x40) != 0)
                .map(|slot| {
                    let name = slot_kind(st.l(slot));
                    Live { x: st.w(slot + 8) as i16, y: st.w(slot + 0xa) as i16, name, cat: category(name) }
                })
                .collect();
            d.live.push(v);
        }
        // The level's `level_state` is not −2 on its first frames (sf-plan `app::SETTLE`).
        if k >= 10 && st.l(LEVEL_STATE) != PLAYING {
            d.end = Some(k + 1);
            d.events.push(Event {
                k,
                x: fx as i32,
                y: fy as i32,
                kind: "end",
                text: format!("level ends (level_state {:#x})", st.l(LEVEL_STATE) as i32),
            });
            break;
        }
    }
    Ok(d)
}

impl Dump {
    pub fn hearts_lost(&self) -> usize {
        self.events.iter().filter(|e| e.kind == "hurt").count()
    }

    pub fn lives_lost(&self) -> usize {
        self.events.iter().filter(|e| e.kind == "death").count()
    }

    /// The page's data, one JSON object.
    pub fn to_json(&self) -> String {
        let mut j = Json::object();
        j.str("level", &self.level.to_string());
        j.str("timing", &self.timing);
        j.num("seed", self.seed);
        j.num("cols", COLS);
        j.num("rows", ROWS);
        j.num("coins_needed", self.coins_needed);
        j.opt("coins_met_at", self.coins_met_at);
        j.num("coins_left_at_end", self.coins_left_at_end);
        j.opt("end", self.end);
        j.num("inputs_end", self.inputs_end);
        j.raw("notes", &crate::json::strs(self.notes.iter()));
        j.raw(
            "palette",
            &list(self.palette.iter().map(|[r, g, b]| format!("\"#{r:02x}{g:02x}{b:02x}\""))),
        );
        let tiles: Vec<String> = self
            .tiles
            .iter()
            .map(|(ix, px)| format!("\"{ix}\":\"{}\"", px.iter().map(|p| format!("{p:02x}")).collect::<String>()))
            .collect();
        j.raw("tiles", &format!("{{{}}}", tiles.join(",")));
        j.raw("body", &list(self.body.iter().map(u16::to_string)));
        j.raw(
            "objects",
            &list(self.objects.iter().map(|o| {
                format!("[{},{},{},\"{}\",\"{}\"]", o.c, o.r, o.marker, o.cat, esc(&o.name))
            })),
        );
        j.raw("track", &list(self.track.iter().map(|(x, y)| format!("[{x},{y}]"))));
        j.raw("inputs", &list(self.inputs.iter().map(u8::to_string)));
        j.raw(
            "live",
            &list(self.live.iter().map(|v| {
                list(v.iter().map(|s| format!("[{},{},\"{}\",null,\"{}\"]", s.x, s.y, esc(s.name), s.cat)))
            })),
        );
        j.raw(
            "events",
            &list(self.events.iter().map(|e| format!("[{},{},{},\"{}\",\"{}\"]", e.k, e.x, e.y, e.kind, esc(&e.text)))),
        );
        if let Some((file, pts)) = &self.guide {
            j.str("guide_file", file);
            j.raw(
                "guide",
                &list(pts.iter().map(|p| {
                    let t = p.t.map_or("null".to_string(), |t| t.to_string());
                    format!("[{},{},{t},{}]", p.c, p.r, p.est)
                })),
            );
        }
        j.finish()
    }
}
