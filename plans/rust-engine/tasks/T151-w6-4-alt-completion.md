# T151 — w6-4-alt-completion

**Split from:** T104 · **Phase:** 2 · **Depends on:** T104, T150; blocks T18, T146 · **Weight:** small

Carries T104's done-when for W6-4's second map (`sync = { level = "W6-4alt" }`).

## Goal

`engine/conform/cases/enemies-w6-4-alt-complete.toml` to W6-4alt's exit, nothing left out,
`resync = false`; in the test block if green, else outside it naming T146.

## What is known (T104)

- W6-4alt differs from W6-4 in 4 map rows and has no switch 51. T104's planner made identical
  progress on both for ~6 700 frames. Start from T150's script: find the first frame the frog
  differs on the two originals, and replan from just before it.

## What T104 left

- `examples/plan_exit` (T104): the coin phase (`--coins N`; switches pressed as targets, a switch whose number no gate has is skipped), a gravity field over the live map (jump 7 rows, a pad 24, marker 62 a jump-through floor, 44 a lift, a cell over spikes enterable within `--gap N` cells of safe ground; a full-speed run jump crosses ~17 cells), passed-over coins coming back after 12 chunks, a search on the original when every engine path dies there, switches before a walled exit. Debug: `PLAN_WALLS=c0,r0,c1,r1` (walls crossed per cell), `PLAN_DUMP`, `PLAN_EXIT_ONLY`; `PLAN_TRACE` follows the field downhill ignoring move rules (misleading). Budget: `--frames` counts the prefix (default 12 000).
- The planner stalls where a route needs a run-up or a precise hop: find the hop by brute force on the original (left/right/up+right holds of a few lengths from a saved state, success = a cell box reached on the ground) and append it to the script, then resume with `--prefix`.
- An exit is reached when `level_state` (h0+$a06, long) is 1 for two frames; the planner can miss it between chunks, so check a finished script on the original and cut it one frame after.
- T104's scratch files under `/tmp/t104/` may still exist on this machine (not guaranteed): named below.

## Tests

- [ ] `enemies-w6-4-alt-complete` reaches the exit on the original; green or its divergence named.

## Done when

- [ ] The case exists and is in the test block if green.
