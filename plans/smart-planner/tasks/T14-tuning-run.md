# T14 — tuning-run

**Phase:** 3 · **Depends on:** T11 · **Weight:** heavy

## Goal

Uncounted runs of the new planner on sets A and B to find and fix planner gaps before the counted runs,
until the four stall levels pass and every level either completes within budget or has a stated cause.
This is where the planner meets all 24 levels for the first time; the fixes are general (move set,
search, coin choice), never per-level options (DESIGN §1 Stance).

## Design sections this implements

DESIGN §1 (M1, M2, M5, M6), §4.2, D5.

## Files

- Any `engine/crates/sf-plan/src/**` file a fix needs, each fix with a test that fails without it.
- `plans/smart-planner/bench/tuning/results.jsonl`, `plans/smart-planner/bench/tuning/RESULTS.md`
- `plans/smart-planner/FINDINGS.md`

## Run

`sf-bench --set A --planner new --jobs 3 --results plans/smart-planner/bench/tuning/results.jsonl`,
then set B the same; parallel jobs are allowed here because timings are not counted (a level near the
budget is re-run alone before it is called a miss). Background slices of at most 2 h; the run resumes.

## Done when

- [ ] W1-4, W2-4, W6-2, W6-3 complete in set A within 30 min each, run alone.
- [ ] Every set-A and set-B level completes, or its failure is one FINDINGS row with cause and has been
      brought to the owner (D5); clear-cut planner bugs fixed, each with a test.
- [ ] Every line in `engine/conform/plan-hints.log` at the start of this task has a FINDINGS row:
      fixed in the planner (the level now plans unaided) or brought to the owner.
- [ ] No level-id branch exists in `sf-plan` (`grep` for level ids in `src/` finds only tests and
      facts), and no set-A or set-B result used a hint.
