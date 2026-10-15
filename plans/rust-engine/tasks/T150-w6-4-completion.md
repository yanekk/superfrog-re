# T150 — w6-4-completion

**Split from:** T104 · **Phase:** 2 · **Depends on:** T104; blocks T18, T146 · **Weight:** heavy

Carries T104's done-when for W6-4 (first map).

## Goal

`engine/conform/cases/enemies-w6-4-complete.toml` to W6-4's exit (all 189 coins), nothing left
out, `resync = false`. It leaves the original at frame 154 (T146's `level_copper` fault), so
it stays outside the test block naming T146.

## Files

- `engine/conform/scripts/enemies-w6-4-complete.txt`, `engine/conform/cases/enemies-w6-4-complete.toml`
- `engine/crates/sf-conform/tests/enemies_w6_cases.rs`, if the case is green.

## Interface

None new: a case in the existing format (`sync`, `script`, `frames`, `resync = false`), its
script made with `examples/plan_exit_w6`.

## What is known (T104)

- T104's script took every coin by frame 9 690 (frog at 67,33); `/tmp/t104/W6-4-pre31.txt` is
  that script (may exist). Switches 48 and 49 are then pressed; 50 (pair 166,16-17) opens the gate
  at 157,3 on the top corridor (rows 2-9), which the exit route needs. Switch 51 (130,3-4) has no
  gate.
- The exit route: the top corridor east, a running jump over 24 cells of spikes (238-261, row 10)
  from the platform at 234-237 (`--gap 13`), down the shaft at 287-294 (entered at rows 9-10)
  through its zig-zag platforms to the exit at 296-299,73-78.
- Pitfalls: west of 67 the frog fell into an open pit (13-40, rows 19-40); its way out is the pad
  at 16,42 (24 rows). East, the room 165-196 rows 24-32 holds a thrower that killed every path at
  196,28. Planning from 9 690 to switch 50 with `--via 166,17` was going west when T104 stopped.
- `/tmp/t104/worktree-leftovers/` (may exist): T104's uncommitted `enemies-w6-4-complete` case and
  script, its `plan_exit_w6` `PLAN_PATH` patch and its `zz_*` probe examples, saved by T104's review.

## What T104 left

- `examples/plan_exit_w6` (T104): the coin phase (`--coins N`; switches pressed as targets, a switch whose number no gate has is skipped), a gravity field over the live map (jump 7 rows, a pad 24, marker 62 a jump-through floor, 44 a lift, a cell over spikes enterable within `--gap N` cells of safe ground; a full-speed run jump crosses ~17 cells), passed-over coins coming back after 12 chunks, a search on the original when every engine path dies there, switches before a walled exit. Debug: `PLAN_WALLS=c0,r0,c1,r1` (walls crossed per cell), `PLAN_DUMP`, `PLAN_EXIT_ONLY`; `PLAN_TRACE` follows the field downhill ignoring move rules (misleading). Budget: `--frames` counts the prefix (default 12 000).
- The planner stalls where a route needs a run-up or a precise hop: find the hop by brute force on the original (left/right/up+right holds of a few lengths from a saved state, success = a cell box reached on the ground) and append it to the script, then resume with `--prefix`.
- An exit is reached when `level_state` (h0+$a06, long) is 1 for two frames; the planner can miss it between chunks, so check a finished script on the original and cut it one frame after.
- T104's scratch files under `/tmp/t104/` may still exist on this machine (not guaranteed): named below.

## Tests

- [ ] `enemies-w6-4-complete` reaches the exit on the original; its divergence named (T146).

## Done when

- [ ] The case exists and reaches the exit on the original.
