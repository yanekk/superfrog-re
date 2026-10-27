# T02 — baseline-sample

**Phase:** 0 · **Depends on:** T01 · **Weight:** medium

## Goal

Early numbers for today's tools, unaided, on the levels that matter most: enough to steer the design
(especially DESIGN §2.6's choice), not the counted baseline (that was T16, D4; since D27 T16 is dropped and this sample is the baseline). It also gives the first
measured answer to "how much of planning time is confirmation on the original".

## Design sections this implements

DESIGN §2.6, §4.2, D4, D10.

## Files

- `plans/smart-planner/bench/sample/results.jsonl`, `plans/smart-planner/bench/sample/RESULTS.md`
- `plans/smart-planner/FINDINGS.md`

## Run

`sf-bench --set A --planner best-old --levels W1-1,W1-4,W2-4,W3-1,W6-2,W6-3 --budget-min 30
--results plans/smart-planner/bench/sample/results.jsonl` — run in background slices of at most 2 h;
the run resumes per level. The machine need not be quiet; note what else was running.

## Done when

- [ ] Sample results and table committed; each level has a result line for each old tool.
- [ ] A FINDINGS row gives the median confirmation share and states the decision for T10: stretch
      confirmation if confirmation is ≥ 40 % of wall-clock on the sample's median, chunked otherwise,
      with the measured number.
- [ ] Any old-tool failure mode seen (stall, hang, crash) is one FINDINGS row each.
