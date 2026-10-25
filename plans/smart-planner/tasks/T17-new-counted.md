# T17 — new-counted

**Phase:** 3 · **Depends on:** T15 · **Weight:** heavy (machine time, ~15–20 h)

## Goal

The counted run of the new planner on sets A and B, one level at a time on a quiet machine, shown
beside the old tools' baseline, which since D27 is T02's six-level sample.

## Design sections this implements

DESIGN §1 (M1–M6), §4.2, D2, D27.

## Files

- `plans/smart-planner/bench/results.jsonl` (new lines), `plans/smart-planner/bench/RESULTS.md`
- `plans/smart-planner/bench/routes/original/{A,B}/W*-*.txt` (the routes, committed as text)

## Run

`sf-bench --set A --planner new --jobs 1`, then `--set B`, guided by each level's committed route
(DESIGN §4.2, D21). Before each slice, check nothing else is building or running an oracle
(`pgrep -fl 'cargo|sf-conform|sf-plan|sf-bench'` shows only this run) and record the check in the
results line's `note`. Background slices of at most 2 h; the run resumes per level. No set-B
`--unguided --follow-standalone` column (dropped, D27).

## Done when

- [ ] 24 set-A and 24 set-B lines committed with routes; RESULTS.md shows the new planner beside the
      baseline from T02's sample (`bench/sample/`), labelled as six set-A levels on a busy machine (D27);
      any slice run on a busy machine is re-run.
- [ ] A clear-cut planner bug found here is fixed with a test and the affected levels re-run; anything
      else is left for T18 to bring to the owner.
