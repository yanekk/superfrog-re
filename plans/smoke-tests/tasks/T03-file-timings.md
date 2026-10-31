# T03 — file-timings

**Phase:** 1 · **Depends on:** T01 · **Weight:** light

## Goal

Find which test files are fast without running the full set. Each test file and each crate's inline tests run
alone, stopped at 11 seconds, because all the labelling needs to know is whether a file finishes in 10. The
result is what T05 and T06 label from.

## Design sections this implements

DESIGN §2.4, §4 (sanctioned runs), §5.2.

## Files

- `tools/smoke_timings.py` (new; kept, so a later plan can re-measure)
- `plans/smoke-tests/timings.tsv` (new, the result)

## Interface

```
venv/bin/python tools/smoke_timings.py [--cap 11] > plans/smoke-tests/timings.tsv
columns: project, label key (as in smoke.toml), seconds or "over", exit status
```

It compiles each workspace first with `cargo test --workspace --no-run`, so compile time is not charged to a
file, then runs entries one at a time and kills each process group at the cap. A file that exits non-zero
within the cap is recorded with its status, and is reported in FINDINGS, since a red test is not this task's to
fix. `#[ignore]`d tests stay skipped. Python files are not timed: all of them together take about 23 s (monorepo
FINDINGS) and all go in smoke.

## Tests

- [ ] the cap kills a slow fake entry and its child, and records `over`
- [ ] a fast fake records its seconds; a failing fake records its status
- [ ] the row's key matches the form `smoke_core` expects

## Done when

- [ ] `timings.tsv` has a row for every test file and every crate with inline tests, checked by count against `git ls-files`.
- [ ] The pass ran under `pir-test`, took at most about 30 minutes after compiling, and no file ran beyond the cap.
