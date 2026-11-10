# T84 — one-route-both-sides

**Phase:** 6 · **Depends on:** T48, T82; blocks T28, T29 · **Weight:** heavy

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T84` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T48's review with the owner's yes (2026-11-10). DESIGN §1 asks for one route that reaches the end of `W1-1`, with no life lost, on the engine and on the original. T48 left two: T27's (`fd-plan/tests/routes/W1-1.txt`) finishes on the engine alone and ends elsewhere on the original; T48's (`engine/conform/scripts/route-W1-1.txt`) finishes on the original, all 21 764 passes agreeing, and does not finish on the engine alone. The owner requires one route for both (2026-11-10).

## Goal
One input script reaches the level's end with no life lost on the engine alone and on the original, and it is the route the app plays (T28). Both guards play the same inputs.

## Why the two differ (measured by T27, T48, T79)
The engine alone does not know which step each pass of the original read its joystick in (`PassInputs`, a pass can read a step late) or which pass cut a redraw (`EdgesCut`), so a creature is started a pass apart and is met a pass apart. On T48's run 28 passes left the engine-alone forecast, all by a creature started a pass apart. T27's route meets contact on 106 passes, never three in a row, so it has no slack. Not measured: whether a route with slack at every such meeting exists.

## Approach, not binding
Start from T48's tool (`plans/forest-dumb-level-1/prototype/t48-route-on-original/onoriginal.rs`, its README says how to run it), which plans from the original's states and forecasts on the engine alone (`Game::own_cuts`). Add the second condition: a candidate is kept only if the engine alone, from the level's start, also finishes it with no life lost. Where one side loses a life, repair with inputs that hold on both, preferring slack (a jump a few steps early, a wait) where a creature is met.

## Rules the owner set (2026-11-10)
- One session. If no such route is found, stop, name the stretch that blocks it (steps, spot as `W1-1:col,row`, which side fails and how) and ask the owner; never loosen a comparison or add an exclusion to get there.
- A new comparison rule is asked first and follows T71's pattern (T82's rules).
- T48's case `route-W1-1` stays as it is and keeps passing: it is the engine against the original over the whole level.
- No existing task is edited.

## Design sections this implements
§1, §2.1, §2.11, §4.

## Reuses
T48's tool and `Game::own_cuts`, `fd_conform::PassInputs::next_read`, `fd-plan`'s planner and `tests/full_route.rs` (T27's guard), `fd-conform/tests/full_route.rs` (T48's guard and its `Watched` engine), T82's picture rules, `games/forest-dumb/docs/13-full-route.md`.

## Files
`fd-plan/tests/routes/W1-1.txt` and `fd-plan/tests/full_route.rs` (its marks), a new case `engine/conform/cases/route-both-W1-1.toml` with its script, `fd-conform/tests/full_route.rs`, `games/forest-dumb/docs/13-full-route.md`, `games/forest-dumb/CLAUDE.md` ("Planning a route"), `fd-plan/README.md`, `smoke.toml`, a tool under `plans/forest-dumb-level-1/prototype/` if one is written; `fd-core/src/**` for fixes only.

## Interface
Route, case and test files. No new engine API unless a difference needs a fix.

## Tests
- [ ] The one route replayed on the engine alone reaches the level's end with no life lost (`fd-plan/tests/full_route.rs`).
- [ ] The same route's case on the original passes with the level's end as its goal: every variable `route-W1-1` compares, the whole picture with no exclusion, no life lost, `Event::LevelEnd` the only event.
- [ ] A test holds that both guards play the same inputs, step for step.
- [ ] `route-W1-1` still passes (`the_route_reaches_the_levels_end_on_the_original_with_no_life_lost`).
- [ ] Its opening on both sides is smoke if it fits (`smoke-times.tsv`); the whole route is full.

## Done when
- [ ] The tests above pass, or the owner has answered the stretch that blocks them.
- [ ] `docs/13` says which route is which, how the one route was found and how to find it again after an engine change.
- [ ] `tools/smoke.sh --check-only` is clean.
