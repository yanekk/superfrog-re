# T15 — set-c-run

**Phase:** 3 · **Depends on:** T12, T14 · **Weight:** medium

## Goal

The counted set-C run: all 24 levels started as Game ▸ Start Level… starts them, planned and confirmed
on the engine alone, giving the app-timing routes recordings and drills need and the C1, C3, C4 numbers.

## Design sections this implements

DESIGN §1 (C1, C3, C4), §4.1 set C, §4.2 (parallel allowed), D3.

## Files

- `plans/smart-planner/bench/results.jsonl` (set C lines), `plans/smart-planner/bench/RESULTS.md`
- `plans/smart-planner/bench/routes/app/W*-*.txt` (the 24 app-timing routes, committed as text)

## Run

`sf-bench --set C --planner new --jobs 3`, guided (DESIGN §4.2; `--jobs` × `--max-mem-gb` under 16 GB). Parallel is allowed (no oracle in the planning; each guide is traced once on the original), but no other build or test
runs meanwhile, because C3 is a wall-clock target. If `plan_exit --engine` is on main, also
`--planner plan_exit` for a set-C baseline; otherwise the baseline column says why it is absent.

## Done when

- [ ] 24 set-C result lines and routes committed, each `# timing: app` and each verified in the real
      app as well as on the engine (DESIGN §4.1); RESULTS.md shows C1, C3, C4.
- [ ] Each miss is fixed if clear-cut (re-run that level) or brought to the owner (D5).
