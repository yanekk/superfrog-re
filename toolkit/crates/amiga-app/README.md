# amiga-app

The game-free parts of a desktop app that plays an Amiga game: sound and the sample clock that paces it, gamepads, key bindings, the picture on the GPU, capture files for end-to-end tests, atomic writes, the single-copy lock, `settings.toml`, and the shared panels, each an AppKit-free model plus a native macOS view. A shell crate: it reads devices, the clock and files, so a game's pure engine core must not depend on it. The run loop, game window, menus and the glue that opens the panels stay the game's. Off macOS the panels are stubs (the save window discards the recording).

## Start here

Nothing is re-exported at the crate root: every item is under its module (`amiga_app::audio::Audio`, `amiga_app::video::picture_rgba`). `Outcome` exists in both `controls_window` and `save_window`.

- Pacing: `let audio = Audio::open(None, "mygame")?; let mut pacer = Pacer::new(host_rate, 313);` then each loop `for _ in 0..pacer.due(audio.clock().samples()) { step(); audio.push(&samples); }`. After a modal panel call `pacer.skip_to(samples)` so the game does not catch up. On an `Err` from `Audio::open`, use `Clock::Wall { start, rate }`.
- Picture: `video.upload(size, &picture_rgba(&pic)); video.present();`.
- Settings and lock: `let _lock = lock::acquire(&dir, "My Game")?; let (s, msg) = settings::load(&dir);` and `settings::save(&dir, &s)` on change.
- Script: implement `ScriptGame` for your game's own lines and call `parse_script_core(text, &mut game)`.
- Capture: `Capture::new(dir, rate)?`, then per step `frame`, `audio`, `clock`, `input`, and `finish()`, `pacing(...)`, `memory()` at the end.

## API by job

Sound and time (`audio`, `pacing`)
- `Audio::open(name: Option<&str>, log_name: &str) -> Result<Audio, String>` (a name no device has makes audio unavailable, for tests), `clock() -> Clock`, `failed()`, `push(&self, samples: &[[i16; 2]])`.
- `Clock::{Device(..), Wall { start, rate }}`, `samples() -> u64` in host samples.
- `Pacer::new(host_rate: u32, frame_lines: u32)`, `set_frame_lines(lines)` (312 while the game keeps short frames), `due(&mut self, samples: u64) -> u32` (marks them stepped), `skip_to(samples)`.
- `Presenter<F>`: `completed(frame)`, `at_refresh() -> Option<&F>` (the newest whole frame, never a blend).

Picture (`video`)
- `Video::new(window: Arc<Window>, scale: Scale)`, `resized`, `upload(&mut self, size: (u32, u32), rgba: &[u8])`, `present()`, `shot(&self, path: &Path) -> Result<Shown, String>` (renders offscreen; works when covered), `points_scale()`; public `scale`, `fullscreen`, `filter`, `bar`, `bar_points`, `occluded`.
- `Scale::{Auto, N(u32)}`, `Filter::{None, Basic, Lottes, Geom, Easymode}` (`ALL`, `name`, `named`, `title`).
- `picture_rgba(pic: &Picture) -> Vec<u8>` (channel x 17), `picture_area`, `picture_scale`, `effective`, `viewport`: whole-number scaling in surface pixels (Retina is 2 a point).

Input (`keys`, `pad`)
- `OriginalInput::{Up, Down, Left, Right, Fire, Key(u8)}`: `name`, `named`, `label`, `apply(self, f: FrameInput) -> FrameInput`.
- `KeyMap(pub BTreeMap<OriginalInput, Vec<KeyCode>>)`, `targets(code)`; `AMIGA_KEYS`, `HOST_KEYS`, `key_named`, `key_name`, `key_label`.
- `PadMap`, `targets(p)`; `PadInput` (`name`, `label`, `named`, `is_button`), `PAD_INPUTS`, `STICK_THRESHOLD` (500 of 1000); `PadEvent::{Connected, Disconnected, Button, Stick}`, `StickAxis`; `PadState::apply(&mut self, e: PadEvent) -> Vec<PadInput>`, `held`, `connected`; `trait PadSource { fn next(&mut self) -> Option<PadEvent> }`; `Gilrs::open() -> Result<(Gilrs, Vec<PadEvent>), String>`; `from_gilrs`, `from_gilrs_button`.

Files (`atomic_write`, `lock`, `settings`, `capture`)
- `atomic_write::write(path: &Path, bytes: &[u8]) -> io::Result<()>`: temp file, fsync, rename.
- `lock::acquire(dir: &Path, app_name: &str) -> Result<Lock, String>` (`flock`; a dead holder's lock is free; `Err` is the message to show), `lock::path`.
- `Settings { scale, fullscreen, filter, widescreen, keys, pad }`, `to_toml`, `from_toml`; `settings::load(home) -> (Settings, Option<String>)` overwrites a damaged file with defaults, `settings::read` leaves it; `save`, `path`.
- `Capture`: `new`, `dir`, `frame(n, (w, h, rgba))`, `window_path`, `window_shown`, `audio(n, samples, ends)`, `clock`, `input`, `pacing(stepped, presented, dropped, clock)`, `memory`, `finish`. File names are the constants `AUDIO`, `AUDIO_FRAMES`, `INPUTS`, `WINDOWS`, `PROMPTS`, `PACING`, `CLOCK`, `MEMORY` and `frame_file(n)`, `window_file(n)`; test drivers read the same names. `png`, `wav`.

Panels
- Controls: `Controls::new()`, `action(a: ControlsAction, s: &mut impl ControlsHost) -> Outcome`, `key`, `pad`, `title`, `wants_pad`; `ControlsAction::named(words)` reads script lines; `trait ControlsHost` (implemented by `Settings`); `bind_key`, `bind_pad`, `keys_title`, `pad_title`, `JOYSTICK`, `Device`; `ControlsWindow::open(event_loop, active)`, `id`, `show`, `shown_status`, `next_click`.
- Start Level: the game describes fields in `StartLevelSpec { fields: Vec<LevelField> }` (`FieldKind::{Dropdown, Number { max, range }, Checkbox}`, `Value`); `Form::new(spec)`, `apply(a: FieldAction) -> Option<Option<Values>>`, `can_start`, `number`, `hint`; `FieldAction::named(spec, words)`; `Values::choice`/`text`/`on`; `whole_number`; `run_modal(app, spec, parent) -> Option<Values>`.
- Recording bar: `BAR_HEIGHT` (34 points), `elapsed(frames) -> String`, `place`; `RecordingBar::new(window)`, `set_shown`, `set_time`, `layout`, `next_click()`. Growing the window is the game's.
- Save window: `SaveForm::new(name, location: impl Display)`, `action(SaveAction) -> Outcome`; `save_window::run(app, parent, shot: &[u8], form, save: &mut dyn FnMut(&str) -> Result<(), String>) -> bool`, modal; `TITLE`.

Scripts (`script_core`): `parse_script_core<G: ScriptGame>(text, game) -> Result<CoreScript<G::Event>, ScriptError>`; `CoreScript { seed, fast_until, events }`; `trait ScriptGame { type Event; host; directive; event; finish }`; `HostInput::{Key, MouseLeft, FocusLost, Pad}`; `lines`, `Line`, `host_events`. The format is in the module doc.

## Where the details are

Each module's `//!` doc holds its file format or window layout (`src/script_core.rs`, `src/settings.rs`, `src/capture.rs`, the `*_window.rs` files). `toolkit/docs/architecture/frame-interrupts-input.md`.
