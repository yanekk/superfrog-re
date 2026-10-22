# T07 — app-replay

**Phase:** 2 · **Depends on:** T06 · **Weight:** light

## Goal

`superfrog --replay <zip>` plays a bug zip in the window for agents and tests: watch-only, ending on
the last frame, so a worker can show or capture what a recording does in the real app.

## Design sections this implements

DESIGN §2.7 (first bullet), §5.2.

## Files

- `engine/crates/sf-app/src/args.rs`, `main.rs`, `recorder.rs` (replay source)
- `engine/crates/sf-e2e/tests/recorder_replay.rs` (new)

## Interface

```
superfrog --replay <zip> [--capture DIR] [--timeout S] [--home DIR]
```

Starts with `Game::start_level(setup, seed)`, feeds the recorded inputs one per step, ignores the
game's keys and pads (menu commands except Pause, Quit and the View items are disabled), and after
the last input holds: no further steps, `superfrog: replay finished at frame N` on stderr. `--timeout`
defaults to 120 under `--replay` with `--capture`. A bad zip: a notice with the parser's reason, then
exit non-zero.

## Tests

- [ ] e2e: a zip made by T06's e2e path replayed with `--capture`: the last captured frame equals its `screenshot.png`; stderr has the finished line with the right N
- [ ] e2e: keys sent during the replay change nothing (frames equal a replay without them)
- [ ] e2e: a corrupt zip exits non-zero with the reason on stderr

## Done when

- [ ] `--replay` behaves as above with the tests listed
- [ ] the test command is green
