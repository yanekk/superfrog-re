# fd-app

The macOS app the owner plays (plan `forest-dumb-level-1`, DESIGN §2.7): it reads the two zips, opens
straight in W1-1 at the level sync point, silent, and steps fd-core's `Game` once a 313-line PAL frame
on the wall clock. It holds the menu bar, Game ▸ Controls…, Game ▸ Start Level… and the bug recorder.
Its game-free parts (pacing, pads, video, capture, lock, keys, settings, windows, a script's
game-free lines) are `toolkit/crates/amiga-app`. It never links an emulator or test tooling (`tests/deps.rs`). Tests drive it
through `fd-e2e`, not by calling its code; the library exports only the state hash.

## Start here

From `games/forest-dumb/engine/`:

1. Play: `cargo run --release -p fd-app`. Esc or ⌘Q quits.
2. A bounded scripted run with a capture:
   `cargo run -p fd-app -- --home /tmp/h --script run.txt --capture /tmp/cap --max-frames 240 --bugs-dir /tmp/bugs`
3. From a test: `fd_e2e::App::launch` then `run_script(...)` (`crates/fd-e2e/tests/smoke.rs`); compare
   `hashes.txt` with `fd_app::state_hash` of a headless `Game`.

## Flags (`src/args.rs`)

`fd-app [--home DIR] [--disks DIR] [--script FILE] [--capture DIR] [--max-frames N] [--timeout S] [--scale N] [--bugs-dir DIR]`

- `--home DIR`: app-data folder; else `FOREST_DUMB_HOME`, else `~/Library/Application Support/Forest-Dumb-RE`. One copy runs per home (a second is refused, "already running").
- `--disks DIR`: the folder of both zips; else `fd_tools::disk_dir()` (`FOREST_DUMB_DISKS`, then the main checkout). Missing disks: a plain message and exit 1.
- `--script FILE`: events in place of the keyboard; makes the run unattended (no real pad, no native message, does not take focus). A script that does not parse is exit 2.
- `--capture DIR`: frames, inputs and `hashes.txt` (below).
- `--max-frames N`: exit 0 after N steps.
- `--timeout S`: stop the whole process group after S seconds; defaults to 120 (`SCRIPT_TIMEOUT_S`) under `--script`. Exit 124 if the group cannot be stopped whole.
- `--scale N`: 1 to 16, for this run only.
- `--bugs-dir DIR`: where bug zips go; else `<home>/bugs` for a scripted run, else `games/forest-dumb/bugs/` of the main checkout, else `~/Documents/Forest Dumb Bugs/`. Zips are `bug-<level>-<YYYY-MM-DD-HHMM>.zip`, `-2`, `-3` on a clash.

Bad arguments: exit 2 with the usage.

## Script format (`src/script.rs`)

One directive a line, `#` comments. `N:` is the frame an event lands before: step N (from 1) sees it.
Game-free lines (`amiga_app::script_core`):

```
seed 1 2                  fast-until 11183
10: down ArrowLeft        20: up ArrowLeft        30: press Space   (down, up before the next frame)
40: mouse down            40: mouse up            45: blur
100: pad connect 1        101: pad down 1 South   (also up, press)
102: pad stick 1 0.8 -0.3 103: pad disconnect 1
```

This game's lines:

- `N: menu <id>`: `start-level`, `stop-recording`, `controls`, `quit` (`menu::Command::id`).
- `N: controls <input> keys|pad`, `controls defaults`, `controls close`: a click in the Controls window (`up keys` starts rebinding Up; the next key pressed binds it).
- `N: shot`: the window as presented, into the capture as `window-NNNNN.png`.
- `N: resize W H`: the inner size in points.
- `N: hang`: stop answering, the watchdog's test.
- `N: cheat`: the title's cheat code on (`Game::set_cheat`); HELP then ends the level.

Answers to the recorder's windows hold no frame and are used in order whenever a window shows; with none
left a window is cancelled, or the save window discarded:

- `form level W1-1`, `form record on|off`, then `form start` or `form cancel` (every `form` run must close).
- `answer-recording save|discard|cancel`: "Recording in progress".
- `note <text>`: the save window's note and Save; `note` alone an empty note; `note discard` Discard.

## Keys and settings (`src/settings.rs`)

`<home>/settings.toml` is the toolkit's format: `scale = "auto"` or 1 to 16, `fullscreen`, `filter`,
`widescreen`, then `[keys]` and `[pad]` mapping an original input to a list of host names. Inputs are
`up`, `down`, `left`, `right`, `fire`, or `key <label>` / `key $<hex>` for an Amiga key (tests use
`"key Esc" = ["KeyE"]` and `"key $5f" = ["KeyH"]`, HELP). Host key names are winit's `KeyCode` names:
`ArrowLeft`, `Space`, `AltLeft`, `KeyW`, `Digit1`, `F1` (`amiga_app::keys::HOST_KEYS`); pad names are in
`amiga_app::pad`. This game's defaults: Up on `Space`, Fire on `AltLeft` (Left Option), the arrows for
the rest; an input the file leaves out takes these. A damaged file gives the defaults, a line starting
`Your settings could not be read`, and is left untouched until a setting changes. Esc always quits and
never reaches the game; ⌘ plus a key is the app's.

## Capture and log

`--capture` writes the files `amiga_app::capture` lists (`frame-NNNNN.png`, `inputs.txt`,
`window-NNNNN.png`, `prompts.txt`, …) plus `hashes.txt`: `N hash` in hex a frame, frame 0 the start.
stderr lines start `fd-app: ` (`playing W1-1`, `menu <id> after step N`, `controls: <status>`,
`game over`, `level 1 complete`, `quit after step N`).

## Library API (`src/lib.rs`)

`HASHES` (`"hashes.txt"`); `state_hash(g: &Game) -> u64` (scene, state, objects, collision plane;
stable within one build only); `hash_line(n: u64, g: &Game) -> String`.

## Where the details are

- `src/main.rs` loop and pacing; `args.rs`; `script.rs`; `menu.rs`; `input.rs` host input to `FrameInput`; `settings.rs`, `controls.rs`; `recorder.rs` bug zips; `disks.rs`, `home.rs`.
- `toolkit/crates/amiga-app/src/` (`script_core.rs`, `keys.rs`, `settings.rs`, `capture.rs`).
- `plans/forest-dumb-level-1/DESIGN.md` §2.7, §2.8, §5.2; `games/forest-dumb/CLAUDE.md` "Playing it"; `games/forest-dumb/docs/BUG-FIX.md`, `frame-length.md`.
