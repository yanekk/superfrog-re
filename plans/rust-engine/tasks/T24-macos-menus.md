# T24 — macos-menus

**Phase:** 3 · **Depends on:** T23 · **Weight:** light

## Goal

The native macOS menu bar every setting and feature lives in (owner's decision: nothing new drawn in
the game). This task builds the menu framework and the core items; T25–T28 each add their own items
to it.

## Design sections this implements

DESIGN §2.7, §7 (native menus).

## Files

- `engine/crates/sf-app/src/menu.rs` — muda menu bar, command enum, dispatch into the app.
- `engine/crates/sf-e2e/tests/menus.rs`

## Interface

```rust
pub enum Command { ImportDisks, Scale(Scale), ToggleFullscreen, Pause, Reset, Quit /* T25–T28 add variants */ }
pub fn build_menu() -> MenuBar;  pub fn on_command(app: &mut AppState, c: Command);
```

Menus: Superfrog (About, Quit ⌘Q) · File (Import Disks…) · Game (Pause, Reset) · View (Scale 1×/2×/3×/Auto, Fullscreen ⌃⌘F).

## End to end (the worker drives this)

- suite: `sf-e2e` · sizes: 2×
- [ ] OS layer: click each item through System Events → its effect (scale changes the window size; fullscreen toggles; Import opens the picker; Quit exits cleanly with settings saved).
- [ ] script layer: each `Command` → same effect.
- [ ] Pause stops `Game::step` (frame counter stays) and resumes exactly where it was.
- [ ] Reset returns to power-on; frames then equal a fresh launch.

## Done when

- [ ] Every item reached through the real menu bar at least once (OS layer) and green.
- [ ] Keyboard shortcuts work through the OS layer.
- [ ] The command enum is the only path from menu to app state.
