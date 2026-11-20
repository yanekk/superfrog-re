# T39 — app-stop

**Phase:** 7 · **Depends on:** T37 · **Weight:** medium

## Goal

What the app does on a stop (owner, 2026-11-11): the game freezes, a line names the stop and its address, a bug recording of the play from its start is saved to the bug folder automatically in the existing bug zip format, and the player can quit. Recordings start where play started (power-on or `--level 1`), and `fd-conform bug replay` replays them on the translated engine.

## Design sections this implements

DESIGN §2.4, §2.10.

## Files

- `fd-app/src/main.rs`, `fd-app/src/stop.rs` (new: the frozen frame with the message drawn over it), `fd-core/src/recording.rs` (a start field: `power-on` or `level-1`), `fd-conform/src/bug.rs` (replay on `--engine translated`), `docs/BUG-FIX.md`.
- A test hook `--stop-after N` (seatbelt-style flag, documented) that raises a synthetic stop at frame N, so the path is testable without a real defect.

## Tests

- [ ] Unit: a stop at frame N writes a zip whose recording replays to frame N on the translated engine and stops the same way.
- [ ] Old level 1 bug zips still replay on the hand engine.

## End to end (the worker drives this)

- suite: `fd-e2e` · sizes: the smallest window size
- [ ] `--stop-after 120` → the picture freezes, the message line is fully visible at the smallest size, a zip appears in the bug folder within 2 s
- [ ] Esc or ⌘Q after the stop quits cleanly

## Done when

- [ ] End-to-end tests pass; `docs/BUG-FIX.md` covers a stop zip.
