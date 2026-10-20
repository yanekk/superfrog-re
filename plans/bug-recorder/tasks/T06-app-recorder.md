# T06 — app-recorder

**Phase:** 2 · **Depends on:** T02 · **Weight:** medium

## Goal

Put the recorder in the app behind menu commands and script events, with no new windows yet: Start
Level with a setup, recording every step across levels and game over, the rules for what ends a
recording, and the zip written to the bugs folder. The window tasks after it each add a view over
this.

## Design sections this implements

DESIGN §2.2, §2.4, §2.5, §2.11, §5.2 (`--bugs-dir`).

## Files

- `engine/crates/sf-app/src/recorder.rs` (new): state machine, bugs dir, zip writing
- `engine/crates/sf-app/src/menu.rs`: `Command::StartLevel`, `Command::StopRecording` (items, ⌘.), the ending rule in `on_command`
- `engine/crates/sf-app/src/main.rs`: `Run` holds the recorder; `step` appends the input
- `engine/crates/sf-app/src/import.rs`: `Ui` gains `start_level`, `recording_in_progress`, `save_recording`, with scripted answers
- `engine/crates/sf-app/src/script.rs`, `args.rs`: events and `--bugs-dir`
- `.gitignore`: `bugs/`; `tools/test_no_bug_zips.py` (new)
- `engine/crates/sf-e2e/tests/recorder_app.rs` (new)

## Interface

```rust
pub enum Recorder { Off, Recording { rec: Recording } }
impl Recorder {
    pub fn start(setup: LevelSetup, seed: Rng, build: &str) -> Recorder;
    pub fn step(&mut self, input: FrameInput);
    pub fn finish(self, at: Location, note: String) -> Option<Recording>;
}
pub fn bugs_dir(arg: Option<&Path>) -> PathBuf;           // DESIGN §2.4
pub fn zip_name(dir: &Path, at_level: LevelId, now: (i32,u8,u8,u8,u8)) -> PathBuf;
pub fn write_zip(path: &Path, rec: &Recording, png: &[u8]) -> io::Result<()>; // atomic_write

pub enum EndChoice { Save, Discard, Cancel }
trait Ui {
    fn start_level(&mut self) -> Option<(LevelSetup, bool /* record */)>;
    fn recording_in_progress(&mut self, action: &str) -> EndChoice;
    fn save_recording(&mut self, shot: &[u8], at: &Location, name: &str) -> Option<String>; // the note, None = discard
}
```

Script events (format in `script.rs`'s doc): `start-level W1-2 lives=7 points=1000 energy=3
difficulty=hard spud wings record`, `answer-recording save|discard|cancel`, `note <text>|discard`,
and `menu stop-recording`. The native `Ui` methods are stubs until T08 and T10; they are never reached
without a script before then because the menu items call through the same `Ui`.

The seed is the host clock's (`host_seed`) under normal play and the script's `seed` under a script,
as power-on already does. The screenshot is the core's last `Picture` at 1×, encoded by
`capture::png`. After saving: a `notice` with the path; the path on the clipboard (NSPasteboard).

## Tests

- [ ] e2e: `start-level W1-1 record`, 300 frames of a script, `menu stop-recording`, `note x`: one zip in `--bugs-dir` whose recording parses, has the setup, 300 inputs equal to the capture's `inputs.txt`, note `x`, and `screenshot.png` equals the captured frame 300 at 1×
- [ ] e2e: a recording through W1-1's exit, the tally and into the slot machine keeps going (inputs count equals frames stepped)
- [ ] e2e: a recording through a game over (lives 3, a script that dies) keeps going past the GAME OVER screen
- [ ] e2e: each ending action (load-state, start-level, reset, continue, import, play-level-file, quit) with `answer-recording cancel` keeps recording; with `discard` the action happens and no zip; with `save` a zip then the action
- [ ] e2e: pause frames are not recorded; save-state, scale, crt do not end the recording
- [ ] unit: `zip_name` with clashes; `bugs_dir` for a checkout, a worktree build (common dir's parent) and no checkout
- [ ] unit: write failure (read-only dir) returns the error; the recording is still held
- [ ] `tools/test_no_bug_zips.py`: fails when a tracked file is under `bugs/` or ends in `.zip`

## Done when

- [ ] Game ▸ Start Level… and Game ▸ Stop Recording ⌘. exist and work through script events, with the tests above
- [ ] no test writes outside its temp `--home`/`--bugs-dir`
- [ ] the test command is green
