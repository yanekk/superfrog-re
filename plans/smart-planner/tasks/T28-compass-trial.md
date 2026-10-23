# T28 — compass-trial

**Split from:** T14 (its context limit, 2026-10-23). Carries T14's "the compass trial's numbers
are in one FINDINGS row with the choice made and why".

**Phase:** 3 · **Depends on:** T14, T27; blocks T15 · **Weight:** heavy

## Goal

T14's Run step 3 (DESIGN §2.2, §2.3): on W1-4, W2-3, W2-4, W6-2, W6-3, W6-4, unguided, run alone,
three seeds each, `sf-plan --compass straight` against `--compass map` (and coin choice as a pull,
if built as a compass option here). Keep a compass only if it completes more of these or plans
them in measurably fewer engine frames; otherwise the default stays straight (owner, 2026-10-22).

## Files

- `engine/crates/sf-plan/src/**` only if a compass is kept or coin choice is wired as one, with tests.
- `plans/smart-planner/FINDINGS.md`.

## Run

`--engine-only` is enough to compare planning (engine frames are machine-independent;
`PLAN_STATS` carries them and `rounds`); 30-min budget; maps from the cache (built once a level,
its time recorded apart). Run each pair alone, or both of a pair side by side so they share the
load.

## Done when

- [ ] The compass trial's numbers are in one FINDINGS row with the choice made and why.
