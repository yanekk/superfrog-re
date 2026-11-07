# fd-e2e

End-to-end tests of the real `fd-app` (plan `forest-dumb-level-1`, DESIGN §5). The driver is the
toolkit's `amiga-e2e`; this crate gives it Forest Dumb's `AppSpec`, makes every launch read the owner's
disks, and holds every test under `tests/`. Two layers: the script layer (events fed through
`--script`, entering where the window's are translated) runs in the test sets; the OS layer (real key
events, menu clicks and window captures through the Accessibility API) needs Accessibility and Screen
Recording and runs only with `FD_E2E_OS=1`. It is not for engine behaviour: compare the engine with the
original in `fd-conform`; here the app is compared with a headless `Game`.

## Start here

1. Run them, from `games/forest-dumb/engine/`: `cargo test -p fd-e2e` (script layer), and the OS layer
   `FD_E2E_OS=1 cargo test --release -p fd-e2e --test drill_os -- --ignored --test-threads 1`.
2. A scripted run checked frame by frame against a headless game (`tests/smoke.rs`):
   ```rust
   let mut app = App::launch(LaunchOpts { max_frames: 240, ..Default::default() });
   let c = app.run_script_unread("20: down ArrowRight\n120: down Space\n130: up Space\n");
   assert!(c.exit.success(), "{}", c.stderr);
   let hashes = common::hashes(&c);           // hashes.txt, frame 0 first
   let fed = common::inputs(&c);              // inputs.txt as FrameInputs by step
   ```
   Then step `common::headless()` on `fed` and compare `common::hash(&g)` with `hashes[n]`.
3. Settings before the run: write `app.home.join("settings.toml")`, e.g. `"[keys]\n\"key Esc\" = [\"KeyE\"]\n"` (`tests/game_over.rs`).
4. Script lines and flags are `fd-app`'s: see `crates/fd-app/README.md`.

## API by job (`src/lib.rs`)

- `spec() -> &'static AppSpec`: `fd-app` with its flags, clearing `FOREST_DUMB_HOME`, keep switch `FD_E2E_KEEP`, scratch `fd-e2e`.
- `binary() -> &'static Path`: `fd-app` built once per test process. `conform_binary()`: `fd-conform`, likewise, for `fd-conform bug replay <zip>` (`tests/recorder_replay.rs`).
- `disks() -> PathBuf`: the zips' folder; panics naming where it looked, never skips.
- `App::launch(opts: LaunchOpts) -> App`: a temporary home, `--disks` prepended to `extra_args`. `App::launch_bare(opts)`: without `--disks` (missing-disk tests). Derefs to `amiga_e2e::App`; the home is removed on drop unless `FD_E2E_KEEP=1`.
- Re-exported from `amiga_e2e`: `AppSpec`, `Captured`, `Flags`, `Image`, `Key`, `LaunchOpts`, `Mods`, `Shot`, `group_alive`, `os_permissions`.

From `amiga_e2e::App` (through deref):
- `run_script(&str) -> Captured`: to the frame cap, every frame read. `run_script_unread`: frames left on disk, read with `c.read_frame(n)`; use it for long runs.
- `start_script(&str)` then `finish()` / `finish_unread()`: act while it runs. `kill()`, `pid()`, `home`.
- OS layer: `start_os()` (or `LaunchOpts { os: true, .. }`), `os_key(Key)`, `os_key_down`, `os_key_up`, `os_key_with(Mods, Key)`, `os_menu(&["Game", "Controls…"])`, `os_menu_items`, `os_press_button(title)`, `os_press_id(id)`, `os_close_window`, `os_window`, `os_windows`, `os_activate`.

`LaunchOpts` defaults: `max_frames: 100`, `timeout_s: 60`, `scale: Some(1)`, `os: false`, `extra_args: []`.
Every launch passes `--home`, `--max-frames` and `--timeout` and runs in its own process group.
`Captured` holds `frames` (empty when unread), `exit`, `inputs` (`inputs.txt` text), `stderr`,
`prompts`, `audio_wav`, `dir`; `read_shot(n)` gives a `shot`'s window image and its `Shot`.

## Test helpers (`tests/common/mod.rs`, `mod common;`)

`UP`, `DOWN`, `LEFT`, `RIGHT`, `FIRE` joystick bits; `inputs(c)` and `joy(c, from, to)`; `hashes(c)`;
`data()` (the game's data, once a process); `headless()` (a `Game` at `Start::Level1`); `hash(g)` and
`picture(g)` (as the capture holds them); `said(c)` (the Controls window's `fd-app: controls: ` lines).

## Tests

`smoke.rs` frames and hashes; `controls.rs` rebinding and settings; `disks.rs` missing disks;
`window.rs` scale, quitting, the watchdog, the lock; `recorder_app.rs`, `recorder_replay.rs` bug zips;
`game_over.rs`, `level_end.rs`; `drill.rs` (smoke) and `drill_sizes.rs` (full) the script-layer drill;
`drill_os.rs` the OS layer (full, writes window captures to `$TMPDIR/fd-drill-os`). Labels are in
`games/forest-dumb/smoke.toml`.

## Where the details are

- `toolkit/crates/amiga-e2e/src/app.rs` (launch, `Captured`) and `src/os.rs` (OS layer, `Key`).
- `toolkit/crates/amiga-app/src/capture.rs`: what each capture file holds.
- `plans/forest-dumb-level-1/DESIGN.md` §5, §5.2; `games/forest-dumb/CLAUDE.md` "Playing it".
