# T168 — w2-4-completion

> **Amended** by the owner, 2026-10-15. "Green", "matches" and "frame-exact" below mean DESIGN §2.1's
> faithful checks (T180's default mode); a completion case is also judged alone (check 4, T182).
> Re-sync pins, `resync = false` and `--strict` are not required. Dependencies on dropped tasks
> are gone; PLAN.md's table is the current list. Where this doc and DESIGN §2.1 disagree, §2.1 wins.

**Phase:** 2 · **Depends on:** T122; blocks T18, T155 · **Weight:** heavy

Split from T122 (owner, 2026-10-15). T122 played W2-1 to its exit; W2-4 needs its route found.

## Goal

1. `w2-4-complete`: a scripted run from W2-4's start (5,73) to its exit (216-217,74-76) through the
   tally, every coin the exit needs collected (99 at the start), checked on the original chunk by
   chunk, committed outside the test block like `w2-1-complete` (T155 makes it green).
2. Carried from T43 through T122: the types no W2 case meets yet, on this route or a case of its
   own: the swooper (template 37, marker 21 at 21,52, update h0+$257e4), an enemy stomped into a
   reward after a spud stun, a heart taken. Check them with `examples/script_seen`, then through
   `run_with` once green (T155).
3. **Confirm and record the original's hang at the gate 18,55** (a possible DESIGN §2.11 case, as
   T105 did for W2-1): with the gate closed (switch 46,66-67 not pulled), T122's planner pushed the
   frog into the wall at 20,56 (`plan_exit` prints frog 326,896) from the chimney at 21-23; from frame 1176 the
   original's clock and frog never changed again. Find the loop (likely `frog_tiles`' wall loop,
   h0+$e3fe), whether the engine reproduces it, and list it for the owner in DESIGN §2.11.

## What T122 found

- A jump reaches 3 rows (`examples/try_inputs`): the start area cannot be left by jumping. The way
  out: right along row 68, pull the switch at 46,66-67 (planner `--jump 4 --via 45,68` from T122's
  frame 216 did it by 360), back through the opened gate 18,55 and up the chimney 21-28,55-68 to the
  room 1-21,21-44 (frame ~744-1224).
- From that room S1 (2-3,12-20, secret passage) is a one-way drop from the top-left room: the pads
  at 2-3,44 and 8-9,44 throw the frog only to row 22. The way on from the room is open; wings at
  59,73 (flight) may be it. The top row's coins (3-5, from 5,4 to 197,4) are most of the 99.

## Tools

As T166's: `examples/plan_exit` (`--no-tile-coins --jump 4`, `--via`, `--reach-only`),
`examples/try_inputs`, `examples/script_seen`, `tools/export_annotated.py`.

## Files

`engine/conform/{cases,scripts}/w2-4-complete.*`; `plans/rust-engine/DESIGN.md` §2.11 (the hang,
listed for the owner).

## Done when

- [ ] `w2-4-complete` reaches `level_outcome` `Completed` on the original's route, committed with
      its red frames (if any) recorded in FINDINGS for T155.
- [ ] Goal 2's types met (or each recorded as unreachable, with why).
- [ ] The gate hang confirmed or refuted, and recorded.
