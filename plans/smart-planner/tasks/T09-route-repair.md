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

- `engine/crates/sf-plan/src/core/guide.rs` (the bodies of `GuideIndex`; the types, signatures and the
  call from `explore.rs` are T08's, so this task does not edit `explore.rs`, `mod.rs` or `lib.rs`)
- `engine/crates/sf-plan/src/core/repair.rs` (T08's stub)
- `engine/crates/sf-plan/src/guide_file.rs` (shell: parses a waypoint file)
- `engine/crates/sf-plan/src/trace.rs` (shell: plays a route script at its own start and samples
  the frog's cell and coins left every 8 frames; on the original through `sf_conform::sync` for
  `original` or no timing line, on the engine from the `start_level` state for `app`, which T12
  wires; T10 reuses it)
- `engine/crates/sf-plan/Cargo.toml` (`sf-conform` from a dev-dependency to a dependency and
  `sf-oracle` added, both already in the workspace: the shell now reaches the oracle and names
  `Amiga`; `tests/purity.rs` still bars them from `src/core/`)
- `engine/crates/sf-plan/tests/guide.rs`, `engine/crates/sf-plan/tests/repair.rs`

## Interface

```rust
/// Points sampled every 8 frames from a run the shell played: frog cell and coins left.
pub fn points_from_trace(trace: &[(Cell, u16)]) -> Vec<GuidePoint>;
/// The choice weight factor for a cell key, given the archive's furthest guide index (DESIGN §2.11).
pub struct GuideIndex { /* (coarse col, coarse row, band | any) -> furthest point index */ }
impl GuideIndex { pub fn new(points: &[GuidePoint]) -> Self; pub fn weight(&self, key: &CellKey, p_max: u32) -> f64; pub fn index(&self, key: &CellKey) -> Option<u32>; }
/// `--follow`: the old route's inputs as T08's `seed_inputs`, its trace as the guide (replacing `opt.guide`;
/// `sf-plan` re-applies an explicit `--guide` after it, DESIGN §2.7).
pub fn follow_options(opt: Options, old_inputs: Vec<FrameInput>, old_trace: &[(Cell, u16)]) -> Options;
/// A route played on the engine from `start`, sampled every 8 frames (pure, core::repair): T10's
/// rest-of-plan guide, T12's app-timing guide.
pub fn trace_engine(start: &Game, inputs: &[FrameInput]) -> Vec<(Cell, u16)>;
pub fn parse_waypoints(text: &str) -> Result<Vec<GuidePoint>, String>;   // shell side, guide_file.rs
/// The caller starts the oracle: `sync::reach(AtLevel)` for sets A and C, `sync::at_snapshot_file`
/// for a set-B leg (replaying `game-whole` to the leg would cost minutes). Shell, trace.rs.
pub fn trace_original(amiga: &mut Amiga, script: &InputScript, frames: u32) -> Result<Vec<(Cell, u16)>, String>;
```

Waypoint file: one `col,row` or `col,row,coins_left` per line, `#` comments, blank lines ignored. A
point without coins matches any band.

## Tests

A test over ~30 s is `#[ignore]` with its time noted (DESIGN §5).

- [ ] `parse_waypoints` reads both line forms and comments; a bad line is an error naming its number.
- [ ] `GuideIndex`: a cell on the path's last point weighs more than one on its first; a cell off the
      path weighs 0.15; a point with coins matches only its band.
- [ ] W6-2 engine-only, guided by `enemies-w6-2-complete`'s trace, reaches the exit within 5 minutes
      (`#[ignore]`, time noted; the experiment: 80 s).
- [ ] An engine-timed route (T08's fixture `tests/fixtures/w1-1-engine.txt`; committed routes are
      original-timed and die early on the engine) replayed from its own start through
      `follow_options` exits at once (the seed alone exits) and is returned unchanged
      (`RunOptions::default()`, shortening off).
- [ ] `trace_engine` of that fixture ends at an exit cell; `trace_original` of `complete-w1-1` from
      `AtLevel` gives points every 8 frames that start at the level's start cell.
- [ ] The same route from a start with one input frame dropped (a carried-state stand-in): the
      explorer exits, faster than unguided on the same level and seed (frames counted, not time).
- [ ] Deterministic for one seed, guided.

## Done when

- [ ] Tests green; one set-B leg (W2-1, carried RNG, FINDINGS T205) planned engine-only from its leg
      snapshot with `follow_options` of its standalone route (the CLI flag is T11's), rounds and time
      in a FINDINGS row.
