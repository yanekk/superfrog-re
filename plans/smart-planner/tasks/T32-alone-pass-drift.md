# T32 — alone-pass-drift

**Phase:** 3 · **Depends on:** T31; blocks T17 · **Weight:** medium

Added by T31 with the owner's yes (2026-10-24).

## Goal

T31's set-B W2-2 and W5-1 routes reach the exit on the original and fail `sf-bench`'s §4.1
faithful check on check 4 only: the engine replaying the route alone (inputs, the original's
pacing and clock seeds) runs one pass ahead of the original from mid-level, then takes an item or
a coin the original does not. Find why the engine drifts a pass ahead.

Owner's order (2026-10-24): prefer fixing the engine (`sf-core`), with a conformance case that
fails without the fix, as T29 did. Never loosen the faithful check (no `exclude`, no lag bar
raised, no check skipped; DESIGN §1 Stance, §8). If the cause is not an engine defect, make the
planner reject such routes (its confirmation, §2.6), or bring the choice to the owner.

- W2-2, from `game-whole` frame 48528 (T31's lead): the alone engine first differs at frame 530,
  falling: `frog_y` $d2/$d8, `frog_speed_tick` 2/3, `frog_crouch_time` 4/3 (original/engine), the
  engine's previous values the original's current ones. Frame 1091: the engine's item taken
  (marker 38 at 67,63) unmatched.
- W5-1, from frame 170003: first differs at frame 917, `frog_x` $859/$85d, `frog_prev_x`
  $855/$859, `frog_speed_tick` 1/2. Frame 1190: the engine's coin (39 left for the exit) unmatched.

One pass exactly, so the pass count, not the frog's physics, is the first suspect: a frame ending
mid-pass the alone engine paces wrongly (rust-engine FINDINGS T184, T195 had this shape). A
suspect, not a finding.

## Files

- `engine/crates/sf-core/src/**` (an engine fix), or `engine/crates/sf-plan/src/**` (a planner
  rejection, only per the order above).
- `engine/conform/cases/`, `engine/conform/scripts/`: one case per route (or cut to the frames
  that matter), green faithful after the fix and red before it.
- `plans/smart-planner/bench/tuning/results.jsonl`, `RESULTS.md`: W2-2 and W5-1 rerun.
- `plans/smart-planner/FINDINGS.md`.

## Interface

No new command or flag.

## The routes

T31's copies: `~/.cache/superfrog-re/t31/new-B-W2-2.txt`, `new-B-W5-1.txt` (seed 1, commit
e8f3a3d). `sf_bench::verify::case(Set::B, start, route)` builds the case §4.1 runs;
`SF_CONFORM_ALONE_TRACE=<file>` lists the alone engine's differing variables per pass. A case
file: `sync = { script = "game-whole", frame = 48528, df1 = true }` (W5-1: 170003), the route
from its frame 1 (`InputScript::from_frame(1)`) as its script, `frames` its last line + 24,
`resync = true`, `game-whole`'s `lag-limit`.

## Tests

- A conformance case per route, red before the fix, green after (or, for a planner rejection, a
  planner test that the route is refused).
- `sf-conform all` stays green.

## Done when

- [ ] The cause is named in FINDINGS (what the engine did differently and why).
- [ ] Fixed in the engine with the cases above, or, if not an engine defect, the planner rejects
      such routes, or the owner chose and the answer is in FINDINGS.
- [ ] W2-2 and W5-1 rerun in set B guided (T26's command, `--levels W2-2,W5-1`, run alone); each
      completes, or its next difference is one FINDINGS row brought to the owner.
