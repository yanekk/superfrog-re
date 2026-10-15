# T147 — w6-2-completion

**Split from:** T104 · **Phase:** 2 · **Depends on:** T104; blocks T18, T146 · **Weight:** medium

Carries T104's done-when for W6-2: a scripted run from the first frame through every coin to the
exit (`level_state` 1), matching the original with `resync = false`, as
`engine/conform/cases/enemies-w6-2-complete.toml`.

## Goal

`enemies-w6-2-complete` (script `enemies-w6-2-complete.txt`) to W6-2's exit; in the test block
(`tests/enemies_w6_cases.rs`) if green, else outside it naming T146.

## What is known (T104)

- W6-2's coins (`coins_left_for_exit` 97 at load) were all taken by about frame 10 900; T104's
  partial script matched the original through 7 488 frames.
- The exit (155-156,26-28) is behind the gate of switch 50 (pair 96,4-5, gate anchor 159,25).
  The switch sits on a ledge (96-101, standing row 6) in the coin room. The way up, found on the
  original from the frog on the floor at 110,16: R 4, U+R 20, R 16 lands on the block top at
  120,11; from there R 12, L 16, U+L 36, L 16 lands on 105,8; then L 4, U+L 8, L 8 lands on
  the ledge at 100,6 (holds in frames). Walking left on the ledge, something hurts the
  frog (a dart, marker 18 at 108,5, likely): time the last steps to the switch.
- Plan with `--gap 6` (the floor-to-block jump crosses 8 cells of spikes).
- `/tmp/t104/W6-2-pre33.txt`: T104's script to the ledge (20 692 frames, much of it wasted
  bouncing; replanning from ~10 900 with the hops above would be far shorter).

## What T104 left

- `examples/plan_exit` (T104): the coin phase (`--coins N`; switches pressed as targets, a switch whose number no gate has is skipped), a gravity field over the live map (jump 7 rows, a pad 24, marker 62 a jump-through floor, 44 a lift, a cell over spikes enterable within `--gap N` cells of safe ground; a full-speed run jump crosses ~17 cells), passed-over coins coming back after 12 chunks, a search on the original when every engine path dies there, switches before a walled exit. Debug: `PLAN_WALLS=c0,r0,c1,r1` (walls crossed per cell), `PLAN_DUMP`, `PLAN_EXIT_ONLY`; `PLAN_TRACE` follows the field downhill ignoring move rules (misleading). Budget: `--frames` counts the prefix (default 12 000).
- The planner stalls where a route needs a run-up or a precise hop: find the hop by brute force on the original (left/right/up+right holds of a few lengths from a saved state, success = a cell box reached on the ground) and append it to the script, then resume with `--prefix`.
- An exit is reached when `level_state` (h0+$a06, long) is 1 for two frames; the planner can miss it between chunks, so check a finished script on the original and cut it one frame after.
- T104's scratch files under `/tmp/t104/` may still exist on this machine (not guaranteed): named below.

## Tests

- [ ] `enemies-w6-2-complete` green in `sf-conform -- all` to the exit, or its first divergence named in the case and T146.

## Done when

- [ ] The case exists, reaches the exit on the original, and is in the test block if green.
