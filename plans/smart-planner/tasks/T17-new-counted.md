# T17 — new-counted

**Phase:** 3 · **Depends on:** T16 · **Weight:** heavy (machine time, up to ~25 h)

## Goal

The counted run of the new planner on sets A and B under exactly T16's conditions, so the two sit side
by side in one table.

## Design sections this implements

DESIGN §1 (M1–M6), §4.2, D2.

## Files

- `plans/smart-planner/bench/results.jsonl` (new lines), `plans/smart-planner/bench/RESULTS.md`
- `plans/smart-planner/bench/routes/original/{A,B}/W*-*.txt` (the routes, committed as text)

## Run

`sf-bench --set A --planner new --jobs 1`, then `--set B`, guided by each level's committed route
(DESIGN §4.2, D21), with T16's quiet-machine check per slice.
Set B also records route repair as an extra, ungated column: `--planner new --follow-standalone`.

## Done when

- [ ] 24 set-A and 24 set-B lines committed with routes; RESULTS.md shows both planners side by side.
- [ ] A clear-cut planner bug found here is fixed with a test and the affected levels re-run; anything
      else is left for T18 to bring to the owner.
