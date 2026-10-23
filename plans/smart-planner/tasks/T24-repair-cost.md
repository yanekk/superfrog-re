# T24 — repair-cost

**Split from:** T14 (its context limit, 2026-10-23). Carries T14's "clear-cut planner bugs fixed,
each with a test" for the repair patterns below.

**Phase:** 3 · **Depends on:** T14; blocks T15 · **Weight:** heavy

## Goal

Original-mode repairs (DESIGN §2.6) cost most of the 30-minute budget on the levels T14 saw fail.
Find general fixes (no per-level option, DESIGN §1 Stance) for the two repair patterns in
FINDINGS (T14 rows, 2026-10-23), or bring the owner the choice where one is a trade:

1. **Heart partings at a new cell each time.** W3-2 and W4-2 part on `Heart` (the original loses
   a heart the engine copy does not) at a different cell after each repair, so §2.6 step 4's
   "three running at one cell" never accepts one, and each repair re-plans (~650 rounds). Accepting
   sooner trades M6 (hearts) for M1 (completion): that is the owner's decision, asked with numbers.
2. **An early parting on a long level re-plans it all.** W4-4 parted 1 863 frames into a 10 710-
   frame plan; its repair did not end in 24 minutes. W1-4's repair took 4 261 rounds. The old
   plan's rest, replayed from the original's state, dies after ~2 000 frames (W1-4), so the seed
   helps little. A splice back onto the old plan was tried and failed (FINDINGS); do not repeat it.

## Files

- `engine/crates/sf-plan/src/oracle.rs`, `src/run.rs`, `src/core/**` as a fix needs; each fix with
  a test that fails without it.
- `plans/smart-planner/FINDINGS.md`.

## How

Run single levels with `SF_PLAN_LOG=1` (prints each stretch: plan frames, rounds, agreed, parting),
alone, e.g. `cd engine && SF_PLAN_LOG=1 cargo run --release -p sf-plan -- W3-2 --guide
conform/scripts/enemies-w3-2-complete.txt --out /tmp/w3-2.txt`. Compare rounds and repairs before
and after on W3-2, W4-2, W4-4, W1-4 (guided by their committed routes, `verify::committed_script`).

## Done when

- [ ] Each pattern has a FINDINGS row: fixed in general (with the before/after numbers), or brought
      to the owner with the trade and their answer recorded.
- [ ] `cargo test -p sf-plan` tests for every fix pass; the M8 tests still pass.
