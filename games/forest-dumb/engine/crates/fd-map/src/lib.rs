//! Each world's level as the map page draws it, with no engine run (plan forest-dumb-map-page
//! DESIGN §2.1, §2.2, §3.2): tiles, palette, every object with its layer, the exit cells and the
//! items the exit needs. [`level`] is the pure side (bytes in, [`MapLevel`] out); this file is the
//! shell's: the page's data file and where the page lives.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use amiga_routeview::json::{Json, esc, list};
use amiga_routeview::site::{self, Category, Entry, PageSpec};
use amiga_routeview::{LevelDump, LiveSpec};

mod level;

pub use level::{
    MapLevel, MapObject, WORLDS, groups, groups_of, items_needed, level_id, map_level, object_name,
    palette, secret_name,
};

/// Every `re/objects-*.toml` as (`re/<file>`, text), sorted by name, gathered by `build.rs`.
pub const TABLES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/tables.rs"));

impl MapLevel {
    /// The page's data, one JSON object, in the shape `amiga_routeview::LevelDump::write_js` names:
    /// a map with no route (empty track, inputs, live and events), plus `exit`.
    pub fn to_json(&self) -> String {
        let mut j = Json::object();
        j.str("level", level::level_id_str(self.world));
        j.str("timing", "app");
        j.num("seed", 1);
        j.num("cols", self.cols);
        j.num("rows", self.rows);
        j.num("coins_needed", self.items_needed);
        j.raw("end", "null");
        j.num("inputs_end", 0);
        j.raw("notes", "[]");
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
                let hex: String = px.iter().map(|p| format!("{p:02x}")).collect();
                format!("\"{ix}\":\"{hex}\"")
            })
            .collect();
        j.raw("tiles", &format!("{{{}}}", tiles.join(",")));
        j.raw("body", &list(self.body.iter().map(u16::to_string)));
        j.raw(
            "objects",
            // Secrets first: the page keys a cell's hover and route events by the last object at
            // that cell, so a coin inside a passage stays the coin.
            &list(self.secrets.iter().chain(&self.objects).map(|o| {
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
            "exit",
            &list(self.exit.iter().map(|(c, r)| format!("[{c},{r}]"))),
        );
        for empty in ["track", "inputs", "live", "events"] {
            j.raw(empty, "[]");
        }
        j.finish()
    }
}

/// A map-only entry (T04 draws it): no route file and no verdict, so the page says "Map only"
/// (DESIGN §2.3); grouped by world as `fd-routeview`'s W1-1 route is.
impl LevelDump for MapLevel {
    fn level_id(&self) -> &str {
        level::level_id_str(self.world)
    }

    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> {
        out.write_all(self.to_json().as_bytes())
    }

    fn entry(&self) -> Entry {
        let id = self.level_id().to_string();
        Entry {
            label: id.clone(),
            group: format!("W{}", self.world),
            id,
            route_file: None,
            verdict: None,
            wall_s: None,
            ref_frames: None,
        }
    }
}

/// The character's offset to the point that names its cell (`char_x + $e`, `char_y + 9`, as
/// fd-core's `Location::of_character` takes it); the page draws the character there too.
const SPOT: (i32, i32) = (0xe, 9);

/// What Forest Dumb's page knows of the game: 16-pixel cells, the character drawn and placed at its
/// spot, an object in play boxed over the cell it stands on, a layer per object group of
/// `re/objects-*.toml` ([`object_name`]), a Secrets layer off at load, and live mode on with the items left (DESIGN §2.5).
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
                "Springs, vanishing blocks, flames, the exit",
                "#ffffff",
                true,
                &[],
            ),
            // Off at load so the map does not spoil them (DESIGN §2.3).
            Category::new("secrets", "Secrets", "--secret", false, &["secret"]),
        ],
        actor: "character".into(),
        anchor: SPOT,
        cell_origin: SPOT,
        live_box: (0, 0, 16, 16),
        live: Some(LiveSpec::new(&[("items_left", "Items left")])),
    }
}

