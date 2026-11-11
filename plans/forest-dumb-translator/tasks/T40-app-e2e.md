# T40 — app-e2e

**Phase:** 7 · **Depends on:** T38, T39 · **Weight:** medium

## Goal

One end-to-end run of the whole translated app as a person would play it: launch, title with sound, Start, level 1 with the hand engine mounted, the level's route to its end (the committed route `fd-plan/tests/routes/W1-1.txt`), into world 2's load and first frames, every frame equal to headless.

## Design sections this implements

DESIGN §2.10, success criteria.

## Files

- `fd-e2e/tests/translated_whole.rs` (new, full, `--ignored` like `finish_level.rs`), `fd-e2e/tests/translated_short.rs` (new, smoke if under 10 s; else full), `games/forest-dumb/smoke.toml` labels and an `[[area]]` for the translated app.

## End to end (the worker drives this)

- suite: `fd-e2e` · sizes: as `finish_level.rs`
- [ ] launch → title, audio non-silent → Start → level 1 → route played by key presses → level complete → world 2 loads → 300 frames of world 2, state equal to headless every frame

## Done when

- [ ] `translated_whole.rs` passes; its time is recorded in FINDINGS and `smoke.toml` labels it.
