//! The map page (DESIGN §2.10): an input script played on the engine from `W1-1`'s start
//! ([`Start::Level1`]), and everything the toolkit's route page (`amiga-routeview`) draws of it: the
//! level's tiles and palette, every object of its object map, the character's position after each
//! frame, the objects in play every other frame, and the events along the way.
//!
//! The page holds the disks' graphics, so it is written under `games/forest-dumb/build/`
//! ([`default_out`]) and never committed.

use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use amiga_conform::script::{self, InputScript};
use amiga_input::Joy;
use amiga_routeview::json::{Json, esc, list, strs};
use amiga_routeview::site::{self, Category, Entry, PageSpec};
use amiga_routeview::verdict::{Status, Verdict};
use amiga_routeview::{Event, LevelDump};
use fd_assets::LEVEL_ROWS;
use fd_core::recording::{LevelId, Location};
use fd_core::{Game, GameData, Start, State};

/// The level the page draws: the only one built.
pub const LEVEL: LevelId = LevelId::W1_1;
/// Frames played past the script's last line, so a jump or fall it starts lands on the page.
pub const TAIL: u32 = 100;
/// The seatbelt: a script whose last line is past this many frames (30 minutes of play) is refused
/// rather than played.
pub const MAX_FRAMES: u32 = 50 * 60 * 30;
/// The character's offset to the point that names its cell, as `Location::of_character` takes it
/// (fd-core `recording.rs`: `char_x + $e`, `char_y + 9`); the page draws the character there too.
const SPOT: (i32, i32) = (0xe, 9);

/// The page's text for W1-1's objects: `re/objects-W1-1.toml`, a value and its group each.
const OBJECTS: &str = include_str!("../../../../re/objects-W1-1.toml");

/// What Forest Dumb's page knows of the game: 16-pixel cells, the character drawn and placed at its
/// spot, an object in play boxed over the cell it stands on, and a layer per object group of
/// `re/objects-W1-1.toml` ([`object_name`]).
pub fn spec() -> PageSpec {
    PageSpec {
        title: "Forest Dumb Forever".into(),
        cell_size: (16, 16),
        categories: vec![
            Category::new("coins", "Coins", "--accent", true, &["coin"]),
            Category::new("creatures", "Creatures", "--enemy", true, &["enemy"]),
            Category::new("bonuses", "Bonuses", "--pickup", true, &["pickup"]),
            Category::new("water", "Water", "--hazard", true, &["hazard"]),
            Category::new(
                "misc",
                "Springs, vanishing blocks, the exit",
                "#ffffff",
                true,
                &[],
            ),
        ],
        actor: "character".into(),
        anchor: SPOT,
        cell_origin: SPOT,
        live_box: (0, 0, 16, 16),
    }
}

/// An object value's layer and name: its group in `re/objects-W1-1.toml` (the page's cats `coin`,
/// `enemy`, `pickup` and `hazard` draw in their own shapes), or `other` for a value the file lacks.
pub fn object_name(value: u8) -> (&'static str, String) {
    let group = groups().into_iter().find(|g| g.0 == value).map(|g| g.1);
    let cat = match group.as_deref() {
        // The page's summary counts the objects named `coin`; docs/10: `$20`..`$23` are coins.
        Some("item") => return ("coin", "coin".into()),
        Some("creature") => "enemy",
        Some("bonus") => "pickup",
        Some("water") => "hazard",
        Some("spring") => "spring",
        Some("vanishing-block") => "block",
        // The page labels an object named `level exit` EXIT.
        Some("exit") => return ("exit", "level exit".into()),
        _ => "other",
    };
    let name = match group {
        Some(g) => format!("{g} ${value:02x}"),
        None => format!("object ${value:02x}"),
    };
    (cat, name)
}

/// (value, group) per `[[value]]` of `re/objects-W1-1.toml`, read by hand: two keys of a flat file.
fn groups() -> Vec<(u8, String)> {
    let mut out = Vec::new();
    let mut value = None;
    for line in OBJECTS.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "value" => value = u8::from_str_radix(v.trim_start_matches("0x"), 16).ok(),
            "group" => {
                if let Some(n) = value.take() {
                    out.push((n, v.trim_matches('"').to_string()));
                }
            }
            _ => {}
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub c: u16,
    pub r: u16,
    pub value: u8,
    pub cat: &'static str,
    pub name: String,
}

