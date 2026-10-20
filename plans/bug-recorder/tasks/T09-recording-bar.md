# T09 — recording-bar

**Phase:** 2 · **Depends on:** T06 · **Weight:** medium

## Goal

The bar under the game picture while recording: an indicator, the elapsed time and STOP, with the
window growing to hold it and the picture unchanged, in a window and in fullscreen.

## Design sections this implements

DESIGN §2.3, D4.

## Files

- `engine/crates/sf-app/src/recording_bar.rs` (new): native views in the game window
- `engine/crates/sf-app/src/window.rs`: sizes with the bar; `viewport` leaves room
- `engine/crates/sf-app/src/main.rs`: show/hide on recorder changes; STOP click → `Command::StopRecording`
- `engine/crates/sf-e2e/tests/window.rs`, `tests/recorder_bar.rs` (new)

## Interface

```rust
pub const BAR_HEIGHT: u32 = /* points, chosen in the task */;
pub fn window_size(scale: u32, widescreen: bool, bar: bool) -> (u32, u32);
pub fn viewport(window: (u32,u32), scale: Scale, bar: bool) -> (u32,u32,u32,u32);
```

The existing window-size tests keep asserting `GAME × n` with no bar and gain the bar case.

## End to end (the worker drives this)

- suite: `sf-e2e` script layer and OS layer · sizes: 1×, 2×, 3×, widescreen 2×, fullscreen
- [ ] script: on `start-level … record` the window is `GAME × n` plus the bar's height and the captured picture is unchanged in size and content; after stop it returns
- [ ] OS: the bar shows "Recording" and a time that advances; clicking STOP opens the save path (T06's `Ui::save_recording`, stub or T10's window)
- [ ] OS: ⌘. does the same; Game ▸ Stop Recording is disabled when not recording
- [ ] OS: in fullscreen the picture's scale leaves the bar fully on screen and nothing overlaps the picture
- [ ] OS: the window capture at every size has no clipped text in the bar

## Done when

- [ ] the bar behaves as above and the end-to-end list passes (OS layer recorded in PROGRESS)
- [ ] the test command is green
