//! The page: `index.html` (the viewer, with the routes' manifest and the game's [`PageSpec`] in it)
//! and `data/<id>.js` per route. Script tags, not fetch, load the data, so the page also works
//! opened from disk; a route loads when it is picked, so two dozen levels' tiles never load at once.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::json::{Json, esc, list, strs};
use crate::verdict::{Status, Verdict};

const PAGE: &str = include_str!("page.html");

/// What the page needs to know of a game: its name, its map's cell size, where its character is
/// drawn and which cell it stands on, and the map's object layers.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSpec {
    /// The game's name: the page is titled "<title> Route Viewer", its heading "<title> route".
    pub title: String,
    /// A map cell's width and height in level pixels; a level's tiles are one cell each.
    pub cell_size: (u16, u16),
    /// The map's object layers in legend order. An object's `cat` puts it on the first layer
    /// listing that cat; a cat no layer lists goes on the first layer with an empty `cats`.
    pub categories: Vec<Category>,
    /// What the page calls the character the route moves ("Follow <actor>").
    pub actor: String,
    /// Added to a track position: the point the route line and the character are drawn at.
    pub anchor: (i32, i32),
    /// Added to a track position before dividing by the cell size: the planner's cell the
    /// character stands on (the hover's and readout's cell, the time-spent layer).
    pub cell_origin: (i32, i32),
    /// A live object's box from its position: x and y offsets, width and height.
    pub live_box: (i32, i32, u16, u16),
}

/// One object layer in the legend.
#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    /// The layer's key (`[a-z]+`, not one of the page's own layers: route, guide, heat, live,
    /// events).
    pub key: String,
    pub name: String,
    /// A colour: `#rrggbb`, or a page colour token such as `--accent`, `--enemy`, `--pickup`,
    /// `--secret`, `--hazard`.
    pub color: String,
    /// Shown when the page opens.
    pub on: bool,
    /// The object cats on this layer. The page draws the cats `coin`, `enemy`, `pickup`, `hazard`
    /// and `secret` each in its own shape, and any other as a square.
    pub cats: Vec<String>,
}

impl Category {
    pub fn new(key: &str, name: &str, color: &str, on: bool, cats: &[&str]) -> Category {
        Category {
            key: key.into(),
            name: name.into(),
            color: color.into(),
            on,
            cats: cats.iter().map(|c| c.to_string()).collect(),
        }
    }
}

/// One route on the page.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The switcher's key and the data file's name: `[A-Za-z0-9._-]`.
    pub id: String,
    pub label: String,
    /// The switcher's row: a world (`W1`) or a level.
    pub group: String,
    /// The route file, shown under the title; `None` for a level drawn with no route.
    pub route_file: Option<String>,
    pub verdict: Verdict,
    /// The planner's planning time and route lengths, for the summary.
    pub wall_s: Option<f64>,
    pub ref_frames: Option<u32>,
}

/// One route's data, as a game's replay hands it over.
pub trait LevelDump {
    /// The route's id on the page (its data file's name): `[A-Za-z0-9._-]`, see [`id_of`].
    fn level_id(&self) -> &str;
    /// Writes the route's data, one JSON object: `level`, `cols`, `rows`, `seed`, `timing`,
    /// `palette`, `tiles`, `body`, `objects`, `track`, `inputs`, `live`, `events`, `notes`,
    /// `coins_needed`, `end`, `inputs_end`, and optionally `guide`, as the page reads them.
    fn write_js(&self, out: &mut dyn Write) -> io::Result<()>;
    /// Its row in the switcher. By default the id is the label and the group, with no route file,
    /// no planner result and a clean verdict with nothing to say.
    fn entry(&self) -> Entry {
        let id = self.level_id().to_string();
        Entry {
            label: id.clone(),
            group: id.clone(),
            id,
            route_file: None,
            verdict: Verdict { status: Status::Clean, faults: vec![], info: vec![] },
            wall_s: None,
            ref_frames: None,
        }
    }
}

/// `name` as a data file's id: anything but `[A-Za-z0-9._-]` becomes `_`.
pub fn id_of(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect()
}

/// Writes `data/<id>.js` for one route.
pub fn write_data(out: &Path, id: &str, json: &str) -> io::Result<()> {
    std::fs::create_dir_all(out.join("data"))?;
    let js = format!("window.ROUTE_DATA=window.ROUTE_DATA||{{}};ROUTE_DATA[\"{}\"]={json};\n", esc(id));
    std::fs::write(out.join("data").join(format!("{id}.js")), js)
}

