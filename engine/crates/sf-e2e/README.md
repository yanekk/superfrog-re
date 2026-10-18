# sf-e2e — the real app, driven end to end

`sf-e2e` launches the real `superfrog` binary (building it first, once per test process) and
checks what it showed, played and saw (DESIGN §5, End to end). Every launch gets a temporary
`--home` with the owner's disks imported into it, and always passes `--max-frames` and `--timeout`.
The app runs in its own process group, so a stuck run is killed whole.

The disks are found as for every other test: `SUPERFROG_DISKS`, else the main checkout root. The
imported cache is kept in `~/.cache/superfrog-re/e2e/` between runs (it is derived from the disks,
so it stays outside the repository) and copied into each temporary home.

## The script layer (in the test block)

```
cd engine && cargo test -q -p sf-e2e
```

`App::run_script` runs the app with `--script FILE --capture DIR`. Script events enter where winit's
are translated (`sf-app/src/input.rs`), so the key mapping and the game loop are the real ones.
The script format is documented in `sf-app/src/script.rs`. The capture folder holds
`frame-NNNNN.png` (frame 0 is the start, frame N the picture after step N), `audio.wav` (the
mixer's output, 16-bit stereo) and `inputs.txt` (the input each step saw). A scripted run opens
an inactive window, plays nothing through the speakers and runs at the PAL rate, so 500 frames
take 10 s. Set `SF_E2E_KEEP=1` to keep the temporary homes and their captures.

T23's files: `import.rs` (the first-run import and the launch decision), `window.rs` (integer
scaling, resize, fullscreen) and `persist.rs` (settings, progress, the lock, no audio output, the
keys moving the frog). Their scripts also use:

- `pick …` / `answer …`: the disk picker's and the missing-disks message's answers. A scripted
  run never opens a native panel; every question it was asked is in the capture's `prompts.txt`
  (`App::launch_empty` starts from an empty home, `App::disks_in_home` copies the disks in).
- `N: resize W H`, `N: scale N|auto`, `N: shot`: a shot is the window's picture as presented,
  drawn offscreen by the same pass (macOS withholds redraws from a covered window), written to
  `window-NNNNN.png` with a line in `windows.txt`.
- `--audio-device NAME`: a name no device has is the "no audio output" hook.

T24's `menus.rs` drives the menu bar (`sf-app/src/menu.rs`). A script names a menu command with
`N: menu <id>` (`import`, `continue`, `scale 1|2|3|auto`, `fullscreen`, `pause`, `reset`, `quit`);
the app logs each command it takes as `superfrog: menu <id> after step N`, which lines an OS-layer
click up with the captured frames. A paused step still counts towards `--max-frames` and captures
the held picture again.

Fullscreen takes over the display, so the two fullscreen tests (`window.rs`, `persist.rs`) are
`#[ignore]`d like the OS layer and run under `SF_E2E_OS=1`.

## The OS layer (only when asked)

```
cd engine && SF_E2E_OS=1 cargo test -q -p sf-e2e -- --ignored --test-threads 1
```

These tests post real key events to the app with CGEvent, click its menu items through the
Accessibility API and capture its window. They bring a Superfrog window to the front, so they
are `#[ignore]`d in the test block (reported as ignored, never as passed) and refuse to run
without `SF_E2E_OS=1`.

### One-time permission grant

In **System Settings → Privacy & Security**, enable the terminal app the tests run in (here,
WezTerm) under both:

- **Accessibility**: posting key events and clicking menu items;
- **Screen & System Audio Recording**: capturing the window.

A missing grant fails the test with the setting to change. A capture that comes back blank
(every pixel transparent) also means Screen Recording is missing, even when macOS's own preflight
check says it is granted.

**The screen must be unlocked.** With it locked the menu bar still answers, but the
Accessibility API lists none of the app's windows (so no panel's buttons), window captures come
back blank and fullscreen never happens. `menus.rs` therefore counts windows and reads their
bounds from the window server, and cancels the picker with ⌘. rather than its Cancel button.

Two macOS 26 behaviours the layer works around:

- `screencapture -l` is refused from the terminal ("could not create image from window"), so the
  window is captured with `CGWindowListCreateImage` through `osascript -l JavaScript`.
- System Events needs a separate Automation grant and timed out waiting for it, so menus are
  clicked through the Accessibility API directly (what System Events' UI scripting uses).
