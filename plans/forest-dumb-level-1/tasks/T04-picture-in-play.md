# T04 — picture-in-play

**Phase:** 1 · **Depends on:** T00 · **Weight:** light

## Goal
Capture the whole play picture from the oracle, because `Rgb12Frame` holds only the 320x56 status bar during a level: the crop uses the frame-end DIW and the game moves it mid-frame.
## Design sections this implements
§7; `plans/forest-dumb-start/FINDINGS.md`.
## Reuses
`crop` (`amiga.rs:726-758`), `Picture::rewindow` (`amiga-chips/src/picture.rs:51`). `picture()` is left alone since Superfrog depends on it. No vAmiga patch.
## Files
`toolkit/crates/amiga-oracle/src/amiga.rs`, a test in `toolkit/crates/amiga-oracle/tests/`, `games/forest-dumb/engine/crates/fd-conform/tests/play_picture.rs`, `smoke.toml`s.
## Interface
```
impl Amiga { pub fn picture_in(&self, diwstrt: u16, diwstop: u16) -> Rgb12Frame }
```
## Tests
- [ ] In play, `picture_in` with T00's window holds the play area and status bar, not 320x56.
- [ ] With the frame-end window it equals `picture()` on the title.
- [ ] Odd or empty windows do not panic.
- [ ] Superfrog's trace format constant is unchanged.
## Done when
- [ ] A saved frame was opened and judged by the worker, recorded in FINDINGS as worker-driven.
- [ ] Toolkit and fd-conform tests are green and labelled.
