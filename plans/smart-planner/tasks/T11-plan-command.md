# T11 — plan-command

**Phase:** 2 · **Depends on:** T05, T10, T22 · **Weight:** medium

## Goal

The one command an agent runs: `sf-plan`, which wires facts, the explorer, guide paths, shortening,
repair, the optional map compass and oracle confirmation into a script on disk, and the adapter that lets `sf-bench` run
it. After this, routing a level is one command with no level-specific options.

## Design sections this implements

DESIGN §2.1, §2.8, §2.9, §2.10, §2.11, §2.12, §4.2 (guided runs), §5.2, D8, D11, D21.

## Files

- `engine/crates/sf-plan/Cargo.toml` (`[[bin]] sf-plan`)
- `engine/crates/sf-plan/src/main.rs`
- `engine/crates/sf-bench/src/adapters.rs` (adapter `new`: passes `--guide <committed route>`, DESIGN
  §4.2, unless `sf-bench --unguided`)
- `engine/crates/sf-bench/src/{main,results,table}.rs` (`--unguided`; a `guided` field in each results
  line and a column in the table)
- `engine/crates/sf-plan/tests/cli.rs`
- `engine/conform/plan-hints.log` (created with a header line saying what it is, DESIGN §2.10)
- `README.md` (a "Planning a route" section: what `sf-plan` does, its one command, link to this plan)

## Interface

```
sf-plan <level> [--snap leg.snap] [--guide route-or-waypoints.txt] [--follow old.txt] [--seed N]
        [--out route.txt] [--budget-min 30] [--frames 30000] [--max-mem-gb 4] [--threads N]
        [--compass straight|map] [--no-cache] [--engine-only]
        [--via c,r]... [--avoid c0,r0,c1,r1]...          # hints, DESIGN §2.10; logged
exit 0: route written, reaches the exit
exit 1: budget or frames exhausted; best partial route written; one-line reason on stderr
exit 2: bad arguments, invalid start, or a guide that does not parse or never leaves its start (DESIGN §2.9);
        never on evidence a level is impossible (an unmet count is exit 1, D16)
first line of the route file: # timing: original | app   (DESIGN §2.6, two timings)
stderr last line: PLAN_STATS engine_frames=… oracle_frames=… confirm_share=… route_frames=… hearts_lost=… hints=N
                 guided=0|1 repairs=N rounds=N cells=N peak_mem_mb=N   (DESIGN §2.4, §2.6, §2.9, §2.11)
```

`--engine-only` plans from the `AtLevel` state transplanted, with no confirmation (for tuning);
`--app` is T12's. `--compass map` builds or loads the T04 map (T05 cache); the default builds none.

## Tests

- [ ] `sf-plan W1-1 --engine-only --seed 1` twice → identical files (M8, test block).
- [ ] A run with `--via` appends one line to the hint log (test points the log at a temp file via
      `SF_PLAN_HINTS_LOG`) and prints `hints=1`; a run without hints writes nothing to it.
- [ ] Bad level id → exit 2 with a usage line; `--frames 100` → exit 1 with a partial route and a
      reason line giving coins needed and coins ever reached.
- [ ] `--guide` with a waypoint file prints `guided=1`, writes nothing to the hint log; a guide file
      with a bad line exits 2 naming the line.
- [ ] `sf-bench --planner new --set A --levels W1-1` records a completed, guided result, and with
      `--unguided` an unguided one; the table shows the `guided` column (may be `#[ignore]`).

## Done when

- [ ] Tests green; README section added: what `sf-plan` does, that a guide path (a committed route or
      a waypoint file) may steer it and how, and that hints exist, are logged, and are a last resort.
- [ ] `sf-plan W1-1` (original mode) writes a route that `sf-bench` verifies as completed.
