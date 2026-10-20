# T154 — w1-4-completion

> **Amended** by the owner, 2026-10-15. "Green", "matches" and "frame-exact" below mean DESIGN §2.1's
> faithful checks (T180's default mode); a completion case is also judged alone (check 4, T182).
> Re-sync pins, `resync = false` and `--strict` are not required. Dependencies on dropped tasks
> are gone; PLAN.md's table is the current list. Where this doc and DESIGN §2.1 disagree, §2.1 wins.

**Phase:** 2 · **Depends on:** T106, T127; blocks T18 · **Weight:** heavy

Added by T106 (owner, 2026-10-15). T106 played W1-1 to W1-3 to their exits; W1-4's planner got a
green route to frame 4054 and no further. This task carries T106's done-when items for W1-4.

## Goal

`complete-w1-4`: W1-4 from its start to its exit (marker 54, 172,41-43) with every coin it needs
(`frog+$5c` from 96 to 0), the exit fade and the tally, fire 70 frames after the exit; green against
the oracle with every re-sync pinned in a test in `engine/crates/sf-conform/tests/frog_cases.rs`
(`completes(name, pins)`, as `complete_w1_3`).

## What T106 found

- `engine/conform/{cases,scripts}/complete-w1-4*` (committed, outside the test block): the route so
  far, 4054 frames, to 266,21 at the top right with 43 coins still needed. Green but for a re-sync
  at 2629 (a lag cut in `panel_update`, as W1-3's at 5079). Extend it with
  `plan_exit W1-4 --prefix engine/conform/scripts/complete-w1-4.txt ...`.
- Two faults met and dodged by shifting the input a few frames (re-plan from just before them
  after `k` idle frames): T131's `clr.w AUD3VOL` at a sample stream's end (first route, frame
  1531: stream 1510-1530 from the pad area round 49,67), and the frog's sprite on the bounce pad
  at 288,44 (frames 3898-3966 on one route, 6028-6076 on another; T131's list, likely T127's).
  The pad is the way up the 288-289 shaft to the top right; `plan_exit --avoid 283,36,292,46`
  drops paths that touch it.
- Every remaining coin the planner reaches on foot is up through that pad. The rest float in open
  sky (rows 1-21, columns 2-150: 6-29,6-17, the column 59-62,12-17, 136-138,13-19, 153-160,1-2):
  they need the wings (104,17, three rows above the floor at 104,20; others at 5,19, 127,34,
  158,47). `plan_exit`'s flood has no flight, so it stalls under them; five steered re-plans did.
- The top right reaches the upper left through the secret passage (marker 38, printed `2` by
  `map_ascii`) at 134-145,1-15. A cave of coins at 124-131,33 opens from above, down 118-119.
- `plan_exit` needs `0: -` / `20: -` as its prefix at a level's start.

## Files

`engine/conform/{cases,scripts}/complete-w1-4*`, `engine/crates/sf-conform/tests/frog_cases.rs`,
`engine/crates/sf-conform/examples/plan_exit.rs` (if flight planning is added).

## Done when

- [ ] `complete-w1-4` plays W1-4 to its exit and through the tally, green against the oracle,
      re-syncs pinned (T106's first done-when item, for W1-4).
- [ ] It is in the test block: a `frog_cases.rs` test asserts the frame after the tally is the
      next level's, the level completed.
