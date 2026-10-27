# T10 — app-wiring

**Phase:** 2 · **Depends on:** T05, T06, T07, T08, T09 · **Weight:** medium

## Goal

The app's View ▸ Widescreen plays the widescreen game, not only its picture: every place that sets the
view sets the mode, the recorder writes switches, the app's widescreen tests describe the new game, and
the README and docs/12 say what widescreen now is.

## Design sections this implements

DESIGN §2.1, §2.7, §2.8, §2.9.

## Files

- `engine/crates/sf-app/src/main.rs` (`toggle_widescreen`, start-up), `menu.rs`, `levelfile.rs` (every
  `set_view(window::view(..))` site also calls `set_widescreen`)
- `engine/crates/sf-app/src/` recorder (header field and `W` tokens on switches while recording)
- `engine/crates/sf-app/src/main.rs` `--replay` (DESIGN §2.8): the game's mode and the view from the
  recording at its start and at each toggle, the saved setting untouched
- `engine/crates/sf-e2e/tests/view.rs` (rewrite the widescreen tests: the centre-equals-4:3 test holds only
  for widescreen off; widescreen on is app == headless engine with the mode, sides non-colour-0 at both
  level ends)
- `engine/crates/sf-e2e/tests/recorder_app.rs` (`a_recording_in_widescreen_saves…` replaced: a widescreen
  recording replays as played and its screenshot is the wide picture)
- `engine/crates/sf-e2e/tests/widescreen_app.rs` (new)
- `README.md` (the View line: widescreen now widens the game), `docs/12-display.md` (widescreen section)

## Interface

No new public API. Script event `menu widescreen` keeps its name.

## Tests

- [ ] app with widescreen on from launch, W3-1 by Continue, 2 000 frames: every frame equals a headless
  `Game` with `set_widescreen(true)` and the same inputs; `probe()` identical.
- [ ] app with widescreen off: unchanged from today's tests (the 4:3 game).
- [ ] mid-level `menu widescreen` at frame 600 and again at 1 200: app equals headless with switches at
  the same steps.
- [ ] at W1-1's left end and a level's right end in widescreen: no colour-0 column from off the map in the
  play-field lines.
- [ ] recorder: a recording with two switches replays (sf-conform `bug replay`) to `screenshot equal`.
- [ ] app `--replay` of that recording, launched with widescreen off in settings, plays and draws as
  recorded (frames equal `bug replay`'s) and leaves `settings.toml`'s widescreen as it was.
- [ ] widescreen setting persists (existing test keeps passing).

## End to end (the worker drives this)

- suite: `sf-e2e` (`view.rs`, `widescreen_app.rs`, `recorder_app.rs`) · sizes: 2× widescreen,
  fullscreen widescreen
- [ ] the tests above, through the real menu and key input.

## Done when

- [ ] e2e green; README and docs/12 updated in this commit.
- [ ] `cd engine && cargo test -q --workspace` run once, recorded.
