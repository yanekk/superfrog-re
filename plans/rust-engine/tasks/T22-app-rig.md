# T22 — app-rig

**Phase:** 3 · **Depends on:** T09, T10, T12, T13 · **Weight:** medium

## Goal

The first window and the rig that proves every later surface: a minimal `superfrog` binary that loads
the asset cache, runs `Game` at the PAL rate, presents frames at 1× integer scale and plays sound;
the seatbelt flags; and `sf-e2e`, which drives the real binary in both layers DESIGN §5 describes and
captures what it showed. Later app tasks add tests here instead of building infrastructure.

## Design sections this implements

DESIGN §2.5, §3.2 (`sf-app`, `sf-e2e`), §5 (End to end), §5.2.

## Files

- `engine/crates/sf-app/src/{main.rs,args.rs,video.rs,audio.rs,pacing.rs,home.rs,script.rs,capture.rs,input.rs}` — `input.rs` is the one point where winit events become `FrameInput` (a fixed default mapping here; T23 makes it remappable)
- `engine/crates/sf-e2e/src/{lib.rs,app.rs,os.rs}`, `engine/crates/sf-e2e/tests/smoke.rs`
- `engine/crates/sf-e2e/README.md` — how to run each layer, and the one-time permission grant.

## Interface

```
superfrog [--home DIR] [--cache-from-disks DIR] [--script FILE] [--capture DIR] [--max-frames N] [--timeout S] [--scale N] [--allow-fullscreen]
```

```rust
// sf-e2e
pub struct App;  impl App {
    pub fn launch(opts: LaunchOpts) -> App;                    // temp home, imports the real disks into it, always passes --max-frames/--timeout
    pub fn run_script(&mut self, script: &str) -> Captured;    // script layer
    pub fn os_key(&mut self, key: Key); pub fn os_menu(&mut self, path: &[&str]); pub fn os_capture(&self) -> Image;  // OS layer
}
pub struct Captured { pub frames: Vec<Image>, pub audio_wav: PathBuf, pub exit: ExitStatus }
```

Under `--script`, events enter where winit events are translated (`input.rs`), so everything
after that point is the real code path.

## End to end (the worker drives this)

- suite: `sf-e2e` (new) · sizes: 1× (320×256)
- [ ] launch with a temp home and the real disks → first captured frame equals `Game::render` of frame 0 (the black power-on frame) and frame 400 equals the engine's headless frame 400.
- [ ] captured audio for 500 frames equals `sf-paula` output for the same writes.
- [ ] `--max-frames 100` exits by itself with status 0; `--timeout 2` on a stuck run kills it and every child.
- [ ] OS layer: launch, post a real key event, capture the window: the window exists and shows the game area. If permissions are missing, the test fails with the System Settings path to grant them.

## Tests

- [ ] pacing: the pacer asked for 49.92 Hz over 10 simulated seconds of audio clock produces exactly 499 or 500 steps, and never interpolates.
- [ ] `--home` is honoured: nothing is written outside it (checked by listing the real app-data folder before and after).
- [ ] the e2e smoke cases above, in the test block.

## Done when

- [ ] Both layers have at least one green test; the script-layer smoke runs in the test block; the OS layer runs only under `SF_E2E_OS=1` (DESIGN §5) and was run green with it.
- [ ] The seatbelt flags behave as DESIGN §5.2 says, each with a test.
- [ ] The permission grant was obtained or is waiting on the owner in PROGRESS "Blocked on the user".

## Needs a person

```
System Settings → Privacy & Security → Accessibility, and → Screen & System Audio Recording:
enable the terminal app the build runs in (the worker names it).
```

Expect: the OS-layer smoke test then passes.
Tell me: that both are enabled, or that you'd rather not grant them (the OS-layer tests are then skipped and recorded as unproven in FINDINGS).
