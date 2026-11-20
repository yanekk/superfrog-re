# T26 — tuning-set-b

**Split from:** T14 (its context limit, 2026-10-23). Carries T14's "every set-B level completes
guided, or its failure is one FINDINGS row with cause and has been brought to the owner".

**Phase:** 3 · **Depends on:** T14, T25; blocks T15 · **Weight:** heavy

## Goal

T14's Run step 1 for set B: the 24 legs of the whole game, each guided by its committed chain
route, on the planner as T25 left it.

## Files

- `plans/smart-planner/bench/tuning/results.jsonl`, `RESULTS.md` (as T25).
- `plans/smart-planner/FINDINGS.md`.

## Run

As T25, with `--set B`. The leg snapshots are made once in `~/.cache/superfrog-re/bench-legs/`
(the whole game played once, minutes; FINDINGS 2026-10-21). `--jobs 2` is allowed (timings are not
counted; a level near the budget is re-run alone); `--jobs` × `--max-mem-gb` stays under 16 GB.
T14 before (load 13–27): W1-1, W1-2, W1-3, W2-1 completed; W1-4, W2-2 (Heart partings at one
cell, since fixed in T14), W2-3 failed.

## Done when

- [ ] Every set-B level completes guided, or its failure is one FINDINGS row with cause and has been
      brought to the owner (D5).
