# T02 — amiga-input

**Phase:** 2 · **Depends on:** T01 · **Weight:** light

## Goal

Move the Amiga's joystick and keyboard input, and the text format inputs are written in, into a
toolkit crate `amiga-input`, leaving the routine that reads keys at Superfrog's addresses where it
is. The oracle, the conformance scripts, the recorder and the app all speak this format, so later
moves depend on it.

## Design sections this implements

DESIGN §2.2 row `amiga-input`, §2.4, §3.1, §3.3.

## Files

- New: `toolkit/crates/amiga-input/` (`src/lib.rs`, `src/keyboard.rs` or one file).
- `engine/crates/sf-core/src/input.rs` (becomes `pub use amiga_input::*`), `sf-core/src/keyboard.rs`
  (keeps `kbd_poll`, `KbdHost`, `polls`, the three addresses; re-exports `key_byte`, `CiaKeyboard`),
  `sf-core/Cargo.toml`, `sf-oracle/src/lib.rs` re-exports.
- `engine/crates/sf-conform/tests/purity.rs`, the toolkit purity list, `sf-app/build.rs` `GAME`,
  `docs/architecture/frame-interrupts-input.md` `owns`, `tools/architecture.py` moved-path table.

## Interface

```rust
// amiga_input (pure; serde)
pub struct Joy(..);           // as today, with PORT1_BUTTON etc.
pub struct KeySet(..);
pub struct FrameInput { .. }
pub const JOY_TOKENS: ..;
pub fn format_tokens(i: &FrameInput) -> String;
pub fn parse_token(t: &str) -> Result<.., ..>;
pub fn parse_tokens(s: &str) -> Result<FrameInput, ..>;
pub fn key_byte(code: u8, down: bool) -> u8;
pub struct CiaKeyboard { .. } // power_on, from_keys, feed; serde form unchanged
```

`Joy`'s doc comment loses "the crack intro on disk 1"; a `learned on:` line may keep why port 1's
button matters.

## Tests

- [ ] Every existing input and keyboard unit test passes from its new home.
- [ ] `CiaKeyboard` and `KeySet` serialise to the same bincode bytes as before (save states).
- [ ] Token format round-trips every token in `JOY_TOKENS` and a full key set.
- [ ] sf-conform `tests/level_routines.rs` `kbd_poll` case and `tests/script.rs` pass.

## Done when

- [ ] `amiga-input` is in the toolkit, pure, named nothing of the game; Superfrog's key-reading
      routine stays in sf-core.
- [ ] Test block green; DESIGN §4's per-task runs green.
