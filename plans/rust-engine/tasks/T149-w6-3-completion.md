# T149 — w6-3-completion

**Split from:** T104 · **Phase:** 2 · **Depends on:** T104; blocks T18, T146 · **Weight:** medium

Carries T104's done-when for W6-3 (first map). W6-3alt's is done (`enemies-w6-3-alt-complete`).

## Goal

`engine/conform/cases/enemies-w6-3-complete.toml` to W6-3's exit (all 138 coins), nothing left
out, `resync = false`; in the test block if green, else outside it naming T146.

## Files

- `engine/conform/scripts/enemies-w6-3-complete.txt`, `engine/conform/cases/enemies-w6-3-complete.toml`
- `engine/crates/sf-conform/tests/enemies_w6_cases.rs`, if the case is green.

## Interface

None new: a case in the existing format (`sync`, `script`, `frames`, `resync = false`), its
script made with `examples/plan_exit_w6`.

## What is known (T104)

- `enemies-w6-3-alt-complete`'s script also plays W6-3 identically to frame 3 105 (the maps
  differ only past column 181 and in two maces' rows). T104 took its first 3 000 frames, then
  planned with `--via` 118,7, 121,5, 150,6, 152,12, 160,6, 171,7, 181,10, 191,6, 201,11,
  179,28, 150,36, 120,40 (the alt's route: up the 115-118 shaft by the wall top at 113-114,9,
  along the top corridor under the maces, down to switch 49 at 120,41-42 whose gate is 87,33),
  then the coin phase: 20 coins left at about frame 6 700.
- `/tmp/t104/W6-3.txt`: that script (may exist). The exit is 229-230,23-25.

## What T104 left

- `examples/plan_exit_w6` (T104): the coin phase (`--coins N`; switches pressed as targets, a switch whose number no gate has is skipped), a gravity field over the live map (jump 7 rows, a pad 24, marker 62 a jump-through floor, 44 a lift, a cell over spikes enterable within `--gap N` cells of safe ground; a full-speed run jump crosses ~17 cells), passed-over coins coming back after 12 chunks, a search on the original when every engine path dies there, switches before a walled exit. Debug: `PLAN_WALLS=c0,r0,c1,r1` (walls crossed per cell), `PLAN_DUMP`, `PLAN_EXIT_ONLY`; `PLAN_TRACE` follows the field downhill ignoring move rules (misleading). Budget: `--frames` counts the prefix (default 12 000).
- The planner stalls where a route needs a run-up or a precise hop: find the hop by brute force on the original (left/right/up+right holds of a few lengths from a saved state, success = a cell box reached on the ground) and append it to the script, then resume with `--prefix`.
- An exit is reached when `level_state` (h0+$a06, long) is 1 for two frames; the planner can miss it between chunks, so check a finished script on the original and cut it one frame after.
- T104's scratch files under `/tmp/t104/` may still exist on this machine (not guaranteed): named below.

## Tests

- [ ] `enemies-w6-3-complete` reaches the exit on the original; green or its divergence named.

## Done when

- [ ] The case exists and is in the test block if green.
