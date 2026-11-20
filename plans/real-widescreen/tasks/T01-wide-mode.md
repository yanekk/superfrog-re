# T01 — wide-mode

**Phase:** 1 · **Depends on:** — · **Weight:** light

## Goal

Make widescreen a mode the game itself knows, and give every horizontal screen edge one home. After this
task the mode reaches every level step, the `Screen` type answers every edge DESIGN §2.2 names, and with
the mode off it answers the original's constants; no site uses it yet (T02–T05 do), so nothing plays
differently.

## Design sections this implements

DESIGN §2.1, §2.2, §3.1, §3.2, §3.4.

## Files

- `engine/crates/sf-core/src/level/screen.rs` (new), `level/mod.rs` (`LevelCtx.screen`)
- `engine/crates/sf-core/src/game.rs` (`set_widescreen`, `widescreen`)
- `engine/crates/sf-app/src/window.rs` (the test pinning `WIDE_EXTRA`)

## Interface

```rust
// sf-core/src/level/screen.rs
pub const WIDE_EXTRA: u16 = 65;           // lores columns each side (DESIGN §2.1)
pub const WIDE_CELLS: u16 = 5;            // ceil(65 / 16): spawn scans move by whole cells (§2.2)

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Screen { pub wide: bool }

impl Screen {
    pub fn extra(self) -> u16;            // 0 or WIDE_EXTRA
    pub fn extra_cells(self) -> u16;      // 0 or WIDE_CELLS
    // One method per edge of DESIGN §2.3–§2.6, each the original's constant when !wide, e.g.:
    pub fn right_despawn(self) -> u16;    // $170 (+65)
    pub fn left_despawn(self) -> u16;     // $30 (+65), added to the frame width
    pub fn sprite_hide_right(self) -> u16;// $150 (+65)
    // … the rest named by T02–T05 as they consume them; each lands with its test.
}

// sf-core/src/game.rs
impl Game {
    /// The widescreen game (DESIGN §2.1, §2.7): from the next step.
    pub fn set_widescreen(&mut self, on: bool);
    pub fn widescreen(&self) -> bool;
}
```

`Game.widescreen` is `#[serde(skip)]` like `view` (DESIGN §3.4): the app sets it. `LevelCtx` carries
`screen: Screen`, built from it each step.

## Tests

- [ ] every `Screen` method returns the original's constant with `wide: false`, and constant + 65 (or
  + 5 cells) with `wide: true`.
- [ ] `WIDE_EXTRA == sf_app::window::extra_columns(GAME.1)` (in `sf-app`).
- [ ] `set_widescreen(true)` on a 4:3 game changes no `State` byte and no picture until a later task
  reads it (a W1-1 run of 500 steps, both settings, identical states).
- [ ] `purity.rs` still passes.

## Done when

- [ ] Interface in place, tests green.
- [ ] `cd engine && cargo test -q --workspace` run once, result recorded in PROGRESS.
