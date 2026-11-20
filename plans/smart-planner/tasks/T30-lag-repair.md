# T30 — lag-repair

**Phase:** 3 · **Depends on:** T25; blocks T15 · **Weight:** heavy

Added by T25's review with the owner's yes (2026-10-23).

## Goal

Find one rule for where an original-mode repair resumes after a `Lag` parting (the original
mid-pass past the level's lag bar, DESIGN §2.6) that finishes both W1-4 and W5-2 guided in set A,
each within 30 minutes, on one commit. No per-level option (DESIGN §1 Stance).

Today's rule (`oracle::kept`, every repaired parting) keeps the frames up to `BACK_OFF` (16) before
the parting: W1-4 done in 9.2 min; W5-2 fails its budget, kept 16 frames back from 954, inside an
~85-frame mid-pass run, and parted for lag there on every repair (~200 in 27 min).

Tried in T25 and its review, each finishing one of the two (logs in FINDINGS, T25 rows):

1. **Before the run** (ad0dd6f): a lag parting keeps the frames before the mid-pass run it ended,
   the stretch's start if the run began before it. W5-2 done in 4.3 min (kept 276 of 376 at its
   first lag parting, route 594 → 870). W1-4 fails: its first lag parting (stretch 5, agreed 194)
   kept 94 frames; from there every plan parted for lag 100 frames in, the run begun at the
   stretch's start, and the repair kept nothing and re-planned the same plan ~2 800 times.
2. **Before the run, else `BACK_OFF`** (8e28ded): as 1, but `BACK_OFF` frames when no frame before
   the run is in the stretch. W5-2 done in 4.4 min; W1-4 fails, now parting for lag 16 frames into
   each stretch (route 3 808) until the budget ends.
3. **Before the run only after a lag repair in a row** (T25 review, not committed): the
   first lag parting backs off `BACK_OFF`, the next in a row (no agreed stretch between) goes before
   the run. W1-4 done in 7.9 min; W5-2 fails: its first parting kept 360 frames into the run, the
   next parted 16 frames in with the run carried in, nothing before it in the stretch, 0 kept.

What the three show: W1-4's run at 148,9 cannot be ended from before it, only from inside it;
W5-2's run at 111,29 only from before it. A rule that looks past the current stretch (a kept
frame of an earlier stretch, which the route has already committed) or that tries both resume
points is a first suspect, not a finding.

## Files

- `engine/crates/sf-plan/src/oracle.rs` (`kept`, the repair loop in `plan_original`), each change
  with a test in `engine/crates/sf-plan/tests/oracle.rs` that fails without it.
- `plans/smart-planner/bench/tuning/results.jsonl`, `RESULTS.md`: W1-4 and W5-2 rerun, and any
  set-A level whose log (`bench/tuning/routes/new-A-*.log`) has a `Lag at` parting.
- `plans/smart-planner/FINDINGS.md`.

## Interface

`oracle::kept(ls, plan_len, accepted, start)` may take more arguments; its other callers are its
tests only. The bench command is T25's: `cd engine && cargo build --release -p sf-bench &&
SF_PLAN_LOG=1 ./target/release/sf-bench --set A --planner new --jobs 1 --levels W1-4,W5-2
--results ../plans/smart-planner/bench/tuning/results.jsonl --table
../plans/smart-planner/bench/tuning/RESULTS.md`, run alone (load under ~10). Seed 1 is
deterministic when no run hits its budget.

## Tests

- A unit test of `kept` per rule case, as T25's `a_lag_parting_keeps_the_frames_before_its_run`
  was (added in ad0dd6f, reverted with the rule in 985382b).
- The two bench runs above, recorded in `results.jsonl`.

## Done when

- [ ] W1-4 and W5-2 complete guided in set A on one commit, each within 30 min, run alone; the
      result lines committed.
- [ ] Every other set-A level with a `Lag at` parting in its T25 log is rerun on that commit and
      completes as before (in T25's logs only W1-4 and W5-2 have one).
- [ ] The rule is in `kept`'s doc comment with why, and each case has a test.
- [ ] Or, if no rule finishes both, the finding is one FINDINGS row brought to the owner.
