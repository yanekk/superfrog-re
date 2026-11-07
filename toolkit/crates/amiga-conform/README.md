# amiga-conform

The game-free half of a conformance harness: it judges an engine's picture and sound against the original's run on `amiga-oracle`, within the owner's tolerances, and writes the diff picture beside a divergence. Test tooling (a shell crate: it reads and writes files); a shipped app never links it, since it pulls in the emulator. A game's conformance crate keeps its cases, memory map, levels, milestones and the run loop that drives both sides, and passes in what these read (excluded rows, file headers, an output folder).

## Start here

Judge a case's pictures (from `tests/faithful.rs`):

```rust
let mut judge = PictureJudge::new(Tolerance::OWNER, frames);
for f in 0..frames {
    judge.frame(f, classify(&ours /* amiga_chips::Picture */, &theirs /* Rgb12Frame */, &[], &Tolerance::OWNER))?;
}
```

Match sound as events (from `tests/faithful.rs`):

```rust
let (mut pt, mut po) = (Paula::default(), Paula::default());
let mut m = SoundMatch::new(Tolerance::OWNER.sound_frames);
m.frame(f, &pt.frame(&their_writes), &po.frame(&our_writes))?;   // per frame
```

Reach a sync point once and reuse it: `sync_cache::at_snapshot(root, magic, key, power_on, get_there)` with `key = sync_cache::cache_key(format, kickstart, disks, drives, what)`. On a cache miss it calls `get_there` and stores the snapshot; `power_on` sets the frame cap, which a restore keeps, so leave room for the frames after.

`Tolerance::OWNER` is the faithful bar. A case cannot loosen it; a task that finds it wrong raises it with the owner.

## API by job

Picture (`compare`, `report`, `screenshot`)
- `Mode::{Faithful, Strict}`; `Tolerance { colour_step, few_pixels, glitch_frames, glitch_budget_pct, sound_frames }`, `Tolerance::OWNER` (also `Default`).
- `classify(ours: &Picture, theirs: &Rgb12Frame, rows: &[Range<u16>], tol: &Tolerance) -> PictureClass` (`Same`, `ColourStep(n)`, `FewPixels(n)`, `Glitch(n)`; a different display window is a glitch).
- `PictureJudge::new(tol: Tolerance, frames: u32)`, `frame(&mut self, f: u32, class: PictureClass) -> Result<(), String>`; public `colour_step`, `few_pixels`, `glitches` frame lists.
- `picture(ours, theirs) -> Option<PixelMismatch>`, `picture_from(ours, theirs, from: u16)`, `picture_except(ours, theirs, rows)`; `race_rows(races: &[(u16, u16)], top: u16) -> Vec<Range<u16>>` gives the rows to leave out for beam races.
- `report::write_diff_png(dir, case, frame, ours, theirs, mismatch) -> PictureDiff` writes `<dir>/<case>-frame<N>.png`; `report::png`, `report::crc32`.
- `screenshot::picture_rgb`, `picture_png(pic)`, `frame_png(f: &Rgb12Frame)`, `screenshot_differs(png: &[u8], pic: &Picture) -> Result<u64, String>`, `decode_png`.

State by pass (`compare`)
- `PassMatch` (`Default`): `frame(&mut self, f: u32, theirs: u32, ours: u32) -> PassStep` says when to compare or hold (`compare_held`, `compare_now`, `hold: Option<Side>`, `apart`); `drop_held()`, `level()` after a re-sync; public `skews`.

Sound
- `paula(ours: &[PaulaWrite], theirs: &[PaulaWrite]) -> Option<String>`: strict first difference; beam positions ignored. `reg_name(reg: u16) -> String`.
- `sound_events(writes: &[PaulaWrite]) -> Vec<SoundEvent>` (`Start`, `Period`, `Volume`, `Stop`; `ch()`); `sound_events::Paula::frame(&mut self, writes) -> Vec<SoundEvent>` keeps register state across frames; `played_volume(value: u16) -> u8`.
- `SoundMatch::new(frames: u32)`, `frame(&mut self, f, theirs: &[SoundEvent], ours: &[SoundEvent]) -> Result<(), String>`, `cut_off()`; public `shifts`, `matched`, `trailing`.
- `channels`: `level(dac: &[i16]) -> u16`, `mixer_levels(p: &amiga_paula::Paula) -> Levels`, `Original::step(&mut self, c: &FrameCapture, frames: u32, lines: u32) -> Levels` (call it on every step, or the DAC goes stale), `line`, `parse(text, header)`, `judge(theirs: &[Option<Levels>], ours: &[Levels], tol: Tolerance) -> Verdict` (`failed() -> Option<Run>`), `channels::OWNER` (its own `Tolerance` type).
- `feed::swing(mix: &[[f32; 2]]) -> u16`, `feed::parse_audio(text, header)`.

Scripts and bugs
- `script::parse(text) -> Result<InputScript, ScriptError>`, `parse_with(text, game_token)`, `parse_with_forms(text, forms, game_token)`: `game_token` sees each token first and returns `None` for one it does not own. `InputScript { lines }`, `idle()`, `input_at`, `from_frame`, `from_steps`, `to_text`, `format_input`. This is not `amiga_oracle::InputScript`; convert with `lines`.
- `bug_compare::verdict_line(first: Option<(u32, u32, &str)>, left: Option<&Left>) -> String`.

Sync cache (`sync_cache`): `cache_key`, `key_text`, `hex`, `cache_path(root, key)` (`.sync2` files), `at_snapshot`, `encode`, `decode`. `at_snapshot` takes any `A: AsMut<Amiga>`, so a game's wrapper works.

## Where the details are

`src/compare.rs`, `src/sound_events.rs`, `src/channels.rs`, `src/sync_cache.rs`, `src/script.rs`; `tests/faithful.rs` for every tolerance at work. `toolkit/docs/architecture/verification.md` (the checks), `sound-chip.md`, `knowledge/self-testing-faithful-bar.md` (the bar), `toolkit/PLAYBOOK.md`.