/// A script played on the engine, as the page draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Dump {
    /// The script's comment lines before its first input line.
    pub notes: Vec<String>,
    /// The level palette, 8 bits a channel.
    pub palette: Vec<[u8; 3]>,
    pub cols: u16,
    /// The tile map, row by row (the page's order; the file holds it column by column).
    pub body: Vec<u16>,
    /// The tiles the map uses, each 16×16 colour numbers, row by row.
    pub tiles: Vec<(u16, [u8; 256])>,
    pub objects: Vec<Object>,
    /// `char_x`, `char_y` after each frame played.
    pub track: Vec<(i32, i32)>,
    pub inputs: Vec<u8>,
    /// The objects in play (x, y, value) every other frame.
    pub live: Vec<Vec<(i32, i32, u8)>>,
    pub events: Vec<Event>,
    /// `items_left` at the start, decoded from BCD: the items the exit needs.
    pub items_needed: u16,
    /// The script's last line's frame: past it the replay only idles.
    pub inputs_end: u32,
}

impl Dump {
    /// The cell the character ends on.
    pub fn end_cell(&self) -> Location {
        let (x, y) = self.track.last().copied().unwrap_or((0, 0));
        Location::of_character(LEVEL, x as u32, y as u32)
    }
}

/// `text` as an input script, an error naming `path` and the line when it is not one or runs past
/// [`MAX_FRAMES`].
pub fn parse_script(path: &Path, text: &str) -> Result<InputScript, String> {
    let s = script::parse(text).map_err(|e| format!("{}: {e}", path.display()))?;
    let last = s.lines.last().map_or(0, |l| l.0);
    if last > MAX_FRAMES {
        return Err(format!(
            "{}: its last line is at frame {last}, past the {MAX_FRAMES}-frame limit",
            path.display()
        ));
    }
    Ok(s)
}

/// A BCD word's value.
fn bcd(w: u16) -> u16 {
    (0..4).rev().fold(0, |n, i| n * 10 + (w >> (4 * i) & 0xf))
}

fn rgb(c: u16) -> [u8; 3] {
    let n = |s: u16| ((c >> s) & 0xf) as u8 * 17;
    [n(8), n(4), n(0)]
}

/// Plays `script` (with `notes`, its header) on the engine from W1-1's start, a step a frame,
/// until [`TAIL`] frames past its last line.
pub fn replay(data: &Arc<GameData>, script: &InputScript, notes: Vec<String>) -> Dump {
    let level = &data.level;
    let cols = level.tiles.columns;
    let body: Vec<u16> = (0..LEVEL_ROWS)
        .flat_map(|r| (0..cols).map(move |c| (c, r)))
        .map(|(c, r)| level.tile(c, r))
        .collect();
    let used: BTreeSet<u16> = body.iter().copied().collect();
    let tiles = used
        .into_iter()
        .map(|t| {
            (
                t,
                std::array::from_fn(|i| level.tile_pixel(t, (i % 16) as u8, (i / 16) as u8)),
            )
        })
        .collect();
    let objects = (0..cols)
        .flat_map(|c| (0..LEVEL_ROWS).map(move |r| (c, r)))
        .filter_map(|(c, r)| {
            let value = level.object(c, r);
            (value != 0).then(|| {
                let (cat, name) = object_name(value);
                Object {
                    c,
                    r,
                    value,
                    cat,
                    name,
                }
            })
        })
        .collect();

    let inputs_end = script.lines.last().map_or(0, |l| l.0);
    let mut g = Game::new(data.clone(), Start::Level1);
    let mut d = Dump {
        notes,
        palette: data.display.palette().iter().map(|&c| rgb(c)).collect(),
        cols,
        body,
        tiles,
        objects,
        track: Vec::new(),
        inputs: Vec::new(),
        live: Vec::new(),
        events: Vec::new(),
        items_needed: bcd(g.state().items_left),
        inputs_end,
    };
    let var = |s: &State, n: &str| s.get(n).unwrap_or(0);
    let mut prev = g.state().clone();
    let mut prev_fire = false;
    for k in 0..inputs_end + TAIL {
        let x = script.input_at(k);
        g.step(x);
        let raised = g.take_events();
        let st = g.state();
        let (cx, cy) = (st.char_x as i32, st.char_y as i32);
        d.track.push((cx, cy));
        d.inputs.push(x.joy.0);
        let mut ev = |kind, text: String| {
            d.events.push(Event {
                k,
                x: cx,
                y: cy,
                kind,
                text,
            })
        };
        let fire = x.joy.0 & Joy::FIRE.0 != 0;
        if fire && !prev_fire {
            ev("fire", "fire pressed".into());
        }
        prev_fire = fire;
        if var(st, "char_rising") != 0 && var(&prev, "char_rising") == 0 {
            ev("jump", "take-off".into());
        }
        if var(st, "char_falling") == 0 && var(&prev, "char_falling") != 0 {
            ev("land", "landed".into());
        }
        if st.hits_taken > prev.hits_taken {
            ev(
                "hurt",
                format!("hit taken ({} → {})", prev.hits_taken, st.hits_taken),
            );
        }
        if st.lives < prev.lives {
            ev(
                "death",
                format!("life lost ({} → {})", prev.lives, st.lives),
            );
        }
        for e in raised {
            let text = match e {
                fd_core::Event::Unported(v) => format!("object ${v:02x} met, not ported yet"),
                e => format!("{e:?}"),
            };
            d.events.push(Event {
                k,
                x: cx,
                y: cy,
                kind: "other",
                text,
            });
        }
        // Object cells that changed this frame: an item taken, a creature spawned, a block gone.
        let rows = LEVEL_ROWS as usize;
        for (i, (&a, &b)) in prev.object_map.iter().zip(&st.object_map).enumerate() {
            let (c, r) = ((i / rows) as u16, (i % rows) as u16);
            if a != b && a != 0 && c < cols {
                d.events.push(Event {
                    k,
                    x: c as i32 * 16,
                    y: r as i32 * 16,
                    kind: "marker",
                    text: format!("cell {c},{r}: marker {a} → {b}"),
                });
            }
        }
        if k % 2 == 0 {
            d.live
                .push(g.objects().iter().map(|o| (o.x, o.y, o.value)).collect());
        }
        prev = g.state().clone();
    }
    d
}

