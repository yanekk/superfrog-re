# T16 — baseline-counted

**Phase:** 3 · **Depends on:** T15 · **Weight:** heavy (machine time, up to ~25 h)

## Goal

The counted baseline: today's tools, unaided, on sets A and B, one level at a time on a quiet machine,
so the new planner's numbers have a comparison. The baseline stays unaided while the new planner's
counted run is guided (owner, 2026-10-22, D21); the table says so. Runs at the end, while the old tools still exist
(D4).

## Design sections this implements

DESIGN §4.2, D2, D4, D10.

## Files

- `plans/smart-planner/bench/results.jsonl` (baseline lines), `plans/smart-planner/bench/RESULTS.md`

## Run

`sf-bench --set A --planner best-old --jobs 1`, then `--set B`. Before each slice, check nothing else
is building or running an oracle (`pgrep -fl 'cargo|sf-conform|sf-plan|sf-bench'` shows only this
run); record the check in the results line's `note`. Background slices of at most 2 h; the run
resumes per level.

## Done when

- [ ] 24 set-A and 24 set-B baseline lines committed, each with tool, completion, time, frames.
- [ ] RESULTS.md shows the baseline columns; any slice run on a busy machine is re-run.
