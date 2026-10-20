# T25 — controls-and-gamepad

**Phase:** 3 · **Depends on:** T24 · **Weight:** light

## Goal

Remappable keys and gamepad support: gilrs feeding the same mapping onto the original's joystick and
keys (DESIGN §2.6), a Controls window to rebind, hot-plug, and the pad path tested with simulated
input (owner, 2026-10-15: the 8BitDo hardware check is optional and not a gate). SDL2's controller support is the fallback if gilrs does not handle the pad.

## Design sections this implements

DESIGN §2.6, §2.9, §5.1.

## Files

- `engine/crates/sf-app/src/{pad.rs,controls_window.rs}`; `menu.rs` — Game → Controls…
- `engine/crates/sf-app/src/input.rs` — pad mapping joins key mapping.
- `engine/crates/sf-e2e/tests/controls.rs`

## Interface

```rust
pub struct PadMap(pub BTreeMap<OriginalInput, Vec<PadInput>>);   // default: d-pad + left stick (threshold 0.5) for directions, A/B for fire
pub fn map_input(keys: &KeyState, pads: &PadState, km: &KeyMap, pm: &PadMap) -> FrameInput   // pure
```

## End to end (the worker drives this)

- suite: `sf-e2e` · sizes: 2×
- [ ] rebind fire to X in the Controls window via the OS layer → X fires, Space no longer does; persists across relaunch.
- [ ] binding one key to two inputs is refused with a message, or resolved as the window says.
- [ ] reset to defaults restores both maps.

## Tests

- [ ] `map_input`: every default binding; key and pad held together; stick inside the dead zone ignored; diagonal.
- [ ] hot-plug events from a fake gilrs source: connect, disconnect mid-move releases held inputs on that device only.

## Done when

- [ ] Tests and e2e green.
- [ ] A FINDINGS row says the 8BitDo on hardware is unverified (unless the owner chose to try it).
- [ ] Bindings persist in `settings.toml`.

## Optional, if the owner wants to try the pad (not a gate)

```
cd engine && cargo run --release -p sf-app -- --max-frames 15000
```
(the worker has built and imported first; the frame cap ends it after about five minutes)

Expect: with the 8BitDo Ultimate 2 connected (Bluetooth or USB, say which), the d-pad and stick move
the frog in W1-1, A jumps, unplugging and replugging mid-level loses nothing.
Tell me: which mode it was in, whether every input worked, and anything that felt laggy.
