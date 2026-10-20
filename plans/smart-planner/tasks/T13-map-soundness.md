# T13 — map-soundness

**Phase:** 3 · **Depends on:** T01, T08 · **Weight:** medium

## Goal

Check the map against reality (M7): a goal-free explorer runs for an hour per level on the engine, and
every standable cell it reaches that the map calls unreachable is a hole in the map. A hole found is
fixed in the move set (with a test) or brought to the owner.

## Design sections this implements

DESIGN §1 M7, §4.2 (engine only, parallel), §2.2.

## Files

- `engine/crates/sf-plan/src/core/explore.rs` (goal-free mode over T08's archive: novelty only)
- `engine/crates/sf-bench/src/soundness.rs`, `sf-bench --soundness [--levels] [--minutes 60] [--jobs N]`
- `plans/smart-planner/bench/soundness.md` (per level: cells reached, cells in map, holes listed by cell)

## Tests

- [ ] On a fixture map with one edge removed, a short explorer run reports the cell behind it as a hole.
- [ ] Explorer deterministic for a seed over a fixed step count.

## Done when

- [ ] `soundness.md` committed for all 24 levels at 60 min each (levels in parallel, `--jobs` ≤ cores).
- [ ] Every hole is fixed (move-set change with a test) and re-checked, or listed for the owner with why.
