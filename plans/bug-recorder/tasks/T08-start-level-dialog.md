# T08 — start-level-dialog

> **Amendment (T00 review, 2026-10-20, from D12).** The Difficulty dropdown offers Easy and Normal
> only, and Lives defaults to 3 (DESIGN §2.1). The end-to-end list reads `Easy/Normal` for the
> difficulty values and `easy` for the W1-2 start. `prototype/` still shows Hard; it is non-binding.

**Phase:** 2 · **Depends on:** T06 · **Weight:** medium

## Goal

The real Start Level window behind Game ▸ Start Level…: the owner's dialog with its dropdowns,
points field, toggles and the recorder switch, returning a `LevelSetup` to T06's recorder.

## Design sections this implements

DESIGN §2.1, D1 (`prototype/index.html`, non-binding).

## Files

- `engine/crates/sf-app/src/start_level_window.rs` (new): a model without AppKit plus the view, as `controls_window.rs`
- `engine/crates/sf-app/src/import.rs`: `NativeUi::start_level`
- `engine/crates/sf-app/Cargo.toml`: objc2-app-kit features (NSPopUpButton, NSWindow as needed), one line of why
- `engine/crates/sf-e2e/tests/recorder_start_level.rs` (new)

## Interface

```rust
pub struct StartLevelForm { pub setup: LevelSetup, pub record: bool, pub points_text: String }
impl StartLevelForm {
    pub fn new() -> Self;                                     // DESIGN §2.1 defaults
    pub fn apply(&mut self, a: FormAction) -> Option<Result<(LevelSetup, bool), String>>; // Start / Cancel
}
pub enum FormAction { Level(LevelId), Lives(Lives), Points(String), Energy(u8),
                      Difficulty(Difficulty), Spud(bool), Wings(bool), Record(bool), Start, Cancel }
```

The window is modal to the game: the game pauses while it is open. Points that are not a whole
number 0–999 999 disable Start and show the range beside the field. Every control has an
accessibility identifier for the OS layer. Return selects Start, Esc Cancel.

## End to end (the worker drives this)

- suite: `sf-e2e` script layer (block) and OS layer (`SF_E2E_OS=1`) · sizes: scale 1×, 2×, 3×, fullscreen
- [ ] OS: Game ▸ Start Level… opens the window with the defaults
- [ ] OS: each dropdown offers exactly its values (29 levels, 3/5/7, 1–4, Easy/Normal)
- [ ] OS: typing 1000000 into Points disables Start; 450000 enables it
- [ ] OS: Start with W1-2, 7 lives, easy, wings, recorder on → W1-2 starts, `frog_lives` 6, recording on (stderr line)
- [ ] OS: Esc closes it and the game resumes unchanged
- [ ] script: `form …` events drive the model as `controls …` does; unit tests cover every `FormAction` and refusal

## Done when

- [ ] the window works as above and the end-to-end list passes (OS layer run by the worker, recorded in PROGRESS)
- [ ] the test command is green
