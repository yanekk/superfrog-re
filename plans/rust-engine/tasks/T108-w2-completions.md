# T108 — w2-completions

**Phase:** 2 · **Depends on:** T43, T101, T107; blocks T18 · **Weight:** heavy

Added by T43 (owner, 2026-10-14): T43's "a scripted completion of each level in world 2 matches the
oracle" moved here. T43 built per-level encounter cases (`enemies-w2-*`) meeting the types reachable
from each level's start; a completion meets every type of its level.

## Goal

A scripted run from the start of each of W2-1, W2-2, W2-3 and W2-4 to its exit (every coin the exit
needs collected: W2-1 needs 35), against the original, frame for frame, `resync = false`, with no
exclusion.

Also carried from T43's tests (its encounter cases did not reach them): an enemy taking a heart in
a frame case (none of `enemies-w2-1..3` loses one), an enemy stunned by the spud and then stomped
into a reward, a crumbling ledge breaking under the frog, and the types no case meets yet: the mace
(63, 64), the spikes (71, 72), the swooper (37, W2-4 only) and anything of W2-4's (no `enemies-w2-4`
case: the planner found no way up from its start at 5,73). `examples/plan` takes `hurt` and `kill`
targets for these (T43).

## Tools

- `cargo run --release --example plan -- <level> <col,row>... [--prefix <script>]` (beam search on
  the engine; about 2.5 min a waypoint). It steps the engine unpaced, so check a route in the case
  run (`sf_conform::run_with`), not on the engine alone (FINDINGS, T84/T85 reviews). Waypoints are the
  frog's centre: aim at the row it stands on, not the air.
- `cargo run --release --example enemies_map -- <level>` lists every enemy marker, the start and
  the exit; `tools/export_annotated.py` draws the map with a grid. W2-1's right half (mace 205,26,
  spikes 237–249,28, exit 257,21) is walled off from its start at column 144: find the way (marker 22
  cells at 116,39 and 143,37 and the shaft at 191–194 are candidates).

## Files

`engine/conform/{cases,scripts}/w2-*-complete.*`; `engine/crates/sf-conform/tests/frog_cases.rs`
(whether each runs in the test block: say in `DESIGN.md § Environment` if one is too slow).

## Done when

- [ ] Four cases, each ending with the level's exit (`level_outcome` `Completed`), green against the
      oracle with no re-sync, in `sf-conform -- all`.
