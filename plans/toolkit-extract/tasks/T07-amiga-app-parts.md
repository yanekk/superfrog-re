# T07 — amiga-app-parts

**Phase:** 2 · **Depends on:** T02 · **Weight:** medium

## Goal

Move the app's game-free parts into a library crate `amiga-app`: sound output, PAL frame pacing,
gamepads, the picture's GPU path and CRT filters, screen and audio capture, atomic file writes, the
single-copy lock, and the Amiga key table with its key map. `sf-app` keeps its run loop, script
commands, menus and windows and calls these (owner, 2026-10-28: no game interface is designed here).

## Design sections this implements

DESIGN §2.2 row `amiga-app`, §2.7 (capture names), §3.2, §7.

## Files

- New: `toolkit/crates/amiga-app/src/{lib,atomic_write,audio,pacing,pad,lock,video,capture,keys}.rs`,
  the shaders `crt*.wgsl`, `port.wgsl` (GPL headers on `crt_geom.wgsl`, `crt_easymode.wgsl` kept).
- `engine/crates/sf-app/src/` (those files become uses of the crate or are deleted; `capture.rs`
  keeps `--capture-hashes` and `unported.txt`, which read `sf_core::feed` and `Game`; `input.rs`
  keeps the ⌘ shortcuts to `menu::Command` and the mouse-as-`PORT1_BUTTON` feed), `sf-app/Cargo.toml`.
- `docs/architecture/app.md` `owns`, moved-path table.

## Interface

```rust
// amiga_app (shell)
pub mod atomic_write { pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()>; }
pub mod lock { pub fn acquire(dir: &Path, app_name: &str) -> Result<Lock, String>; } // message names app_name
pub mod pacing { /* as today, from amiga_paula's constants */ }
pub mod audio; pub mod pad;
pub mod video { pub struct Video; /* draws an amiga_chips::Picture, filters as today */ }
pub mod capture {
    pub const FRAME: &str = "frame-{:05}.png"; /* window-, windows.txt, inputs.txt, prompts.txt, audio.wav */
    pub struct Capture; // frames, inputs, audio; game extras added by the caller
}
pub mod keys { pub enum OriginalInput; pub const AMIGA_KEYS: ..; pub struct KeyMap; }
```

Log prefixes and messages that say `superfrog:` take the app's name as a parameter.

## Tests

- [ ] The moved files' unit tests pass in the toolkit.
- [ ] sf-app's tests pass; sf-e2e's capture-based tests (`recorder_*`, a capture and a pacing test of
      the worker's choice) pass, showing the app's files are unchanged.
- [ ] A capture written by the app is byte-equal to one from the base for the same short script
      (frame PNGs, `inputs.txt`).

## Done when

- [ ] `amiga-app` holds the listed parts, depends on no `sf-*` crate, names nothing of the game.
- [ ] The app builds, runs and captures as before; test block and DESIGN §4's per-task runs green.
