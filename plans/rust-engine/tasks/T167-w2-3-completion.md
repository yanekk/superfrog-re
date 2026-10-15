# T167 — w2-3-completion

**Phase:** 2 · **Depends on:** T122; blocks T18, T155 · **Weight:** heavy

Split from T122 (owner, 2026-10-15). T122 played W2-1 to its exit; W2-3 needs its last leg found.

## Goal

`w2-3-complete`: a scripted run from W2-3's start (143,75) to its exit (124-125,8-10) through the
tally, every coin collected, checked on the original chunk by chunk, committed outside the test
block like `w2-1-complete` (T155 makes it green). Record what it meets (`examples/script_seen`).

## What T122 found

- With `--no-tile-coins --coins 0` from 120 idle frames the planner took every coin by frame 3384
  (frog at 190,14): the switch at 119,29 opening its gate 157,27 (frame ~3048), then up to
  178-198,14-23. The exit is then "out of reach" in the planner's flood.
- The exit room (100-127,3-10) is entered from the left: the top corridor (60-101,3-8) and the
  secret passage at 102-105,4-6. S9 (118-119,11-12) is a hole in the exit room's floor, maybe
  reachable from the block 111-112,13 (jump right from its top).
- The top corridor is above a tall room (60-78,3-41) of blocks and crumbling ledges (marker 36,
  rows 12-29), over a launcher shaft (launchers on 61 and 78, rows 42-58). A launcher throws the
  frog sideways at 20 px a frame with `vy` −5 (`level/collision.rs` `launcher`); T122's planner
  (`--launch 14`, vias up the shaft) never rose above row 52. How the room is climbed (or whether
  it is only descended, the corridor being reached another way) is open.
- The long slope 78-103,15-29 joins the room's right side at row ~28 to the area under the exit
  room (94-121,13-23).
- T122's run1 script reached 77,38 at frame 5448 with every coin taken (re-plan from there).

## Tools

As T166's: `examples/plan_exit` (`--ground 36`, `--launch`, `--jump 4`), `examples/try_inputs`,
`examples/script_seen`, `tools/export_annotated.py`.

## Files

`engine/conform/{cases,scripts}/w2-3-complete.*`.

## Done when

- [ ] `w2-3-complete` reaches `level_outcome` `Completed` on the original's route, committed with
      its red frames (if any) recorded in FINDINGS for T155.