/// The toolkit page as Forest Dumb needs it. It loads a web font; the page opens from disk and
/// makes no request outside it, so the line is dropped and the page falls back to the system's
/// fonts. And a route the toolkit can only call clean, faults or failed is shown as the page's own
/// `unknown` ("No verdict"): a clean route there reads "reaches the exit", which nothing here knows.
pub fn offline(page: &str) -> String {
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

/// The six levels' page under `out` ([`write_levels`] over [`WORLDS`]).
pub fn write_site(
    out: &Path,
    files: &fd_assets::Files,
    program: &[u8],
    route: Option<&dyn LevelDump>,
) -> Result<PathBuf, String> {
    write_levels(out, files, program, &WORLDS, route)
}

/// The page under `out` for `worlds`, in order: `index.html` (with [`spec`], live mode on) and
/// `data/W?-1.js` per world, each map only, read from `files` (disk 2's, unpacked) and `program`
/// (disk 1's `FOREST_DUMB_FOREVER`). `route`, when given, takes the place of its level's map-only
/// entry (W1-1's route from `fd-routeview --script`); a route for a level not listed is an error.
/// Every level is read before anything is written, so a missing file leaves the old page whole.
pub fn write_levels(
    out: &Path,
    files: &fd_assets::Files,
    program: &[u8],
    worlds: &[u8],
    route: Option<&dyn LevelDump>,
) -> Result<PathBuf, String> {
    let mut maps = Vec::new();
    for &w in worlds {
        let level = fd_assets::read_level(w, files).map_err(|e| e.to_string())?;
        maps.push(map_level(w, &level, program)?);
    }
    let mut levels: Vec<&dyn LevelDump> = Vec::new();
    for m in &maps {
        match route {
            Some(r) if r.level_id() == m.level_id() => levels.push(r),
            _ => levels.push(m),
        }
    }
    if let Some(r) = route
        && !maps.iter().any(|m| m.level_id() == r.level_id())
    {
        return Err(format!(
            "a route on {} but no such level written",
            r.level_id()
        ));
    }
    let fail = |e: io::Error| format!("{}: {e}", out.display());
    site::write_site(out, &spec(), &levels).map_err(fail)?;
    let page = out.join("index.html");
    let html = std::fs::read_to_string(&page).map_err(fail)?;
    std::fs::write(&page, offline(&html)).map_err(fail)?;
    Ok(page)
}

/// `games/forest-dumb/build/map` of the main checkout this crate was built in, so a build in any
/// worktree writes the owner's one page (DESIGN §2.9); the build's own checkout when no main one is
/// found. `fd-app` falls back to `<home>/map` with [`out_in`].
pub fn default_out() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    out_in(here).unwrap_or_else(|| {
        let game = here.join("../../..");
        game.canonicalize().unwrap_or(game).join("build/map")
    })
}

/// `games/forest-dumb/build/map` of the main checkout `dir` is in, if any: the git common dir's
/// parent, read from the files git leaves (as `fd-app` finds its bugs folder), not by running git.
pub fn out_in(dir: &Path) -> Option<PathBuf> {
    main_checkout(dir).map(|root| root.join("games/forest-dumb/build/map"))
}

fn main_checkout(dir: &Path) -> Option<PathBuf> {
    for d in dir.ancestors() {
        let git = d.join(".git");
        if git.is_dir() {
            return Some(d.to_path_buf());
        }
        if git.is_file() {
            let text = std::fs::read_to_string(&git).ok()?;
            let own = d.join(text.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim());
            let common = match std::fs::read_to_string(own.join("commondir")) {
                Ok(c) => own.join(c.trim()),
                Err(_) => own,
            };
            return std::fs::canonicalize(common)
                .ok()?
                .parent()
                .map(Path::to_path_buf);
        }
    }
    None
}
