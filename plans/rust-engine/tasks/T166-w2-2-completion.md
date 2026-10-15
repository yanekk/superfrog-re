# T166 — w2-2-completion

**Phase:** 2 · **Depends on:** T122; blocks T18, T155 · **Weight:** heavy

Split from T122 (owner, 2026-10-15). T122 played W2-1 to its exit; W2-2 needs its route found.

## Goal

`w2-2-complete`: a scripted run from W2-2's start to its exit (85,74) through the tally, every coin
the exit needs collected, checked on the original chunk by chunk, committed outside the test block
like `w2-1-complete` (T155 makes it green). Record what it meets (`examples/script_seen`) in its case
file and in T155.

## What T122 found

- Tile coins (marker 55) count in `frog+$7e`, not toward the exit: plan with `--no-tile-coins`.
  95 coins needed at the start.
- At frame ~408 the planner drops down the secret shaft at 38-39,12-47 into the lower-left; from
  there the right side's coins are out of reach. Take the top and the right side first (from
  T122's frame-360 state the whole top and right side is reachable: `--reach-only` map).
- Columns ~145-200, rows 41-77 are a one-way pocket (`v` on the reach map): the planner falls in for
  its coins (147,54, 156-164,68-73) and ends at 178,67 under the gate 178,64 (switch 49 at 231,43).
  `--skip 140,40,200,77` keeps it out.
- The column of 34 coins at 16,9-75 is reached from the top room (15-23,5-8) by falling down it, not
  by climbing; the shaft at 21-23 has pads at rows 27, 49, 65 in separate segments.
- T122's last attempt (`--jump 4 --skip 0,12,100,83 --skip 140,40,200,77 --via 150,5 --coins 30
  --no-exit` from frame 360) reached 150,5 at 1608 and 43 needed at 2952, but in the pocket again.

## Tools

`examples/plan_exit` (with T122's `--no-tile-coins`, `--jump 4`, `--skip`, `--reach-only`),
`examples/try_inputs` (test a jump or pad on the engine from a script's end), `examples/script_seen`,
`tools/export_annotated.py` (needs `build/`: symlink the disk zips into the worktree root, then
`extract_disks.py`, `unpack.py`).

## Files

`engine/conform/{cases,scripts}/w2-2-complete.*`; `plans/rust-engine/tasks/T155-*.md` only through
FINDINGS (add the red frames there).

## Done when

- [ ] `w2-2-complete` reaches `level_outcome` `Completed` on the original's route, committed with
      its red frames (if any) recorded in FINDINGS for T155.
