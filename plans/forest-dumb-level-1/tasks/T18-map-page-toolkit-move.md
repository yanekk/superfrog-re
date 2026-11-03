# T18 — map-page-toolkit-move

**Phase:** 4 · **Depends on:** T17 · **Weight:** medium

## Goal
Move the page's game-free half to toolkit `amiga-routeview`, with its game data behind a small interface.
## Design sections this implements
§2.10, §3.6.
## Reuses
`sf-routeview/src/{site,json,guide,verdict}.rs`, `page.html`. `dump.rs` and `levels` stay per game.
## Files
`toolkit/crates/amiga-routeview/`, `games/superfrog/engine/crates/sf-routeview/**`, smoke labels.
## Interface
```
pub struct PageSpec { pub title: String, pub cell_size: (u16,u16), pub categories: Vec<Category> }
pub trait LevelDump { fn level_id(&self) -> &str; fn write_js(&self, out: &mut dyn Write) -> io::Result<()>; }
pub fn write_site(root: &Path, spec: &PageSpec, levels: &[&dyn LevelDump]) -> io::Result<()>
pub fn progress(events: &[Event], facts: &Facts) -> Vec<Event>   // stall events
```
`sf-bench::results` dependency becomes a parameter.
## Tests
- [ ] `sf-routeview/tests/replay.rs` and its `src` tests pass unchanged, including W1-1's exit at 2844 frames.
- [ ] Toolkit test writes a site for a made-up level and checks the manifest and that `index.html` loads by script tags.
- [ ] Tier name check.
## Done when
- [ ] Superfrog's page has the same content, byte-compared on one level.
- [ ] `page.html` holds no game names or fixed cell constants.
- [ ] Each moved crate keeps its `opt-level` override in the root manifest of every workspace that builds it (toolkit, both games), and the bench or page timing matches the pre-move run.
