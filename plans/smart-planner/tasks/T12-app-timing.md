# T12 — app-timing

**Phase:** 2 · **Depends on:** T11 (and `pir/bug-recorder` merged into main, DESIGN D3) · **Weight:** light

## Goal

Plan routes that play in the app: start the level the way Game ▸ Start Level… starts it under a script
and plan and confirm on the engine alone, so recordings, regression cases and drills get routes that do
not die from being paced to the original's lag (bug-recorder T06, T11).

## Design sections this implements

DESIGN §2.1 (`--app`), §2.6 (app mode), §4.1 set C, D3.

## Before starting

`git grep -n 'pub fn start_level' -- engine/crates/sf-core` must find `Game::start_level`. If it does
not, stop and ask the owner: bug-recorder is not merged yet, and this task must not copy its API.

## Files

- `engine/crates/sf-plan/src/main.rs` (`--app`), `engine/crates/sf-plan/src/app.rs`
- `engine/crates/sf-bench/src/{sets,verify}.rs` (set C start and verification)
- `engine/crates/sf-plan/tests/app.rs`

## Interface

```
sf-plan <level> --app [--seed N] [--out route.txt] ...
pub fn app_start(level: LevelId, seed: u32) -> Game;   // Game::start_level(display, &LevelSetup{level, ..Default::default()}, Rng::seeded(seed, seed))
```

The seed pair matches what the recorder's script `start-level` event uses; read it from bug-recorder's
code rather than assuming `(0, 0)` (`plan_exit --engine` used `Rng::seeded(0,0)`).

## Tests

- [ ] W1-1 `--app --seed 1` plans to the exit; the route replays from `app_start` to `level_state` ≠ −2
      with no life lost; twice → identical (test block, engine only).
- [ ] The W1-1 shaft at column 92 (bug-recorder T11, 2026-10-20) is not a stall: planning finishes
      under 2 min.

## Done when

- [ ] Tests green; `sf-bench --set C --planner new --levels W1-1` records a completed result.
