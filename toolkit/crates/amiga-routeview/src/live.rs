//! The page's live mode, both halves of its file: what a running game writes four times a second
//! ([`LiveFrame`], [`write_live`]) and what the page needs to read it ([`LiveSpec`]). The page
//! re-reads the file by inserting a script tag, because a page opened from disk cannot fetch; so the
//! file is JavaScript, `window.LIVE = {...};`, and is replaced whole by rename so a reader never
//! sees half of one.

use std::io;
use std::path::Path;

use crate::json::{Json, esc, list};

/// The live file's name, beside `index.html`; [`write_live`] writes it.
pub const LIVE_FILE: &str = "live.js";

/// A page's live mode, set in [`crate::PageSpec::live`]. `None` there leaves the page as it was:
/// no file is ever asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveSpec {
    /// The file the page re-reads, relative to `index.html`: [`LIVE_FILE`].
    pub file: String,
    /// How often the page re-reads it, in milliseconds.
    pub poll_ms: u32,
    /// A frame whose time stamp is older than this, by the page's clock, is shown as "App not
    /// running", never as the current position.
    pub stale_ms: u32,
    /// Each counter the page shows, in order: its key in [`LiveFrame::counters`] and its label.
    pub counter_labels: Vec<(String, String)>,
}

impl LiveSpec {
    /// `live.js`, read every 250 ms, stale after 3 s, with these counters.
    pub fn new(counter_labels: &[(&str, &str)]) -> LiveSpec {
        LiveSpec {
            file: LIVE_FILE.into(),
            poll_ms: 250,
            stale_ms: 3000,
            counter_labels: counter_labels.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect(),
        }
    }
}

/// One sample of play, as the game reads it off its state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LiveFrame {
    /// When it was taken, in milliseconds since the Unix epoch: the writer's clock, never the core's.
    pub t_ms: u64,
    /// In a level, not on a title, menu, load or game-over screen.
    pub playing: bool,
    /// The level's id on the page (a manifest entry's `id`).
    pub level: Option<String>,
    /// The character in level pixels, at the point the game's cell rule uses.
    pub actor: Option<(i32, i32)>,
    /// The map cell the character stands on.
    pub cell: Option<(u16, u16)>,
    /// Creatures in play: level pixels and the object cat whose shape draws them.
    pub actors: Vec<(i32, i32, String)>,
    /// Object cells whose value differs from the level file's: column, row, value (0: gone).
    pub cells: Vec<(u16, u16, u8)>,
    /// Counters by key, shown with [`LiveSpec::counter_labels`].
    pub counters: Vec<(String, i64)>,
}

impl LiveFrame {
    /// The file's text: `window.LIVE = {...};` and a newline.
    pub fn to_js(&self) -> String {
        let pair = |a: String, b: String| format!("[{a},{b}]");
        let mut j = Json::object();
        j.num("t", self.t_ms);
        j.bool("playing", self.playing);
        match &self.level {
            Some(l) => j.str("level", l),
            None => j.raw("level", "null"),
        }
        j.raw("actor", &self.actor.map_or("null".into(), |(x, y)| pair(x.to_string(), y.to_string())));
        j.raw("cell", &self.cell.map_or("null".into(), |(c, r)| pair(c.to_string(), r.to_string())));
        j.raw("actors", &list(self.actors.iter().map(|(x, y, cat)| format!("[{x},{y},\"{}\"]", esc(cat)))));
        j.raw("cells", &list(self.cells.iter().map(|(c, r, v)| format!("[{c},{r},{v}]"))));
        let mut counters = Json::object();
        for (k, v) in &self.counters {
            counters.num(k, v);
        }
        j.raw("counters", &counters.finish());
        format!("window.LIVE = {};\n", j.finish())
    }
}

/// Writes `frame` as `dir/live.js`: to `live.js.tmp` first, then renamed over it, so a page
/// reading at any moment gets the previous frame or this one, whole. The folder must exist.
pub fn write_live(dir: &Path, frame: &LiveFrame) -> io::Result<()> {
    let tmp = dir.join(format!("{LIVE_FILE}.tmp"));
    std::fs::write(&tmp, frame.to_js())?;
    std::fs::rename(&tmp, dir.join(LIVE_FILE))
}
