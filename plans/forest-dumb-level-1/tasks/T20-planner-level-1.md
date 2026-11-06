# T20 — planner-level-1

**Phase:** 4 · **Depends on:** T11, T17, T19 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T20` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again. A reviewer's pack has no FINDINGS: read it after your cold read.

## Goal
The planner works on `W1-1`, reaches the furthest point the built engine allows, and shows the route on the map page.
## Design sections this implements
§2.11.
## Reuses
`amiga-plan`, T19; `sf-plan/src/{app,main,oracle}.rs` and `facts.rs` as pattern; route confirm via `fd-conform`.
## Files
`games/forest-dumb/engine/crates/fd-plan/{src/{main.rs,game.rs,facts.rs},tests/route.rs}`, `fd-routeview`, `smoke.toml`.
## Interface
```
pub struct PlanState(Game);   impl PlanGame for PlanState { .. }   // a newtype: the orphan rule bars implementing it on fd-core's Game here
pub fn facts(level: &Level, data: &GameData) -> Facts
fd-plan level W1-1 [--guide FILE] [--budget-mins N] [--out FILE]
```
## Tests
- [ ] `facts` match T00's readings.
- [ ] A route reaches a recorded expected point on replay.
- [ ] Same result across runs and thread counts.
- [ ] Budget stops and reports.
- [ ] The route's stretches confirmed on the original where modelled.
## Done when
- [ ] A test replays the planned route on the engine to the expected point, and the page shows it.
- [ ] A short version is smoke with an `[[area]]`; the longer one is in full.
