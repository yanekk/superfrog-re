# T23 — app-shell

**Phase:** 3 · **Depends on:** T21, T22 · **Weight:** medium

## Goal

Make the minimal window into the game's real shell: the first-run disk import with a native file
picker and the launch decision from T09, keyboard input mapped onto the original's inputs, integer
scaling in a resizable window, fullscreen, remembered settings and progress, the single-instance
lock, and the silent fallback when there is no audio device.

## Design sections this implements

DESIGN §2.2, §2.5, §2.6, §2.7 (scaling, fullscreen), §2.9, §2.11, §3.5.

## Files

- `engine/crates/sf-app/src/{import.rs,settings.rs,progress.rs,lock.rs,atomic_write.rs,window.rs}`; `input.rs` (T22) — the fixed mapping becomes `KeyMap`
- `engine/crates/sf-app/src/main.rs` — wires import → cache → game loop.
- `engine/crates/sf-e2e/tests/{import.rs,window.rs,persist.rs}`

## Interface

```rust
pub struct Settings { pub scale: Scale /* Auto | N */, pub fullscreen: bool, pub crt: bool, pub widescreen: bool, pub keys: KeyMap, pub pad: PadMap }   // settings.toml
pub struct KeyMap(pub BTreeMap<OriginalInput, Vec<KeyCode>>);   // default: arrows + Space/Ctrl for fire, plus the original's keys
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()>   // temp in same dir, fsync, rename
```

## End to end (the worker drives this)

- suite: `sf-e2e` (T22) · sizes: 1×, 2×, 3×, fullscreen on the built-in display (`--allow-fullscreen`, ≤ 300 frames)
- [ ] empty temp home → the import prompt appears; the script layer supplies the four zips → cache written, title reached.
- [ ] the picker cancelled, or only two disks chosen → the message names the missing disks; "Choose Disks…" reopens the picker, "Quit" exits with nothing written.
- [ ] a wrong zip offered → the message names the slot; a missing disk → names it; nothing written to the cache.
- [ ] corrupt the cache → next launch re-imports silently; also move the remembered disks → next launch asks again.
- [ ] arrow keys and fire move the frog in W1-1 exactly as the same `FrameInput` script does headless (frames equal).
- [ ] resize the window: the picture stays an integer scale, centred, never smeared; at a size below 1× it shows 1× cropped-free with the window's minimum size enforced.
- [ ] fullscreen on and off by the shortcut; frames identical to windowed.
- [ ] quit and relaunch: scale, fullscreen and furthest world are remembered.
- [ ] second instance shows the message and exits; kill -9 the first, relaunch: the stale lock is taken over.
- [ ] launch with audio output unavailable (test hook selects a null device): game runs at the same pace.
- [ ] OS layer: a real key press moves the frog.

## Tests

- [ ] `atomic_write` interrupted between write and rename leaves the old file intact.
- [ ] damaged `settings.toml` / `progress.toml` → defaults plus a message, game still starts.

## Done when

- [ ] The end-to-end cases above are green, the script-layer ones in the test block.
- [ ] Nothing is written outside `--home` in any test.
- [ ] Progress from `Game::progress()` is saved and offered at the next launch.
