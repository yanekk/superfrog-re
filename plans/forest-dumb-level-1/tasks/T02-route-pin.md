# T02 — route-pin

**Phase:** 0 · **Depends on:** — · **Weight:** light

## Goal
Pin Superfrog's planned routes across a refactor, because T17 moves the planner and today's determinism tests only compare runs within one build, so a move could change routes unnoticed.
## Design sections this implements
§3.6, §7.
## Reuses
`sf-plan/tests/common/mod.rs`, `sf-plan/tests/{reach,walk,explore}.rs`, the route file format.
## Files
`games/superfrog/engine/crates/sf-plan/tests/route_pin.rs`, `games/superfrog/engine/crates/sf-plan/tests/pins/` (SHA-1 only, no disk data), `games/superfrog/smoke.toml`.
## Interface
```
fn pinned(level: &str, seed: u64) -> [u8; 20]   // SHA-1 of the route the planner returns for this input
```
A short fixed-budget explore and walk on levels fast enough for smoke; a longer one under `#[ignore]`.
## Tests
- [ ] Same result with 1 thread and with several.
- [ ] A deliberate one-move change to an action order fails the pin (mutation checked by hand in the session, reported in the commit).
- [ ] Pins hold nothing disk-derived beyond a hash.
## Done when
- [ ] The pin test runs in smoke under 30 s and is labelled.
- [ ] The commit names how a person re-pins, and that a re-pin needs a reason.
