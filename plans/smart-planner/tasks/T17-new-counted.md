# T17 — new-counted

**Phase:** 3 · **Depends on:** T15 · **Weight:** heavy (machine time, ~15–20 h)

## Goal

The counted run of the new planner on sets A and B, one level at a time on a quiet machine, shown
beside the old tools' baseline, which since D27 is T02's six-level sample.

## Design sections this implements

DESIGN §1 (M1–M6), §4.2, D2, D27.

## Files

- `plans/smart-planner/bench/results.jsonl` (new lines), `plans/smart-planner/bench/RESULTS.md`
- `engine/crates/sf-bench/src/table.rs` (and `main.rs`/`run.rs` if a flag is the way): the baseline's source in the table
- `plans/smart-planner/bench/routes/original/{A,B}/W*-*.txt` (the routes, committed as text)

## Run

`sf-bench --set A --planner new --jobs 1`, then `--set B`, guided by each level's committed route
(DESIGN §4.2, D21). Before each slice, check nothing else is building or running an oracle
(`pgrep -fl 'cargo|sf-conform|sf-plan|sf-bench'` shows only this run) and record the check in the
slice's commit message (`LevelResult` has no `note` field). Background slices of at most 2 h; the run resumes per level. No set-B
`--unguided --follow-standalone` column (dropped, D27).

The baseline does not reach `bench/RESULTS.md` by itself: `table::render` reads one results file
(`--results`, default `bench/results.jsonl`) and the T02 lines live in `bench/sample/results.jsonl`.
The worker either copies the sample's `best-old` lines into `bench/results.jsonl` once or has
`sf-bench` read the sample file as the baseline; either way `table.rs`'s header, which today says
only "Baseline is `best-old` … unaided", names the source (T02 sample, six set-A levels, busy
machine, D27), with a `table.rs` test. Set B's baseline cells stay "—".

## Done when

- [ ] 24 set-A and 24 set-B lines committed with routes; RESULTS.md shows the new planner beside the
      baseline from T02's sample (`bench/sample/`), labelled as six set-A levels on a busy machine (D27);
      any slice run on a busy machine is re-run.
- [ ] A clear-cut planner bug found here is fixed with a test and the affected levels re-run; anything
      else is left for T18 to bring to the owner.
