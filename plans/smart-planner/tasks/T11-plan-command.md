# T11 — plan-command

**Phase:** 2 · **Depends on:** T05, T10 · **Weight:** medium

## Goal

The one command an agent runs: `sf-plan`, which wires facts, the cached map, danger, coin choice,
search, repair and oracle confirmation into a script on disk, and the adapter that lets `sf-bench` run
it. After this, routing a level is one command with no level-specific options.

## Design sections this implements

DESIGN §2.1, §2.8, §2.9, §2.10, §5.2, D8, D11.

## Files

- `engine/crates/sf-plan/Cargo.toml` (`[[bin]] sf-plan`)
- `engine/crates/sf-plan/src/main.rs`
- `engine/crates/sf-bench/src/adapters.rs` (adapter `new`)
- `engine/crates/sf-plan/tests/cli.rs`
- `engine/conform/plan-hints.log` (created with a header line saying what it is, DESIGN §2.10)
- `README.md` (a "Planning a route" section: what `sf-plan` does, its one command, link to this plan)

## Interface

```
sf-plan <level> [--snap leg.snap] [--follow old.txt] [--seed N] [--out route.txt]
        [--budget-min 30] [--frames 30000] [--threads N] [--no-cache] [--engine-only]
        [--via c,r]... [--avoid c0,r0,c1,r1]...          # hints, DESIGN §2.10; logged
exit 0: route written, reaches the exit
exit 1: budget or frames exhausted; best partial route written; one-line reason on stderr
exit 2: unplannable (count cannot be met, start invalid)
stderr last line: PLAN_STATS engine_frames=… oracle_frames=… confirm_share=… route_frames=… hearts_lost=… hints=N
```

`--engine-only` plans from the `AtLevel` state transplanted, with no confirmation (for tuning and M7);
`--app` is T12's.

## Tests

- [ ] `sf-plan W1-1 --engine-only --seed 1` twice → identical files (M8, test block).
- [ ] A run with `--via` appends one line to the hint log (test points the log at a temp file via
      `SF_PLAN_HINTS_LOG`) and prints `hints=1`; a run without hints writes nothing to it.
- [ ] Bad level id → exit 2 with a usage line; `--frames 100` → exit 1 with a partial route.
- [ ] `sf-bench --planner new --set A --levels W1-1` records a completed result (may be `#[ignore]`).

## Done when

- [ ] Tests green; README section added, saying hints exist, are logged, and are a last resort.
- [ ] `sf-plan W1-1` (original mode) writes a route that `sf-bench` verifies as completed.
