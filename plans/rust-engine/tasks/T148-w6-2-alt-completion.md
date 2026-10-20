# T148 — w6-2-alt-completion

> **Amended** by the owner, 2026-10-15. "Green", "matches" and "frame-exact" below mean DESIGN §2.1's
> faithful checks (T180's default mode); a completion case is also judged alone (check 4, T182).
> Re-sync pins, `resync = false` and `--strict` are not required. Dependencies on dropped tasks
> are gone; PLAN.md's table is the current list. Where this doc and DESIGN §2.1 disagree, §2.1 wins.

**Split from:** T104 · **Phase:** 2 · **Depends on:** T104, T147; blocks T18, T146 · **Weight:** small

Carries T104's done-when for W6-2's second map (`sync = { level = "W6-2alt" }`).

## Goal

`engine/conform/cases/enemies-w6-2-alt-complete.toml` to W6-2alt's exit, nothing left out,
`resync = false`; in the test block if green, else outside it naming T146.

## Files

- `engine/conform/scripts/enemies-w6-2-alt-complete.txt`, `engine/conform/cases/enemies-w6-2-alt-complete.toml`
- `engine/crates/sf-conform/tests/enemies_w6_cases.rs`, if the case is green.

## Interface

None new: a case in the existing format (`sync`, `script`, `frames`, `resync = false`), its
script made with `examples/plan_exit_w6`.

## What is known (T104)

- W6-2alt differs from W6-2 in 6 map rows; T104's planner made identical progress on both for
  the first ~6 400 frames. Start from T147's script: play it on W6-2alt, find the first frame the
  frog's position differs (compare both originals in lockstep), and replan from just before it.

## What T104 left

- `examples/plan_exit_w6` (T104): the coin phase (`--coins N`; switches pressed as targets, a switch whose number no gate has is skipped), a gravity field over the live map (jump 7 rows, a pad 24, marker 62 a jump-through floor, 44 a lift, a cell over spikes enterable within `--gap N` cells of safe ground; a full-speed run jump crosses ~17 cells), passed-over coins coming back after 12 chunks, a search on the original when every engine path dies there, switches before a walled exit. Debug: `PLAN_WALLS=c0,r0,c1,r1` (walls crossed per cell), `PLAN_DUMP`, `PLAN_EXIT_ONLY`; `PLAN_TRACE` follows the field downhill ignoring move rules (misleading). Budget: `--frames` counts the prefix (default 12 000).
- The planner stalls where a route needs a run-up or a precise hop: find the hop by brute force on the original (left/right/up+right holds of a few lengths from a saved state, success = a cell box reached on the ground) and append it to the script, then resume with `--prefix`.
- An exit is reached when `level_state` (h0+$a06, long) is 1 for two frames; the planner can miss it between chunks, so check a finished script on the original and cut it one frame after.
- T104's scratch files under `/tmp/t104/` may still exist on this machine (not guaranteed): named below.

## Tests

- [ ] `enemies-w6-2-alt-complete` reaches the exit on the original; green or its divergence named.

## Done when

- [ ] The case exists and is in the test block if green.
