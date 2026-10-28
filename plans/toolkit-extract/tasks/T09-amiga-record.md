# T09 — amiga-record

**Phase:** 3 · **Depends on:** T03, T06, T07 · **Weight:** heavy

## Goal

Move the recorder's generic half into `amiga-record` (draft decision 5, task T6b): the recording of
per-frame inputs with seed, build tag and note, its zip of `recording.txt` and a screenshot, the
`Recorder` and `Replay` state machines, the headless replay loop and the screenshot check. The game
supplies a header type for its start point, stop point and toggles, and its text form. Superfrog
keeps using it, so it is tested now, and game two needs a recorder as soon as the owner plays it.
Every recording and bug zip written today must still parse and replay unchanged.

## Design sections this implements

DESIGN §2.2 row `amiga-record`, §2.8.

## Files

- New: `toolkit/crates/amiga-record/src/{lib,recording,recorder,replay,zip,check}.rs`.
- `engine/crates/sf-core/src/recording.rs` (keeps `LevelSetup`, `Difficulty`, `Lives`, `Location`,
  `SCREENS`, `MAX_POINTS`, `LIFE_EVERY`, `MAGIC`, widescreen, and implements the header trait),
  `sf-app/src/recorder.rs` (keeps `zip_name`, `named_level`, `bugs_dir`, `EndChoice`),
  `sf-conform/src/bug.rs` (keeps the oracle comparison, put-back, retime, case writing).
- Purity lists (`amiga-record` is pure), `sf-app/build.rs` `GAME`, chapter `owns`, moved-path table.

## Interface

```rust
// amiga_record (pure)
pub const ZIP_RECORDING: &str = "recording.txt";
pub const ZIP_SCREENSHOT: &str = "screenshot.png";
pub const MAX_FRAMES: u32 = 50 * 60 * 60 * 24;
pub trait GameRecording {
    type Header: Clone;                       // start point, stop point, toggles
    const MAGIC: &'static str;
    fn header_text(h: &Self::Header) -> String;
    fn parse_header(lines: &mut dyn Iterator<Item = &str>) -> Result<Self::Header, String>;
    // per-frame game tokens (Superfrog's `W`) via a hook, if the text form needs one
}
pub struct Recording<G: GameRecording> { pub build: String, pub seed: Vec<u8>, pub note: String,
                                         pub header: G::Header, pub inputs: Vec<FrameInput> }
pub enum Recorder<G> { Off, Recording(..) }   // start, step, frames, full, finish(stop) -> Recording
pub struct Replay;                             // new, next, finished, just_finished
pub fn read_zip<G>(bytes: &[u8]) -> Result<(Recording<G>, Vec<u8>), String>;  // the one read_zip
pub fn zip_bytes<G>(rec: &Recording<G>, png: &[u8]) -> Vec<u8>;
pub trait Replayable { fn step(&mut self, i: FrameInput); fn picture(&self) -> Picture; }
pub fn replay<R: Replayable>(r: &mut R, rec_inputs: &[FrameInput], each: impl FnMut(u32, &R)) -> ..;
pub fn screenshot_differs(png: &[u8], pic: &Picture) -> Result<u64, String>;  // png decode: shell
```

If the PNG decode cannot be pure, the screenshot check lives in `amiga-conform` (shell) and
`amiga-record` stays pure; the worker decides and records it.

## Tests

- [ ] Every recording and bug zip in the repo's tests and under `bugs/` parses and re-serialises
      byte-identical (text and zip).
- [ ] sf-conform `recorder_*` and `widescreen_recording` tests and sf-e2e `recorder_*` tests pass.
- [ ] `Recorder` → `finish` → text → `parse` → `Replay` round-trips a synthetic game header in the
      toolkit, with no Superfrog type.
- [ ] `MAX_FRAMES` cut-off and `full()` behave as today.
- [ ] One `read_zip` remains in the engine and toolkit together.

## Done when

- [ ] `amiga-record` holds the generic half named nothing of the game; Superfrog's header is in sf-core.
- [ ] Old recordings and bug zips replay unchanged (the bug-zip replay `docs/BUG-FIX.md` uses works).
- [ ] Test block green; DESIGN §4's per-task runs green.
