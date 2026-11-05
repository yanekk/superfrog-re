# T27 — full-route

**Phase:** 6 · **Depends on:** T20, T26, T45, T46, T47 · **Weight:** medium

## Goal
The planner finds a complete route to the end of `W1-1` on the engine. This is the owner's requirement that the plan ends with the whole level planned. Checking that route on the original is T48's, in its own session (DESIGN §4.3).
## Design sections this implements
§2.11, §1.
## Reuses
`fd-plan`, `games/superfrog/tools/{handover.py,recording_guide.py}` as pattern, `games/superfrog/docs/planner-stall-ask-owner.md`.
## Files
`fd-plan/tests/full_route.rs`, `games/forest-dumb/tools/{handover.py,recording_guide.py}` if needed, `games/forest-dumb/docs/` route note, `fd-plan/guides/` (cells only), `smoke.toml`.
## Interface
```
fd-plan level W1-1 --budget-mins 30 --out build/routes/W1-1.route
```
A route is inputs only and holds no disk data; it may be committed as a test fixture.
## Tests
- [ ] Route replayed on the engine reaches the level-end milestone with no life lost.
- [ ] The engine alone meets every milestone within tolerance.
## Done when
- [ ] The route passes on the engine; a committed route guards it.
- [ ] Any owner-played stretch is in FINDINGS with its date.
## Needs a person
Only if the planner stalls after a tuned budget. The worker shows where on the map page and prepares the app script, with the owner's key mapping.
```
{the app script handover prints, with --timeout}
```
Expect: the app fast-forwards to the stuck place, recorder on, then pauses for you.
Tell me: play that stretch, press Stop, save, and say you have done it.