impl Dump {
    /// The page's data, one JSON object.
    pub fn to_json(&self) -> String {
        let mut j = Json::object();
        j.str("level", LEVEL.name());
        j.str("timing", "app");
        j.num("seed", 1);
        j.num("cols", self.cols);
        j.num("rows", LEVEL_ROWS);
        j.num("coins_needed", self.items_needed);
        j.raw("end", "null");
        j.num("inputs_end", self.inputs_end);
        j.raw("notes", &strs(self.notes.iter()));
        j.raw(
            "palette",
            &list(
                self.palette
                    .iter()
                    .map(|[r, g, b]| format!("\"#{r:02x}{g:02x}{b:02x}\"")),
            ),
        );
        let tiles: Vec<String> = self
            .tiles
            .iter()
            .map(|(ix, px)| {
                format!(
                    "\"{ix}\":\"{}\"",
                    px.iter().map(|p| format!("{p:02x}")).collect::<String>()
                )
            })
            .collect();
        j.raw("tiles", &format!("{{{}}}", tiles.join(",")));
        j.raw("body", &list(self.body.iter().map(u16::to_string)));
        j.raw(
            "objects",
            &list(self.objects.iter().map(|o| {
                format!(
                    "[{},{},{},\"{}\",\"{}\"]",
                    o.c,
                    o.r,
                    o.value,
                    o.cat,
                    esc(&o.name)
                )
            })),
        );
        j.raw(
            "track",
            &list(self.track.iter().map(|(x, y)| format!("[{x},{y}]"))),
        );
        j.raw("inputs", &list(self.inputs.iter().map(u8::to_string)));
        j.raw(
            "live",
            &list(self.live.iter().map(|v| {
                list(v.iter().map(|&(x, y, value)| {
                    let (cat, name) = object_name(value);
                    format!("[{x},{y},\"{}\",null,\"{cat}\"]", esc(&name))
                }))
            })),
        );
        j.raw(
            "events",
            &list(self.events.iter().map(|e| {
                format!(
                    "[{},{},{},\"{}\",\"{}\"]",
                    e.k,
                    e.x,
                    e.y,
                    e.kind,
                    esc(&e.text)
                )
            })),
        );
        j.finish()
    }
}

/// A played script with its row in the switcher.
pub struct Route<'a> {
    pub dump: &'a Dump,
    pub entry: Entry,
}

impl LevelDump for Route<'_> {
    fn level_id(&self) -> &str {
        &self.entry.id
    }

    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> {
        out.write_all(self.dump.to_json().as_bytes())
    }

    fn entry(&self) -> Entry {
        self.entry.clone()
    }
}

/// `games/forest-dumb/build/route-viz`, which the game's `.gitignore` keeps out of git.
pub fn default_out() -> PathBuf {
    let game = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    game.canonicalize().unwrap_or(game).join("build/route-viz")
}

