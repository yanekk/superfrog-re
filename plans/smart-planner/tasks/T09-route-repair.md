# T09 — route-repair

**Phase:** 1 · **Depends on:** T08 · **Weight:** medium

Rewritten by the 2026-10-22 amendment (D19, D21): repair is a guide path plus an archive seed, and
guide paths are this task's.

## Goal

Steer the explorer along a guide path, from a route or a waypoint file, and start it from an old
route: for whole-game legs whose carried state differs from the standalone start, for original mode's
repair after a parting (T10), and for the benchmark's guided runs (each level's committed route).

## Design sections this implements

DESIGN §2.7, §2.11, D21.

## Files

- `engine/crates/sf-plan/src/core/guide.rs` (`GuidePoint`'s weight; the `GuidePoint` type is T08's)
- `engine/crates/sf-plan/src/core/repair.rs`
- `engine/crates/sf-plan/src/guide_file.rs` (shell: parses a waypoint file)
- `engine/crates/sf-plan/src/trace.rs` (shell: plays a route script at its own start and samples
  the frog's cell and coins left every 8 frames; on the original through `sf_conform::sync` for
  `original` or no timing line, on the engine from the `start_level` state for `app`, which T12
  wires; T10 reuses it)
- `engine/crates/sf-plan/Cargo.toml` (`sf-conform` from a dev-dependency to a dependency: the shell
  now reaches the oracle; `tests/purity.rs` still bars it from `src/core/`)
- `engine/crates/sf-plan/tests/guide.rs`, `engine/crates/sf-plan/tests/repair.rs`

## Interface

```rust
/// Points sampled every 8 frames from a run the shell played: frog cell and coins left.
pub fn points_from_trace(trace: &[(Cell, u16)]) -> Vec<GuidePoint>;
/// The choice weight factor for a cell key, given the archive's furthest guide index (DESIGN §2.11).
pub struct GuideIndex { /* (coarse col, coarse row, band | any) -> furthest point index */ }
impl GuideIndex { pub fn new(points: &[GuidePoint]) -> Self; pub fn weight(&self, key: &CellKey, p_max: u32) -> f64; pub fn index(&self, key: &CellKey) -> Option<u32>; }
/// `--follow`: the old route's inputs as T08's `seed_inputs`, its trace as the guide.
pub fn follow_options(opt: Options, old_inputs: Vec<FrameInput>, old_trace: &[(Cell, u16)]) -> Options;
pub fn parse_waypoints(text: &str) -> Result<Vec<GuidePoint>, String>;   // shell side, guide_file.rs
pub fn trace_original(sync: &SyncPoint, script: &InputScript) -> Result<Vec<(Cell, u16)>, String>; // shell, trace.rs
```

Waypoint file: one `col,row` or `col,row,coins_left` per line, `#` comments, blank lines ignored. A
point without coins matches any band.

## Tests

- [ ] `parse_waypoints` reads both line forms and comments; a bad line is an error naming its number.
- [ ] `GuideIndex`: a cell on the path's last point weighs more than one on its first; a cell off the
      path weighs 0.15; a point with coins matches only its band.
- [ ] W6-2 engine-only, guided by `enemies-w6-2-complete`'s trace, reaches the exit within 5 minutes
      (`#[ignore]`, time noted; the experiment: 80 s).
- [ ] An old route replayed from its own start through `follow_options` exits at once (the seed alone
      exits) and is returned unchanged.
- [ ] The same route from a start with one input frame dropped (a carried-state stand-in): the
      explorer exits, faster than unguided on the same level and seed (frames counted, not time).
- [ ] Deterministic for one seed, guided.

## Done when

- [ ] Tests green; one set-B leg (W2-1, carried RNG, FINDINGS T205) planned engine-only with
      `--follow` of its standalone route, rounds and time in a FINDINGS row.
