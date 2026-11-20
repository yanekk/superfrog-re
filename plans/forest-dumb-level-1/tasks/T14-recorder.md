# T14 — recorder

**Phase:** 3 · **Depends on:** T12, T13 · **Weight:** medium

## Goal
Start Level with a one-entry selector and the recorder box, Stop with a note, one zip that replays headless to the same result.
## Design sections this implements
§2.7, §2.8.
## Reuses
`amiga-record` (`GameRecording`, `Replayable`, zip), T12's windows, `sf-app/src/recorder.rs`, `sf-core/src/recording.rs:343`, `sf-conform/src/bug.rs` replay as pattern.
## Files
`fd-core/src/recording.rs`, `fd-core/Cargo.toml`, `fd-conform/tests/purity.rs` (`amiga-record`, `amiga-disk` on `ALLOWED`), `fd-app/src/{recorder.rs,menu.rs}`, `fd-conform/src/bug.rs`, `fd-conform/src/main.rs`, `fd-e2e/tests/{recorder_app.rs,recorder_replay.rs}`, `smoke.toml`.
## Interface
```
impl GameRecording for FdRecording { .. }   impl Replayable for Game { .. }
fd-conform bug replay <zip>    // prints `screenshot equal|differs`
```
## Tests
- [ ] Round trip of format; version and build mismatch named.
- [ ] Empty recording; Start Level and Quit during a recording ask Save, Discard or Cancel.
- [ ] Replay reaches the same final picture as its screenshot.
## End to end (the worker drives this)
- suite: `fd-e2e` · sizes: default, scale 1
- [ ] Menu → Start Level → selector shows level 1 only, recorder on → OK restarts the level, recording bar shows.
- [ ] Stop → note → save → one zip in `--bugs-dir`.
- [ ] Recorder off → no bar, no zip.
- [ ] Long note, empty note, cancel the save window.
## Done when
- [ ] A saved recording replays to the same result, tested; smoke holds one with an `[[area]]`.
- [ ] Zip location git-ignored.