/// What the page says in place of a verdict.
pub const NO_VERDICT: &str =
    "no verdict: the level's end is not ported yet (T26), so the page cannot say a script finishes";

/// The toolkit page as the map page needs it. It loads a web font; the map page opens from disk and
/// makes no request outside it (T19), so the line is dropped and the page falls back to the
/// system's fonts. And a route the toolkit can only call clean, faults or failed is shown as the
/// page's own `unknown` ("No verdict"): a clean route there reads "reaches the exit", which nothing
/// here knows.
fn offline(page: &str) -> String {
    let page = page.replace(
        r#""status":"clean","faults":[""#,
        r#""status":"unknown","faults":[""#,
    );
    page.lines()
        .filter(|l| !(l.starts_with("<link") && l.contains("://")))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Plays the script at `script_path` on `data` and writes the page under `out`: `index.html` and
/// `data/W1-1.js`. Returns the page's path and the replay. A missing or malformed script is an error
/// naming its path.
pub fn write_page(
    data: &Arc<GameData>,
    script_path: &Path,
    out: &Path,
) -> Result<(PathBuf, Dump), String> {
    let text = std::fs::read_to_string(script_path)
        .map_err(|e| format!("script {}: {e}", script_path.display()))?;
    let script = parse_script(script_path, &text)?;
    let notes = text
        .lines()
        .take_while(|l| l.starts_with('#'))
        .map(str::to_string)
        .collect();
    let dump = replay(data, &script, notes);
    let entry = Entry {
        id: LEVEL.name().into(),
        label: LEVEL.name().into(),
        group: "W1".into(),
        route_file: Some(script_path.display().to_string()),
        // Until the level's end is ported (T26) the page gives no verdict ([`offline`] marks it
        // unknown) and says why.
        verdict: Verdict {
            status: Status::Clean,
            faults: vec![NO_VERDICT.into()],
            info: vec![],
        },
        wall_s: None,
        ref_frames: None,
    };
    let route = Route {
        dump: &dump,
        entry: entry.clone(),
    };
    let fail = |e: io::Error| format!("{}: {e}", out.display());
    site::write_level(out, &route).map_err(fail)?;
    let page = site::write_index(out, &spec(), &[entry], false).map_err(fail)?;
    let html = std::fs::read_to_string(&page).map_err(fail)?;
    std::fs::write(&page, offline(&html)).map_err(fail)?;
    Ok((page, dump))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_of_the_object_file_has_a_layer() {
        let g = groups();
        assert_eq!(g.len(), 24, "{g:?}");
        assert_eq!(object_name(0x20), ("coin", "coin".into()));
        assert_eq!(object_name(0x01).0, "enemy");
        assert_eq!(object_name(0xff), ("exit", "level exit".into()));
        assert_eq!(object_name(0x99), ("other", "object $99".into()));
        let s = spec();
        for v in g.iter().map(|g| g.0) {
            let cat = object_name(v).0;
            let layers = s
                .categories
                .iter()
                .filter(|c| c.cats.iter().any(|x| x == cat) || c.cats.is_empty())
                .count();
            assert!(layers >= 1, "{v:#x}");
        }
    }

    #[test]
    fn palette_words_become_eight_bit_colours_and_bcd_its_value() {
        assert_eq!(rgb(0x0f80), [0xff, 0x88, 0x00]);
        assert_eq!(bcd(0x0282), 282);
        assert_eq!(bcd(0x9999), 9999);
    }

    #[test]
    fn the_page_makes_no_request_outside_its_folder() {
        let p = offline(
            "<title>t</title>\n<link rel=\"stylesheet\" href=\"https://x/y\">\n<style>a{}</style>\n",
        );
        assert_eq!(p, "<title>t</title>\n<style>a{}</style>\n");
        let m = offline(r#"[{"id":"W1-1","status":"clean","faults":["no verdict"]}]"#);
        assert_eq!(
            m,
            "[{\"id\":\"W1-1\",\"status\":\"unknown\",\"faults\":[\"no verdict\"]}]\n"
        );
    }

    #[test]
    fn a_script_past_the_limit_is_refused_by_name() {
        let e = parse_script(
            Path::new("long.txt"),
            &format!("0: R\n{}: -\n", MAX_FRAMES + 1),
        )
        .unwrap_err();
        assert!(e.starts_with("long.txt: ") && e.contains("limit"), "{e}");
        let e = parse_script(Path::new("bad.txt"), "0: Q\n").unwrap_err();
        assert!(e.starts_with("bad.txt: line 1"), "{e}");
    }
}
