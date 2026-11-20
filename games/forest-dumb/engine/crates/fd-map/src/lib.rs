//! Each world's level as the map page draws it, with no engine run (plan forest-dumb-map-page
//! DESIGN §2.1, §2.2, §3.2): tiles, palette, every object with its layer, the exit cells and the
//! items the exit needs. [`level`] is the pure side (bytes in, [`MapLevel`] out); this file is the
//! shell's: the page's data file and where the page lives.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use amiga_routeview::LevelDump;
use amiga_routeview::json::{Json, esc, list};

mod level;

pub use level::{
    MapLevel, MapObject, WORLDS, groups, groups_of, items_needed, level_id, map_level, object_name,
    palette,
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
            "exit",
            &list(self.exit.iter().map(|(c, r)| format!("[{c},{r}]"))),
        );
        for empty in ["track", "inputs", "live", "events"] {
            j.raw(empty, "[]");
        }
        j.finish()
    }
}

/// A map-only entry (T04 draws it): the default switcher row, no route file.
impl LevelDump for MapLevel {
    fn level_id(&self) -> &str {
        level::level_id_str(self.world)
    }

    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> {
        out.write_all(self.to_json().as_bytes())
    }
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
