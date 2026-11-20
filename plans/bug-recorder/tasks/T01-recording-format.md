# T01 — recording-format

> **Amendment (T00, 2026-10-20).** `Difficulty` is `{ Easy, Normal }` (opt_game_mode 0/1): the
> original's options screen cannot select hard, and the owner dropped it. `LevelSetup::default()`
> has `Lives::Three` (the game's own new-game value, `opt_lives` 0 at power-on; owner). `seed` is
> the `Start::PowerOn { seed }` the start (b) boots with (T02); the recording's frame 0 is the first
> step after which `Game::scene()` is `Level` or `Bonus`. A recording stores no load or pass data:
> T05's retiming reads both from a replay. Refuse `difficulty hard` like any unknown value.

**Phase:** 1 · **Depends on:** T00 · **Weight:** light

## Goal

Define what a recording is, once, in pure code both the app and sf-conform use: the dialog's values
with their ranges and defaults, the start's random state, every step's input, where it stopped and
the owner's note, as readable text; and give `sf_disk::zip` a writer so a bug zip can be made and
read back.

## Design sections this implements

DESIGN §2.1 (values, ranges, defaults), §2.5, §2.6, §2.11 (parser refusals), §3.1, D9.

## Files

- `engine/crates/sf-core/src/recording.rs` (new), `sf-core/src/lib.rs` (export)
- `engine/crates/sf-core/src/input.rs`: token format moved in from sf-conform (`script.rs` `format_input` and the token parser)
- `engine/crates/sf-conform/src/script.rs`: uses the moved tokens, behaviour unchanged
- `engine/crates/sf-disk/src/zip.rs`: `write_stored`, promoted from the test-only writer in
  `sf-disk/tests/common/zipwrite.rs` (`write_zip`, stored path) using zip.rs's public `crc32`; the
  tests' helper keeps only its deflated path, built on it (owner, plan review 2026-10-20: one writer)

## Interface

```rust
// sf_core::recording
pub enum Difficulty { Easy, Normal, Hard }            // opt_game_mode 0/1/2
pub enum Lives { Three, Five, Seven }                 // opt_lives 0/1/2
pub struct LevelSetup {
    pub level: LevelId, pub lives: Lives, pub points: u32 /* 0..=999_999 */,
    pub energy: u8 /* 1..=4 */, pub difficulty: Difficulty, pub spud: bool, pub wings: bool,
}
impl Default for LevelSetup { /* W1-1, Five, 0, 4, Normal, false, false */ }
impl LevelSetup {
    pub fn validate(&self) -> Result<(), String>;
    pub fn next_life(&self) -> u32;                   // next multiple of 200_000 above points
}
pub enum Location { Level { id: LevelId, col: u16, row: u16 }, Screen(&'static str) }
impl Display for Location { /* "W1-2:94,51" or "slot machine" */ }
pub struct Recording {
    pub build: String, pub setup: LevelSetup, pub seed: Rng /* meaning per T00 */,
    pub stopped_at: Option<Location>, pub note: String,
    pub inputs: Vec<FrameInput>,                      // one per engine step from the level's frame 0
}
impl Recording {
    pub fn to_text(&self) -> String;
    pub fn parse(text: &str) -> Result<Recording, RecordingError>; // RecordingError { line, why }
}
pub const ZIP_RECORDING: &str = "recording.txt";
pub const ZIP_SCREENSHOT: &str = "screenshot.png";

// sf_core::input
pub fn format_tokens(input: FrameInput) -> String;
pub fn parse_tokens(s: &str) -> Result<FrameInput, String>;

// sf_disk::zip
pub fn write_stored(entries: &[(&str, &[u8])]) -> Vec<u8>;
```

Inputs are stored run-length as script lines (DESIGN §2.6) but held expanded in memory: a long
session is still only a few MB expanded, and the app appends one per step.

## Tests

- [ ] round trip: a recording with every field set, every token kind, 100 000 frames, `parse(to_text) == self`
- [ ] the header lines in DESIGN §2.6 order; the input lines only where the input changes
- [ ] refusals with line numbers: unknown version, missing field, lives 4, energy 0 and 5, points 1 000 000, unknown difficulty, frame numbers not increasing, a bad token, an unknown level
- [ ] `next_life`: 0 → 200 000, 199 999 → 200 000, 200 000 → 400 000, 999 999 → 1 000 000
- [ ] a note with a `#`, unicode, and an empty note survive the round trip; a newline in a note is refused by `validate`
- [ ] `Location` displays `W1-2:94,51` and screen names
- [ ] `write_stored` output read back by `read_entries` with matching names, bytes and CRCs; an empty entry; a 2 MB entry
- [ ] sf-conform's script tests still pass unchanged after the token move; the purity test passes

## Done when

- [ ] the interface above exists with the tests listed, and the purity test is green
- [ ] sf-conform parses scripts through the moved token code with no behaviour change
- [ ] the test command is green
