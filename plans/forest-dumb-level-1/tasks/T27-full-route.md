# T27 — full-route

**Phase:** 6 · **Depends on:** T20, T22, T24, T25, T26 · **Weight:** heavy

## Goal
The planner finds a complete route to the end of `W1-1`, checked on the engine and the original. This is the owner's requirement that the plan ends with the whole level planned.
## Design sections this implements
§2.11, §1.
## Reuses
`fd-plan`, T15's compare if a recording is needed, `games/superfrog/tools/{handover.py,recording_guide.py}` as pattern, `games/superfrog/docs/planner-stall-ask-owner.md`.
## Files
`fd-plan/tests/full_route.rs`, `games/forest-dumb/tools/{handover.py,recording_guide.py}` if needed, `games/forest-dumb/docs/` route note, `fd-plan/guides/` (cells only), `smoke.toml`.
## Interface
```
fd-plan level W1-1 --budget-mins 30 --out build/routes/W1-1.route
```
A route is inputs only and holds no disk data; it may be committed as a test fixture.
## Tests
- [ ] Route replayed on the engine reaches the level-end milestone with no life lost.
- [ ] The same route on the original passes the case and reaches the milestone.
- [ ] No exclusions but sound in that case.
- [ ] No `Event::Unported` over the whole route, and every `PORTED` routine shows non-zero hits on the original in some case; a routine with none is named and raised with the owner.
- [ ] The engine alone meets every milestone within tolerance.
## Done when
- [ ] Route and checks pass, in full; a committed route guards it.
- [ ] Any owner-played stretch is in FINDINGS with its date.
## Needs a person
Only if the planner stalls after a tuned budget. The worker shows where on the map page and prepares the app script, with the owner's key mapping.
```
{the app script handover prints, with --timeout}
```
Expect: the app fast-forwards to the stuck place, recorder on, then pauses for you.
Tell me: play that stretch, press Stop, save, and say you have done it.
