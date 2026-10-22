# T18 — results-and-retire

**Phase:** 3 · **Depends on:** T17 · **Weight:** medium

## Goal

Close the plan on numbers: the verdict for every target from the committed table, each miss brought to
the owner with its cause, and, once M1 holds or the owner accepts the miss, today's level-routing tools
deleted so agents have one tool (M9).

## Design sections this implements

DESIGN §1 (all targets), D5, D1 (`plan_bonus` stays).

## Files

- `plans/smart-planner/bench/RESULTS.md` (verdict section: each M and C target, met or missed, why)
- Deleted after the owner's go or M1 holding: `engine/crates/sf-conform/examples/{plan_exit,plan_exit_w6,explore,complete}.rs`
  (`complete` has no adapter and no baseline, D12)
- `engine/crates/sf-bench/src/adapters.rs` (old adapters removed with them; their recorded results stay)
- `README.md`, `CLAUDE.md` § What's known (one line: routes are planned with `sf-plan`)
- `plans/smart-planner/FINDINGS.md`

## Needs a person

The owner decides each missed target: accept it with the explanation written in RESULTS.md, or approve
a task to fix it (D5). The worker asks with the table and each miss's cause in front of them.

## Done when

- [ ] RESULTS.md verdict section lists M1–M6, M8, M9 and C1, C3, C4 with the measured value and
      met/missed, says the counted new-planner results were guided and the baseline unaided (D21),
      and shows T14's unaided set-A results beside them as recorded, not gated.
- [ ] Every miss has the owner's recorded decision (dated FINDINGS row).
- [ ] Old tools deleted only if M1 holds or the owner accepted; the test block is green after deletion
      (no test or doc still names them except history).
