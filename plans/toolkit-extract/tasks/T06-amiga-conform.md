# T06 — amiga-conform

**Phase:** 3 · **Depends on:** T05 · **Weight:** medium

## Goal

Move the game-free half of the comparison harness into `amiga-conform`: picture and sound comparison
with their tolerances, the sound-event log, the diff picture and report writing, the per-channel
sound judge, the audio swing, and the input-script lexer with a hook for a game's own tokens. A
second game compares itself with the original through the same judges Superfrog uses. Everything
tied to Superfrog's cases, memory map, levels or milestones stays (owner, 2026-10-28).

## Design sections this implements

DESIGN §2.2 row `amiga-conform`, §2.6, §7.

## Files

- New: `toolkit/crates/amiga-conform/src/{compare,report,sound_events,channels,feed,script}.rs`
  (the moved halves; names the worker's), `tests/faithful.rs` (the synthetic part).
- `engine/crates/sf-conform/src/{compare,report,sound_events,channels,feed,script,case,lib}.rs`,
  `tests/faithful.rs` (keeps its oracle-backed tests), `Cargo.toml`.
- `docs/architecture/verification.md` `owns`, moved-path table.

## Interface

```rust
// amiga_conform
pub enum Mode { .. }  pub struct Tolerance { .. }  // OWNER unchanged
pub fn picture(ours: &Picture, theirs: &amiga_oracle::Rgb12Frame) -> Option<PixelMismatch>;  // as today's
pub struct PictureJudge; pub enum PictureClass; pub fn classify(..);
pub fn paula(..);  pub fn reg_name(addr: u32) -> &'static str;
pub mod report { pub fn write_diff_png(..); pub fn png(..); pub fn crc32(..); }
pub mod channels { pub fn level(..); pub fn mixer_levels(..); pub fn judge(..); }
pub mod script {
    pub fn parse_with(text: &str, game_token: impl Fn(&str) -> Option<Result<FrameInput, String>>)
        -> Result<InputScript, String>;
}
```

Where a moved function took a `Case`, the toolkit form takes the fields it read; sf-conform keeps a
wrapper with today's signature. `compare.rs`'s panel rows (`PANEL_ROWS`, `VarId::PanelBitmap`) are
Superfrog's and are passed in as an excluded-rows parameter.

## Tests

- [ ] The synthetic `faithful.rs` tests (today lines 50–232) and `compare`'s unit tests pass in the
      toolkit with no disk.
- [ ] `sf-conform`'s `tests/script.rs`, `faithful.rs` (oracle part), `channels --ignored` give the
      same results as T01's base (the `T216` label assert stays the known red).
- [ ] Superfrog's `C<entry>.<k>` token parses through the hook as today.
- [ ] `Tolerance::OWNER` byte-identical (a test comparing it with the base's values).

## Done when

- [ ] `amiga-conform` holds DESIGN §2.6's list, no `Case`, `VarId`, `FrameOut` or milestone in it.
- [ ] Test block green; DESIGN §4's per-task runs green.
