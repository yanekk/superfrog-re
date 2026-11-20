# T10 — title-app-paths

**Phase:** 3 · **Depends on:** T01 · **Weight:** medium

## Goal

The title's paths are driven in the app by key presses, each equal to a headless run of the translated
program every step: the rows, every code T01 lists, a wrong code, the cheat code, Esc (the Amiga key,
through a binding), KONIEC (already held, reused), and game over back to the title. The app test is
the owner's real input path; the headless run is what the cases judge against the original.

## Design sections this implements

DESIGN §2.2, §2.6; brief "The app".

## Files

- `games/forest-dumb/engine/crates/fd-e2e/tests/title_paths.rs` (new, full) and
  `title_paths_short.rs` (smoke, the rows and one code, a few seconds), labelled in
  `games/forest-dumb/smoke.toml`; an `[[area]]` for the title's codes.
- `fd-e2e/README.md`: the new tests.

## End to end (the worker drives this)

- suite: fd-e2e, script layer, as `translated_drill.rs` (`follow`, `press`) · size: default scale
- [ ] Down/Up through the rows → `title_cursor` and the drawn rows equal headless each step.
- [ ] Each code by its keys, then START → the world's SCENA screen and play, equal headless.
- [ ] A wrong code → what T01 measured, equal headless.
- [ ] The cheat code, START, HELP (bound `"key $5f"`) → the level ends, equal headless.
- [ ] Amiga Esc bound to a host key → what T01 measured (quit with no zip, or nothing).
- [ ] World 2 by its code, deaths to game over → high scores → title, equal headless.

## Tests

The list above; each test fails with a step and both states on the first difference.

## Done when

- [ ] Every path above passes in the app, equal to headless every step.
- [ ] The short test is in smoke and its time noted; the rest are full.