/// Writes `data/<id>.js` for one level's dump.
pub fn write_level(out: &Path, level: &dyn LevelDump) -> io::Result<()> {
    let mut json = Vec::new();
    level.write_js(&mut json)?;
    let json = String::from_utf8(json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_data(out, level.level_id(), &json)
}

/// Writes every level's data file and `index.html` (with its `<!doctype>` skeleton) under `root`.
pub fn write_site(root: &Path, spec: &PageSpec, levels: &[&dyn LevelDump]) -> io::Result<()> {
    let mut entries = Vec::new();
    for l in levels {
        write_level(root, *l)?;
        entries.push(l.entry());
    }
    write_index(root, spec, &entries, false).map(|_| ())
}

fn manifest(entries: &[Entry]) -> String {
    list(entries.iter().map(|e| {
        let mut j = Json::object();
        j.str("id", &e.id);
        j.str("label", &e.label);
        j.str("group", &e.group);
        j.str("status", e.verdict.status.word());
        j.raw("faults", &strs(e.verdict.faults.iter()));
        j.raw("info", &strs(e.verdict.info.iter()));
        j.str("data", &format!("data/{}.js", e.id));
        match &e.route_file {
            Some(f) => j.str("route_file", f),
            None => j.raw("route_file", "null"),
        }
        if e.wall_s.is_some() || e.ref_frames.is_some() {
            let mut b = Json::object();
            b.opt("wall_s", e.wall_s);
            b.opt("ref_frames", e.ref_frames);
            j.raw("bench", &b.finish());
        } else {
            j.raw("bench", "null");
        }
        j.finish()
    }))
}

fn spec_json(spec: &PageSpec) -> String {
    let pair = |(a, b): (i32, i32)| format!("[{a},{b}]");
    let mut j = Json::object();
    j.str("actor", &spec.actor);
    j.raw("cell", &pair((spec.cell_size.0 as i32, spec.cell_size.1 as i32)));
    j.raw("anchor", &pair(spec.anchor));
    j.raw("cell_origin", &pair(spec.cell_origin));
    let (bx, by, bw, bh) = spec.live_box;
    j.raw("live_box", &format!("[{bx},{by},{bw},{bh}]"));
    j.raw(
        "layers",
        &list(spec.categories.iter().map(|c| {
            let mut l = Json::object();
            l.str("key", &c.key);
            l.str("name", &c.name);
            l.str("color", &c.color);
            l.bool("on", c.on);
            l.raw("cats", &strs(c.cats.iter()));
            l.finish()
        })),
    );
    j.finish()
}

/// `s` as HTML text.
fn html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Writes `index.html`. `bare` leaves out the `<!doctype>`/`<head>` skeleton, for a host that
/// adds its own (a published artifact); without it the file opens straight from disk.
pub fn write_index(out: &Path, spec: &PageSpec, entries: &[Entry], bare: bool) -> io::Result<PathBuf> {
    // The manifest goes in last: a route file's name is free text and must not be read as a
    // placeholder.
    let mut page = PAGE
        .replace("__TITLE__", &html(&spec.title))
        .replace("__ACTOR__", &html(&spec.actor))
        .replace("__SPEC__", &spec_json(spec))
        .replace("__MANIFEST__", &manifest(entries));
    if !bare {
        page = format!(
            "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\">\
             <style>body{{margin:0}}</style></head><body>\n{page}\n</body></html>\n"
        );
    }
    std::fs::create_dir_all(out)?;
    let path = out.join("index.html");
    std::fs::write(&path, page)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> PageSpec {
        PageSpec {
            title: "Made-up <Game>".into(),
            cell_size: (8, 12),
            categories: vec![Category::new("gems", "Gems", "--accent", true, &["coin"]), Category::new("rest", "Rest", "#ffffff", false, &[])],
            actor: "hero".into(),
            anchor: (4, -6),
            cell_origin: (4, -12),
            live_box: (0, -8, 8, 8),
        }
    }

    #[test]
    fn ids_are_file_safe() {
        assert_eq!(id_of("w64 ordered/s3+"), "w64_ordered_s3_");
        assert_eq!(id_of("W1-4.v2"), "W1-4.v2");
    }

    #[test]
    fn the_manifest_and_spec_sit_in_the_page_and_cannot_close_its_script() {
        let dir = std::env::temp_dir().join(format!("amiga-routeview-site-{}", std::process::id()));
        let e = Entry {
            id: "L1".into(),
            label: "L1".into(),
            group: "A".into(),
            route_file: Some("a</script>b__TITLE__.txt".into()),
            verdict: Verdict { status: Status::Faults, faults: vec!["lost 1 heart".into()], info: vec![] },
            wall_s: Some(60.0),
            ref_frames: None,
        };
        let p = write_index(&dir, &spec(), &[e], false).unwrap();
        let html = std::fs::read_to_string(&p).unwrap();
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains(r#""id":"L1","label":"L1","group":"A","status":"faults","faults":["lost 1 heart"]"#));
        assert!(html.contains(r#"a<\/script>b__TITLE__.txt"#), "a route file name is not a placeholder");
        assert!(html.contains("<title>Made-up &lt;Game&gt; Route Viewer</title>"));
        assert!(html.contains(">Follow hero</button>"));
        assert!(html.contains(r#""actor":"hero","cell":[8,12],"anchor":[4,-6],"cell_origin":[4,-12],"live_box":[0,-8,8,8]"#));
        assert!(html.contains(r#"{"key":"gems","name":"Gems","color":"--accent","on":true,"cats":["coin"]}"#));
        assert!(!html.contains("__MANIFEST__") && !html.contains("__SPEC__") && !html.contains("__ACTOR__"));
        let bare = std::fs::read_to_string(write_index(&dir, &spec(), &[], true).unwrap()).unwrap();
        assert!(bare.starts_with("<title>"));
        write_data(&dir, "L1", "{\"a\":1}").unwrap();
        let js = std::fs::read_to_string(dir.join("data/L1.js")).unwrap();
        assert_eq!(js, "window.ROUTE_DATA=window.ROUTE_DATA||{};ROUTE_DATA[\"L1\"]={\"a\":1};\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
