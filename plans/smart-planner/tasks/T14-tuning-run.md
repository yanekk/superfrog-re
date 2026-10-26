# T14 — tuning-run

**Phase:** 3 · **Depends on:** T11 · **Weight:** heavy

Amended 2026-10-22 (D19–D21, D23): tunes the explorer, tries the compasses, records unaided results.

## Goal

Uncounted runs of the new planner on sets A and B to find and fix planner gaps before the counted runs,
until the four stall levels pass guided and every level either completes within budget or has a stated
cause. This is where the explorer meets all 24 levels for the first time; the fixes are general
(choice weights, burst shape, memory policy, shortening defaults), never per-level options (DESIGN §1
Stance). It also decides the compass (DESIGN §2.2, §2.3) and records what the planner does on a level
with no route.

## Design sections this implements

DESIGN §1 (M1, M2, M4, M5, M6, the unaided record), §2.2, §2.3, §2.4, §2.12, §4.2, D5, D20, D21.

## Files

- Any `engine/crates/sf-plan/src/**` file a fix needs, each fix with a test that fails without it.
- `plans/smart-planner/bench/tuning/results.jsonl`, `plans/smart-planner/bench/tuning/RESULTS.md`
- `plans/smart-planner/FINDINGS.md`

## Run

1. Guided: `sf-bench --set A --planner new --jobs 2 --results plans/smart-planner/bench/tuning/results.jsonl`,
   then set B the same. Parallel jobs are allowed because timings are not counted (a level near the
   budget is re-run alone before it is called a miss); `--jobs` × `--max-mem-gb` stays under 16 GB.
2. Unaided: set A again with `--unguided`, into the same results file (the `guided` field tells them
   apart). Recorded, not gated (DESIGN §1).
3. Compass trial: on W1-4, W2-3, W2-4, W6-2, W6-3, W6-4, unguided, run alone, three seeds each, with
   `--compass straight` and `--compass map` (and coin choice as a pull, if built as a compass option
   here). Keep a compass only if it completes more of these or plans them in measurably fewer engine
   frames; otherwise the default stays straight (owner, 2026-10-22).

Background slices of at most 2 h; the run resumes.

## Done when

- [ ] W1-4, W2-4, W6-2, W6-3 complete guided in set A within 30 min each, run alone.
- [ ] Every set-A and set-B level completes guided, or its failure is one FINDINGS row with cause and
      has been brought to the owner (D5); clear-cut planner bugs fixed, each with a test.
- [ ] The unaided set-A results are in the tuning table, each failure with a one-line cause.
- [ ] The compass trial's numbers are in one FINDINGS row with the choice made and why.
- [ ] Route length against the committed routes is in the tuning table, and T22's defaults were tuned
      until M4 holds or each level over 25% has a stated cause.
- [ ] Every line in `engine/conform/plan-hints.log` at the start of this task has a FINDINGS row:
      fixed in the planner (the level now plans without it) or brought to the owner.
- [ ] No level-id branch exists in `sf-plan` (`grep` for level ids in `src/` finds only tests and
      facts), and no set-A or set-B result used a hint.
