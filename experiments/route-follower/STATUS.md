# Status — PAUSED by the owner (2026-10-24)

The main batch was stopped mid-way at the owner's request. Nothing is lost: every finished run is a row
in `results/runs.csv`, and `tools/make_batch.py` skips runs that already have a row.

## Done

- Prototype: `engine/crates/sf-plan/examples/follow/` (machines, reference run, executors fixed / A /
  Apure / B1 / B2 / B3 / B23). Wrapper `f.sh`; `FOLLOW_TARGET=target-dev` runs a second build.
- References (each committed route played once on the original, every frame recorded) for W1-1, W1-4,
  W2-1, W3-2, W5-2, W6-1, W6-2, W4-3 under `~/.cache/superfrog-re/route-follower/` (regenerate:
  `./f.sh many "$PWD/batches/refs.txt"`, ~2 min of oracle).
- Main batch, exit-fixed program (commit "exit = level_state below $64"): fixed 120/120 runs,
  A 116/120, B1 105/120. B2, B23, B3 and the determinism repeats not started.
- `results/runs-v1-exitbug.csv`: an earlier pass with the exit bug (missed exits on W1-4, W2-1, W3-2);
  kept for the record, not used.

## Signal so far (W1-1 hand runs, before the batch; to be confirmed by it)

- Fixed and A: break under every perturbation (table in `results/SUMMARY.md`); A replays its own
  timing exactly.
- B1: reaches no exit.
- B23 on W1-1, engine no lag: exit at 4 947 (route 4 474, +11%), no life or heart lost, ~256 engine
  frames per frame played, 55 s. B2: exit 5 525 (+23%). B3 alone: stalls when a coin is missed.

## To resume

```
cd experiments/route-follower
python3 tools/make_batch.py          # what is left
./f.sh many "$PWD/batches/main.txt" > results/main.log 2>&1   # hours; one oracle at a time
python3 tools/summarize.py           # results/SUMMARY.md
```

Then: question 5's comparison (`sf-plan --app --follow <committed route> --seed 1 --threads 3
--budget-min 10` on W1-1, W2-1, W5-2, time to a working route), and `REPORT.md`.
