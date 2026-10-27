# T25 — tuning-set-a

**Split from:** T14 (its context limit, 2026-10-23). Carries T14's Done-when items: the four stall
levels within 30 min alone; every set-A level completes or has a FINDINGS row with cause brought
to the owner; route length in the tuning table with T22's defaults tuned; the plan-hints.log check;
no level-id branch and no hinted result.

**Phase:** 3 · **Depends on:** T14, T24; blocks T15 · **Weight:** heavy

## Goal

T14's Run steps 1 (set A) and its route-length tuning, on the planner as T14 and T24 left it, run
alone on a quiet machine so the 30-minute budgets mean what they say.

## Files

- `plans/smart-planner/bench/tuning/results.jsonl`, `plans/smart-planner/bench/tuning/RESULTS.md`
  (commit these two only; `routes/` beside them is not git-ignored and stays out of git).
- `engine/crates/sf-plan/src/**` for a tuning fix (T22's `SHORTEN_SHARE`, `SHORTEN_MIN_ROUNDS`),
  each with a test.
- `plans/smart-planner/FINDINGS.md`.

## Run

`cd engine && cargo build --release -p sf-bench` first (the bench rebuilds `sf-plan`, not itself),
then `SF_PLAN_LOG=1 ./target/release/sf-bench --set A --planner new --jobs 1 --results
../plans/smart-planner/bench/tuning/results.jsonl --table ../plans/smart-planner/bench/tuning/RESULTS.md`.
Background slices of at most 2 h. A rerun skips levels recorded for the same commit: do not commit
between slices, or resume with `--levels`. A level that fails while the machine is busy (load over
~10) is re-run alone before it is called a miss.

T14's numbers (FINDINGS, 2026-10-23) are the before: six of the first nine levels completed shorter
than their committed routes; failures were budget-bound under load, the near-exit stall (W1-4,
W2-3, W3-4), or the repair patterns T24 takes.

## Done when

- [ ] W1-4, W2-4, W6-2, W6-3 complete guided in set A within 30 min each, run alone.
- [ ] Every set-A level completes guided, or its failure is one FINDINGS row with cause and has been
      brought to the owner (D5); clear-cut planner bugs fixed, each with a test.
- [ ] Route length against the committed routes is in the tuning table, and T22's defaults were tuned
      until M4 holds or each level over 25% has a stated cause.
- [ ] `engine/conform/plan-hints.log` has no entry lines (only its header at T14's start): recorded
      in FINDINGS; any line since gets a row (fixed in the planner, or brought to the owner).
- [ ] No level-id branch in `sf-plan` (`grep` for level ids in `src/` finds only tests, facts, the
      `lag_limit_for` case-name rule and `parse_args`' placeholder), and no result used a hint.
