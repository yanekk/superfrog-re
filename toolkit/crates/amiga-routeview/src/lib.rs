//! The route viewer's game-free half: `index.html` (the viewer, with the routes' manifest and the
//! game's [`site::PageSpec`] in it) and `data/<id>.js` per route, written from what a game's replay
//! hands over as a [`site::LevelDump`]. The verdict a route gets ([`verdict::verdict`]) and where a
//! route that reaches no exit stops making progress ([`verdict::progress`]) are decided here from
//! plain facts the game reads off its own state.

pub mod guide;
pub mod json;
pub mod site;
pub mod verdict;

pub use site::{Category, Entry, LevelDump, PageSpec, write_site};
pub use verdict::{Bench, Event, Facts, Outcome, Stall, Status, Verdict, progress};
