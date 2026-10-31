# T07 — smoke-fits

**Phase:** 2 · **Depends on:** T02, T05, T06 · **Weight:** heavy

## Goal

Run the real smoke set, including from a cold compile, and make it fit 15 minutes. This is the only place before
the end gate where the 15-minute promise is tested.

## Design sections this implements

DESIGN §2.4, §2.6, §4.

## Files

- `toolkit/smoke.toml`, `games/*/smoke.toml`: moving fast files to `full` if needed
- `tools/smoke.py`: only if §3.2's feature-rebuild concern is real
- `tools/test_smoke_labels.py` (new): the check passes on this repo

## Interface

The order of moves when over budget, from DESIGN §2.4: move the slowest fast files to `full`, one at a time, with
each move in the commit message. Never move a test covering an area. If the areas alone do not fit, stop and ask
the owner what to drop or swap, with the measured time of each area.

Measure twice: once warm, and once from a fresh worktree under the main checkout's `.claude/worktrees/` with its
own empty cargo target, because pir's gate may compile from nothing. The worktree is removed afterwards, pass or
fail. If other sessions were running tests at the time, say so beside the number.

## Tests

- [ ] `tools/test_smoke_labels.py`: the check on the real repo gives no problems
- [ ] a real `tools/smoke.sh` run under `pir-test`, green
- [ ] a real cold run under `pir-test`, green

## Done when

- [ ] Both measured totals are in FINDINGS, and the cold one is 15 minutes or less.
- [ ] Any test that failed in a real run is fixed if the cause is the label or runner; otherwise raised, not relabelled away.
- [ ] Neither `tools/full.sh` nor `PIR_TEST_SCOPE=all tools/test.sh` was run.
