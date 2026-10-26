# T27 — tuning-unaided

**Split from:** T14 (its context limit, 2026-10-23). Carries T14's "the unaided set-A results are
in the tuning table, each failure with a one-line cause".

**Phase:** 3 · **Depends on:** T14, T26; blocks T15 · **Weight:** heavy

## Goal

T14's Run step 2: set A with `--unguided`, into the same tuning results file (the `guided` field
tells them apart). Recorded, not gated (DESIGN §1): what the planner does on a level with no route.

## Files

- `plans/smart-planner/bench/tuning/results.jsonl`, `RESULTS.md` (as T25).
- `plans/smart-planner/FINDINGS.md`.

## Run

As T25, with `--unguided`; `--jobs 2` allowed. A failed run's reason carries `sf-plan`'s own
no-route line (T14), which is the one-line cause; add a FINDINGS row where it needs more.

## Done when

- [ ] The unaided set-A results are in the tuning table, each failure with a one-line cause.
