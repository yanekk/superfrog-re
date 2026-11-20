# T76 — stale-hits-taken-exclusion

**Phase:** 6 · **Depends on:** —; blocks T27 · **Weight:** small

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T76` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T47 with the owner's yes (2026-11-09), as DESIGN §4.2 asks for a red seen on the base branch. `fd-conform/tests/pass_input.rs` `up_pressed_and_released_mid_scroll_matches_and_reaches_the_jump` fails on `dbc0b2c0` and on the feature branch after T73 and T63: `character-run-slide-buttons` excludes `var hits_taken` (T24), and the runner fails it with "exclusion `var hits_taken` (T24) hid no difference: remove it". The counter now matches the original on every pass.

## Goal
Compare `hits_taken` in `character-run-slide-buttons` and remove the stale exclusion, so the test is green again.

## Design sections this implements
§2.2, §4.2.

## Reuses
T67's case and test as they are.

## Files
`engine/conform/cases/character-run-slide-buttons.toml`.

## Interface
None.

## Tests
- [ ] `pass_input.rs` passes, `hits_taken` compared on every pass of `character-run-slide-buttons`.

## Done when
- [ ] No exclusion in `character-run-slide-buttons` names T24.
- [ ] The fix commit ends with the trailer `Escape: forest-dumb-level-1 T24` (owner, 2026-11-09).
