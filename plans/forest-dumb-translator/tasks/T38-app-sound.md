# T38 — app-sound

**Phase:** 7 · **Depends on:** T37 · **Weight:** medium

## Goal

The app plays the game's sound (owner, 2026-11-11): each frame's Paula writes go through `amiga-paula` to `amiga-app`'s audio, as Superfrog's app does, paced by the audio clock.

## Design sections this implements

DESIGN §2.10.

## Files

- `fd-app/src/main.rs`, `fd-app/src/audio.rs` (new, following `sf-app`'s use of `amiga_paula` and `amiga_app::audio`), `fd-app/README.md` (the mute option, if the app gains one).

## Tests

- [ ] Unit: a frame's Paula writes turn into the same samples as `amiga-paula` given them directly.
- [ ] Headless: the title's first 300 frames of samples equal across two runs (deterministic).

## End to end (the worker drives this)

- suite: `fd-e2e` · sizes: one
- [ ] launch, capture audio over the title → non-silent, and equal to the headless samples for the same frames
- [ ] level 1 → a sound effect at the frame the headless run plays it

## Done when

- [ ] Both end-to-end tests pass in full; the capture's audio equals headless.
