# T33 — dead-end-repair

**Phase:** 3 · **Depends on:** T27, T30; blocks T17 · **Weight:** heavy

Added by T27 with the owner's yes (2026-10-24).

## Goal

Stop an original-mode repair from spending the rest of the budget in a dead end. In T27's unaided
logs (`bench/tuning/routes/new-A-W1-3.log`, `new-A-W4-1.log`) a reach stretch was confirmed in
full (W1-3 stretch 4: 1 559 frames to route 5 826; W4-1 stretch 3: 978 frames to route 3 001). The
next stretch, planned from the original's state at exactly the reach point, archived only 5 and 2
cells and spun ~25 min each until the 30-min budget ended: `plan_engine` has no round cap for a
whole plan.

Suspected cause: `Goal::Reach` (`core/explore.rs`) is met on the frame the frog enters the point's
region, with no survival check, while `Goal::Exit` needs `EXIT_TAIL` idle frames alive
(`Walk::survives`). So a reach can end in a doomed state (mid-fall over a pit, about to be hit).

## Files

- `engine/crates/sf-plan/src/core/explore.rs` (`Goal::Reach`, `Walk::survives`), each change with a
  test in `engine/crates/sf-plan/tests/explore.rs`.
- `engine/crates/sf-plan/src/oracle.rs` (the repair loop in `plan_original`, `Marks`, `kept`), with
  tests in `engine/crates/sf-plan/tests/oracle.rs`.
- `plans/smart-planner/DESIGN.md`: §2.6 amended, decision D26.
- `plans/smart-planner/bench/tuning/results.jsonl`, `RESULTS.md`: W1-3 and W4-1 unaided reruns.
- `plans/smart-planner/FINDINGS.md`.

## Steps

1. **Reproduce first.** Rerun W4-1 unaided, original mode, seed 1, `SF_PLAN_LOG=1` (the dead end
   began at ~317 s). Confirm or refute the doomed-reach cause: replay from the stuck stretch's
   start, check whether every joystick hold dies within ~40 frames, and record what kills it. Write
   the answer in FINDINGS before fixing. If the cause is different, stop and tell the owner before
   building step 2's first part.
2. **Fix, both parts.**
   - If confirmed: `Goal::Reach` also requires the frog to survive `EXIT_TAIL` idle frames after
     reaching the point, as `Goal::Exit` does.
   - Dead-end detection, whatever the cause: a stretch whose archive holds at most `DEAD_CELLS`
     cells (start at 8) after `DEAD_ROUNDS` rounds (start at 100) is a dead end, and its time is
     not spent further. The repair rewinds the route to the previous stretch start (`Marks` already
     holds the original's snapshot and hearts there; reuse `oracle::kept` / T30's lag-repair
     rewind) and re-plans that stretch with the next seed. A second dead end from the same mark
     rewinds one mark further. Counted in rounds, never time, so M8 holds. D16 still holds: this
     backs up, it never gives up on the level.
   - Amend DESIGN §2.6 and add D26 with this rule and its reason.

## Interface

`DEAD_CELLS` and `DEAD_ROUNDS` are `pub const`s in `oracle.rs` (or `core`), beside
`REACH_ROUNDS`. The dead-end check reads the archive's cell count (what the no-route line reports
as `cells archived`) in the shell's round loop after each round, with no clock; `Progress` does not
carry it today. The bench command is T25's, unaided:
`cd engine && cargo build --release -p sf-bench && SF_PLAN_LOG=1 ./target/release/sf-bench --set A
--planner new --unguided --jobs 1 --levels W1-3,W4-1 --results
../plans/smart-planner/bench/tuning/results.jsonl --table
../plans/smart-planner/bench/tuning/RESULTS.md`, run alone (load under ~10).

## Tests

- A unit test that a dead-end stretch rewinds to the previous mark, and a second dead end from the
  same mark rewinds one further.
- A test that a reach into a doomed state is not a goal (a hand-made case, or the W4-1 state if
  cheap).
- Same route at 1 and 4 threads (M8) with the dead-end rule on.

## Done when

- [ ] FINDINGS has the reproduced cause of the W4-1 dead end, written before the fix.
- [ ] `Goal::Reach` requires survival (if the cause was confirmed), and the dead-end rewind is in,
      each with its test; same route at 1 and 4 threads.
- [ ] DESIGN §2.6 amended and D26 added; the PR notes that T24's reach stretch and T30's lag rewind
      behave as before when no dead end occurs.
- [ ] W1-3 and W4-1 unaided, original mode, through `sf-bench --jobs 1` into the tuning results, and
      neither spends its budget in a dead end (completing is hoped for, not required); times and
      outcome in FINDINGS.
