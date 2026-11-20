# T34 — retire-old-tools

**Phase:** 3 · **Depends on:** T18 · **Weight:** medium

Added by T18 with the owner's yes (2026-10-26): T18's deletion of the old tools, set aside.

## Goal

Delete today's level-routing tools so agents have one (M9), once the owner has settled the open
question T18 left: **does the owner accept the M1 miss (and M5, C1's), or first play the failing
stretches?** D5 deletes the old tools only once M1 holds or the owner accepts the miss. The owner's
standing rule is that a level the planner fails twice goes to them to play the stuck stretch
(recorded in the app, spliced into the level's guide).

The failing levels, from T17's counted run (`bench/verdict.md`):

- Coins met, exit not reached: set A W6-4 (48 cells); set B W2-3, W3-2, W3-3, W6-4 (10–48 cells);
  set C W6-4 (48 cells).
- Coins never met (a repair's reach missed, then the rest-of-level plan spent the budget): set A
  W2-3 (9 short), W3-4 (82 short); set B W4-4 (40 short), W6-1 (2 short); set C W3-4 (3 short).

## Design sections this implements

DESIGN §1 (M1, M5, C1, M9), D5, D12 (`complete` retired with the others), D1 (`plan_bonus` stays).

## Steps

1. **Ask the owner first** which way: accept the misses as recorded, or play the failing stretches.
   If they play: for each stretch, show the stall (map picture and cell, `W6-4:263,66` style), hand a
   ready app command that plays to just before it, turn the recording into a trail spliced into the
   guide, rerun that level with `sf-bench` (same set, seed 1, 30 min), and record the result. Which
   levels to play is the owner's choice.
2. Record the outcome in `bench/verdict.md` (M1, M5, C1 rows, M9 row) and regenerate `RESULTS.md`
   (`cd engine && cargo run --release -p sf-bench -- --table-only`).
3. **Delete** `engine/crates/sf-conform/examples/{plan_exit,plan_exit_w6,explore,complete}.rs` and
   the old adapters in `engine/crates/sf-bench/src/adapters.rs` (`OldTool`, `PLAN_EXIT`,
   `PLAN_EXIT_W6`, `EXPLORE`, `OLD`, `old_for`) with the `best-old` planner path in `run.rs`/`main.rs`
   and their tests. Their recorded results in `results.jsonl`, `sample/` and `tuning/` stay, and the
   table still renders the `best-old` column from them.
4. `README.md` and `CLAUDE.md` § What's known: one line, routes are planned with `sf-plan`; no doc
   still names the deleted tools except history (plans, FINDINGS, commit messages).

## Files

- `plans/smart-planner/bench/verdict.md`, `RESULTS.md`; `results.jsonl` for any rerun
- `engine/crates/sf-conform/examples/{plan_exit,plan_exit_w6,explore,complete}.rs` (deleted)
- `engine/crates/sf-bench/src/{adapters,run,main}.rs`, `engine/crates/sf-bench/tests/bench.rs`
- `README.md`, `CLAUDE.md`
- `plans/smart-planner/FINDINGS.md`

## Tests

- The test block is green after the deletion.
- `sf-bench --table-only` still renders the recorded `best-old` results (a test with a `best-old`
  fixture, no old adapter linked).
- `grep -rn "plan_exit\|plan_exit_w6\|examples/explore\|examples/complete" engine README.md CLAUDE.md`
  finds nothing outside history.

## Done when

- [ ] The owner's answer to the open question is a dated FINDINGS row; any replayed level's rerun is
      in `results.jsonl` and the verdict.
- [ ] The four old tools and their adapters are deleted; `plan_bonus` stays.
- [ ] M9's verdict row reads met: 1 tool (+ `plan_bonus`).
- [ ] README and CLAUDE.md name `sf-plan` as the one routing tool.
