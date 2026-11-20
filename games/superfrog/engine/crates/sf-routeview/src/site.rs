//! The page is the toolkit's (`amiga_routeview::site`, moved there in forest-dumb-level-1 T18);
//! this module says what Superfrog's map holds ([`spec`]) and hands a replayed route over to it
//! ([`Route`]).

use std::io::{self, Write};

use amiga_routeview::LevelDump;
pub use amiga_routeview::site::{Category, Entry, PageSpec, id_of, write_data, write_index, write_level, write_site};

use crate::dump::Dump;

/// Superfrog's page: 16×16 cells; the frog's x, y are its record's (+0, +2), so the planner's cell
/// is ((x+$17)/16, (y-$10)/16), and y sits on the ground line when landed, so the body's middle is
/// about (x+$17, y-10); a live object's box is its x-2, y-14, 20×18. The layers are the object
/// cats `dump::objects` gives.
pub fn spec() -> PageSpec {
    PageSpec {
        title: "Superfrog".into(),
        cell_size: (16, 16),
        categories: vec![
            Category::new("coins", "Coins", "--accent", true, &["coin"]),
            Category::new("enemies", "Enemies", "--enemy", true, &["enemy"]),
            Category::new("pickups", "Collectibles", "--pickup", true, &["pickup"]),
            Category::new("hazards", "Spikes", "--hazard", true, &["hazard"]),
            Category::new("secrets", "Secret walls", "--secret", true, &["secret"]),
            Category::new("misc", "Exits, springs, switches, scenery", "#ffffff", true, &[]),
            Category::new("terrain", "Ice, conveyors, other markers", "#9aa7a0", false, &["other", "surface"]),
        ],
        actor: "frog".into(),
        anchor: (0x17, -10),
        cell_origin: (0x17, -0x10),
        live_box: (-2, -14, 20, 18),
    }
}

/// A replayed route with its row in the switcher.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spec_names_every_cat_the_dump_gives() {
        let s = spec();
        // dump.rs: `Object::cat`'s values; `exit`, `fixture`, `spring` and `switch` share the
        // catch-all layer.
        for cat in ["coin", "enemy", "pickup", "hazard", "secret", "other", "surface"] {
            assert_eq!(s.categories.iter().filter(|c| c.cats.iter().any(|x| x == cat)).count(), 1, "{cat}");
        }
        assert_eq!(s.categories.iter().filter(|c| c.cats.is_empty()).count(), 1);
    }
}
