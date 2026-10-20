---
setup:
  - python3 -m venv venv && venv/bin/pip install -q --disable-pip-version-check -r requirements.txt
  - [ ! -x tools/setup/oracle.sh ] || tools/setup/oracle.sh
test:
  - NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py' || [ $? -eq 5 ]
  - cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-core -p sf-disk -p sf-app
  - cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-conform --lib --test purity --test script
  - [ -z "$(ls engine/crates/sf-conform/tests/recorder_*.rs 2>/dev/null)" ] || (cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-conform --test 'recorder_*')
  - cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-e2e --test window --test menus
  - [ -z "$(ls engine/crates/sf-e2e/tests/recorder_*.rs 2>/dev/null)" ] || (cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-e2e --test 'recorder_*')
---

# Bug recorder — Design

This plan builds on the finished `rust-engine` plan. Its `DESIGN.md` is sealed and still binds
everything here that it governs: what faithful means (§2.1), the copyright boundary (§2.3), the menu
bar as the home of every feature (§2.7), the purity boundary (§3.1), the app's seatbelts (§5.2).
Cited below as `RE §n`.

## 1. Purpose

The owner plays the Rust engine (`sf-app`), sees something wrong, and hands it to an agent without
describing it in words or testing the fix. The owner starts a level from a dialog with the recorder
on, plays, presses STOP, types a note, and gets one zip. An agent replays it headlessly, plays the
same start and inputs on the original (the vAmiga oracle), decides whether the behaviour is the
original's (kept, RE §2.1) or ours, fixes ours, and leaves a conformance case that fails before the
fix and passes after.

### Success criteria

- A recording made in the app replays headlessly to the same final picture as its screenshot.
- The same recording played on the original from the same start stays in step through a level's
  end, the tally, the slot machine and the next level's load (T00 proves, T05 keeps proving).
- `@repo/single fix the bug in bugs/<zip>` ends in either a fix plus a new case, or a plain report
  with both pictures saying the original does the same, by following `docs/BUG-FIX.md`.
- The acceptance bug (black screen instead of W1-2's intro after W1-1's slot machine, owner
  2026-10-20) has gone through that whole flow (T13).

### Stance

- Recording starts only at a level start chosen in the dialog. A start the oracle can reproduce is
  what makes a recording comparable; "record from anywhere" would need a mid-game transplant into
  the original, which nothing supports.
- The recording is inputs, not snapshots: a start plus per-frame inputs is a few KB, readable as
  text, and already almost a conformance case.
- The owner never re-tests a fix. The agent's evidence is the replay and the oracle.

---

## 2. Behaviour specification

### 2.1 The Start Level dialog

Game ▸ Start Level… opens a native window (the Controls window, `controls_window.rs`, is the model:
RE §2.7, "nothing new drawn in-game").

| Field | Control | Values | Default | Sets |
|---|---|---|---|---|
| Level | dropdown | the 29 levels a game plays (`level_entry.rs`'s `ids()`; W6-1's unused copy is left out) | W1-1 | which level |
| Lives | dropdown | 3, 5, 7 | 5 | `opt_lives` 0/1/2 as the options screen sets it; `frog_new_game` stores 2/4/6 spare lives |
| Points | number field | 0–999 999 | 0 | `score` (h0+$834) |
| Energy | dropdown | 1–4 hearts | 4 | `frog_hearts` (h0+$830) |
| Difficulty | dropdown | Easy, Normal, Hard | Normal | `opt_game_mode` 0/1/2 (h0+$9e6) |
| Spud | checkbox | | off | `frog_spud_on` |
| Wings | checkbox | | off | `frog_flight` |
| Enable recorder? | checkbox | | on | whether a recording starts |

Lives are the options screen's three choices because the owner wants them exactly as the game's own
menu offers them (2026-10-20). Points are a field because a dropdown of a million values is not one;
the score shows six digits, hence the limit. `frog_next_life` (h0+$880) is set to the next multiple of
200 000 above the points, as a real game would have it, so an extra life comes when it would have.
The dialog always opens on these defaults (owner chose new-game values, 2026-10-20).

Start begins the level fresh with exactly these values; the game is otherwise normal. Lives and
difficulty are options-screen values that `frog_new_game` reads before the level loads, so they are
set before it, not poked after; the other four are set at the level's first frame. How the engine
gets there is T00's to decide (§7, D2).

### 2.2 Recording

- A recording starts on the level's first frame when Start is pressed with the recorder on. It holds
  the start (§2.6) and every stepped frame's `FrameInput`, in memory.
- It runs until STOP: across the level's end, the tally, the slot machine, later levels and game
  over. Game over does not end it, because a bug may come after the GAME OVER screen (owner,
  2026-10-20).
- Pause stops stepping, so paused frames are not frames of the recording. Scale, fullscreen, CRT,
  widescreen and Save State leave it running: they change nothing the game reads.
- These menu actions would change the game and so end it: Load State, Start Level, Reset, Continue,
  Import Disks, Play Level File, Quit (menu, ⌘Q or window close). Each first shows "Recording in
  progress" with Save…, Discard and Cancel (owner, 2026-10-20). Save… opens the save window (§2.4),
  with the screenshot taken now; after Save or Discard there, the action goes ahead. Discard drops
  the recording and the action goes ahead. Cancel returns to play with the recording still running.
- The game is paused while the question or the save window is open, so the screenshot and the last
  recorded frame are the same moment.

### 2.3 The recording bar and STOP

- While recording, a bar sits under the game picture, never over it, so it never covers play or the
  screenshot (owner, 2026-10-20). It shows a recording indicator, the elapsed time and a STOP button.
- The window grows by the bar's height while recording and shrinks back after; the game picture's
  size and scale do not change. In fullscreen the picture's integer scale is chosen to leave room for
  the bar under it.
- ⌘. and Game ▸ Stop Recording do what STOP does (owner chose ⌘., 2026-10-20). The menu item is
  disabled when not recording.
- The bar is native AppKit views in the window (as the Controls window builds its views), not drawn
  by wgpu, per RE §2.7.

### 2.4 Saving

- STOP pauses the game and opens the save window: the screenshot, the location, the zip's name, a
  one-line note (may be empty), Discard and Save.
- Save writes one zip into the repo's `bugs/` folder and shows its path in a notice, copied to the
  clipboard, so the owner pastes it to the agent. Discard drops it. Either way the game resumes, now
  not recording.
- `bugs/` is the main checkout's (the parent of the git common dir found from the build's
  `CARGO_MANIFEST_DIR`), so a build in any worktree saves in one place. When that is not a git
  checkout the folder is `~/Documents/Superfrog Bugs/`. `--bugs-dir DIR` overrides both; every test
  passes a temp dir so no test writes into the repo. The owner chose a git-ignored repo folder
  (2026-10-20).
- Name: `bug-<level-id>-<YYYY-MM-DD-HHMM>.zip`, the level being where STOP was pressed (or the
  start level when not in one), with `-2`, `-3` appended rather than overwrite.

### 2.5 The bug zip

One zip, stored entries (no compression; the PNG is already compressed, the text is small):

| Entry | Content |
|---|---|
| `recording.txt` | §2.6 |
| `screenshot.png` | the game picture of the last recorded frame, at 1× (the `Picture` the core rendered, not the window), so a headless replay can compare it exactly |

The location and note are in `recording.txt`'s header, so the text alone is a complete report.
The zip is never committed (RE §2.3: a picture rendered from the disks is disk-derived). `*.zip` is
already git-ignored; `bugs/` is added; a Python test fails if any tracked file is under `bugs/` or is a
zip. The agent commits only what it derives: a case and a script, which are inputs and facts.

### 2.6 The recording format

Text, one header block then the inputs, readable by a person and parsed by `sf_core::recording`
(pure, so the app and sf-conform share one parser):

```
superfrog recording 1
build <engine build string>
level W1-1
lives 5
points 0
energy 4
difficulty normal
spud off
wings off
seed <lo-hex> <hi-hex>
frames 18342
stopped-at W1-2:94,51          # or a screen name: tally, slot machine, world map, intro, title, game over
note black screen after the slot machine
inputs
0: -
112: R
130: R F
...
```

The input lines use the conformance script's tokens and its "a line holds until the next" rule
(`sf-conform/src/script.rs`), with frames counted from 0 at the level's first frame, one per engine
step. The token formatting moves from sf-conform into `sf_core::input` so both sides use one copy
(T01). `seed` is whatever T00 picks as the start's random state (§7, D2). `build` is informational:
a replay on a newer engine is the point of a fix, so a different build is never refused.

### 2.7 Replay

- `superfrog --replay <zip>` plays the recording in the window, watch-only: the game's keys do
  nothing, Pause works, it holds on the last frame and prints `superfrog: replay finished at frame N`
  on stderr. It exists for agents and tests, not people (owner, 2026-10-20), so it has no further
  controls. It refuses a zip it cannot parse with a message naming why.
- `sf-conform bug replay <zip>` replays headlessly (no window, no audio device), writes chosen frames
  as PNGs to a scratch folder outside the repo, and checks the final picture equals `screenshot.png`
  and the final location equals `stopped-at`. A mismatch is reported, not fatal: on a fixed engine it
  is expected.

### 2.8 The comparison with the original

- `sf-conform bug compare <zip>` starts the original at the same level with the same values
  (`SyncPoint::Setup`, T03), retimes the inputs from engine steps to the original's frames (§2.9), and
  runs the faithful checks (RE §2.1) over the whole recording. Same: it says so and saves the
  original's picture at the STOP frame beside the zip. Different: it names the first differing
  frame and variable, with `--strict` for the frame-exact view.
- `sf-conform bug case <zip> <name>` writes `engine/conform/cases/<name>.toml` and
  `engine/conform/scripts/<name>.txt`: the permanent case.

### 2.9 Timing: engine steps against the original's frames

Normal play never lags (RE §2.1, `Pacing::OnTime`); the original does (mid-pass runs up to 390 frames
on W4-3). An input recorded at engine step n was read by the engine's pass k. Fed to the original at
frame n it lands in a different pass and the run drifts. The rule the plan expects, and T00 must
prove: feed the original each input at the frame its pass k reads it, found by the existing pass
detection (`sync.rs` `in_level_wait`, `PassClock`), and run the engine side paced as cases already do.
If T00 shows that is not enough (the original's lag also slows objects relative to the frog, T84
review), the fallback is comparing in legs re-synced from the original (`route_oracle`'s chunks),
and §2.8's verdict becomes per leg.

### 2.10 The fix procedure

`docs/BUG-FIX.md`, pointed at from `CLAUDE.md` so a fresh `@repo/single` session finds it. In order:

1. `sf-conform bug replay`: confirm the replay ends on the reported picture; look at the frames
   before STOP to see the problem.
2. `sf-conform bug compare`: the original from the same start with the same inputs.
3. Same on both: a quirk of the original, kept (RE §2.1). Report in plain words what happens and
   when, with both pictures saved beside the zip; commit nothing (owner chose, 2026-10-20).
4. Different: `--strict` names the first differing frame and variable; fix the engine.
5. `sf-conform bug case`: the recording becomes a case; show it fails on the commit before the fix
   and passes after; add it to `tests/cases.rs` (or `#[ignore = "slow: …"]` with its reason if long).
6. Fresh review; the fix commit carries an `Escape:` trailer naming what it escaped (`Escape:
   rust-engine T{nn}` or `Escape: unknown`).
7. A difference the case cannot show (the replay shows it, the paced comparison does not) is reported
   to the owner as such, not forced into a case.

### 2.11 The unhappy paths

- A zip that is not a bug zip, a missing entry, a header field out of range, a frame number not
  increasing: refused by the parser with the line and why; never a panic.
- Writing the zip fails (disk full, folder not writable): a notice saying so, and the save window
  stays open so the note is not lost; the recording is kept until saved or discarded.
- The app crashes or is killed while recording: the recording is lost. It is memory only, and a
  crash is itself a bug the owner reports by hand; writing every frame to disk is not worth its cost.
- Two app copies cannot run at once (RE's `lock.rs`), so two recordings never race for a name.
- A recording on a level the dialog offers but the oracle cannot reach the same way: T03 proves
  every one is reachable; one that is not is a T03 failure, not a silent skip.
- Easy and hard have no conformance cases yet (the difficulty plan's). A difference on them may be
  the first time anyone looked; the procedure says so in the report.

---

## 3. Architecture

### 3.1 The boundary

RE §3.1 holds unchanged: `sf-core`, `sf-assets`, `sf-disk` are pure (no clock, fs, network, float;
`sf-conform/tests/purity.rs` enforces it); `sf-app` and `sf-conform` are shells. New pure code:
`sf_core::recording` (format, `LevelSetup`, retiming rule as a function of pass data), `sf_core`'s
level start with a setup, `Game::location`, and the zip writer in `sf_disk::zip`. If the purity test
fails, move the code; never relax the test.

### 3.2 Modules

- `sf_core::recording` (new): `LevelSetup`, `Difficulty`, `Recording`, `Location`; text format.
- `sf_core::input`: gains the token format moved from sf-conform.
- `sf_core::game`: `Game::start_level(display, &LevelSetup, Rng)`, `Game::location()`.
- `sf_disk::zip`: gains `write_stored(entries) -> Vec<u8>`.
- `sf_conform::sync`: `SyncPoint::Setup { level, setup, seed }`; case TOML gains its form.
- `sf_conform::bug` (new) and `main.rs` subcommands `bug replay|compare|case`.
- `sf_app::recorder` (new): the recording state machine, ending rules, zip writing, bugs dir.
- `sf_app::start_level_window`, `sf_app::recording_bar`, `sf_app::save_window` (new): native views,
  each a model without AppKit plus a thin view, as `controls_window.rs` is.
- `sf_app::menu`: `Command::StartLevel`, `Command::StopRecording`; `args.rs`: `--replay`,
  `--bugs-dir`.

### 3.3 Storage

The recording is memory until saved; the zip is written by `atomic_write.rs` (temp then rename), so a
crash mid-write leaves no half zip under the final name.

---

## 4. Testing

- Unit (pure): format round-trips, parser refusals, setup ranges, retiming, location, zip writer
  read back by `sf_disk::zip::read_entries`.
- Oracle (sf-conform tests, need the disks and kickstart like every case): `Setup` start equals the
  engine's start for every level and difficulty (T03); recorded routes through tally and slot stay
  in step (T05).
- App, script layer (in the test block): Start Level, recording across a level end and game over,
  ending rules, zip contents, replay (T06, T07).
- App, OS layer (`SF_E2E_OS=1`, not in the block, RE §5): the dialog, bar, ⌘., save window and
  question through real clicks and keys (T08–T10), and the drill (T11).
- None of these proves the owner can find a bug with it; T13's recording by the owner is that.

---

## 5. Environment — read this before running anything

RE §5 holds: macOS 26.5.1 arm64, rustc/cargo 1.99.0, Python 3.14.8, vAmiga pinned in
`~/.cache/superfrog-re/vamiga/`, Kickstart in `~/amiga-roms/`, disks in the main checkout root.

**The test command.** The block above: the Python suite and the Rust tests of the parts this plan
changes, not the whole workspace (owner, 2026-10-20, D11). Measured 2026-10-20: the full workspace in a
fresh worktree took 6 247 s (about 50 minutes of it tests, the rest a cold build) and its one failure
was `whole_game.rs` `the_app_paced_from_power_on_through_the_boot_loads_is_the_run_alone` ("the window
fell behind"), a wall-clock check that fails on a loaded machine. The scoped lines took 66 s, 10 s and
268 s on a warm build. Every new test file in sf-conform and sf-e2e is named `recorder_*.rs` so the
glob lines pick it up; the guards skip a glob line until such a file exists, because cargo fails on a
`--test` pattern that matches nothing. A worker whose change reaches another part (a fix in T13 most
likely) also runs that part's tests and says so; T13 runs the whole workspace once before the plan
ends. `.pir/settings.json` carries only the Python line, which tests none of this plan's work.
Quiet: `cargo test -q` prints dots and one result line per binary; for detail, add `-- --nocapture`.
Colour is off inside the commands (`NO_COLOR=1`, `CARGO_TERM_COLOR=never`); the session sets only
`COLORTERM=truecolor`, which forces nothing. `sf-disk`'s `atn_reference` test needs the venv, so the
setup lines come first.

**Setup.** As `rust-engine`: the venv and `tools/setup/oracle.sh` (a no-op when vAmiga is built).

**Dependencies.** RE's policy holds: anything is allowed, named with one line of why in the task that
adds it. Expected: more `objc2-app-kit` features (NSPopUpButton, NSWindow, NSAlert, NSPasteboard) for
the native views. No zip crate: the reader exists and a stored-entry writer is small.

**End to end.** `sf-e2e` exists and carries this (no rig task): the script layer (in the block)
gains script events for the new commands and windows, as `controls …` events drive the Controls
window; the OS layer gains real clicks on the new windows. Sizes: scale 1×, 2×, 3× and fullscreen on
the built-in display, as RE §5. The free backend is the asset cache in a temp home, with
`--bugs-dir <tmp>`.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| The owner can find and record a real bug with it | A real person's play; T13's recording is that check |
| Granting Accessibility and Screen Recording for the OS layer | A macOS privacy grant (RE §5.1) |

### 5.2 Seatbelts

RE §5.2 holds (`--max-frames`, `--timeout`, `--home`, windowed under `--script`). Added:

| Flag / mechanism | Default | Effect |
|---|---|---|
| `--bugs-dir DIR` | the repo's `bugs/` | tests always pass a temp dir; no test writes into the repo |
| `--replay` under tests | `--timeout 120` applies as under `--script` | a replay that never ends is killed |
| `bug compare` frame cap | the recording's own length plus the oracle's 20 000 load search | the oracle never runs unbounded |

Nothing in this plan touches anything outside this Mac and repo, so there is no §5.3.

---

## 6. Recovery

Nothing here can lock anyone out. A bad zip is deleted by hand; `bugs/` can be emptied at any time.

---

## 7. Decisions and rationale

- D1 (owner, 2026-10-20): the prototype (`prototype/index.html`) was approved as the direction: menu
  items, the dialog with Lives and Energy as dropdowns (Lives 3/5/7 like the options screen), the bar
  under the picture, the save window, the "Recording in progress" question. A non-binding reference.
- D2: how a level is started with a setup is T00's to measure, not to assume. Candidates: (a)
  `Start::Level` plus pokes, which skips the front end and leaves the variables `level_entry.rs`'s
  `NOT_MODELLED` lists (the RNG, fades, the music's place among them); (b) the player's way, as
  Continue does (`Game::start_at_with` + `type_code`): power-on, lives and difficulty set on the
  options screen, the level code typed, run unseen, then the four values at frame 0, the random state
  written on the original to match. The acceptance bug is at a screen transition, exactly where (a)'s
  leftovers could matter, so (b) is preferred unless T00 shows (a) is equal there too. The chosen
  one fixes what `seed` in §2.6 means.
- D3: inputs only, no snapshots (DKC1Recomp keeps snapshots because it records from anywhere; the
  owner refused that here).
- D4: STOP key ⌘. (owner, 2026-10-20): the Mac convention for stop, clashing with nothing.
- D5: the zip is not committed and lives in git-ignored `bugs/` (owner, 2026-10-20); RE §2.3 rules
  out committing the screenshot.
- D6: menu actions that change the game ask Save/Discard/Cancel; game over never ends a recording
  (owner, 2026-10-20).
- D7: a quirk produces a report and nothing committed (owner, 2026-10-20).
- D8: `--replay` is for agents and tests, watch-only (owner, 2026-10-20).
- D9: the recording parser lives in sf-core because sf-app may not depend on sf-conform (it pulls in
  the oracle, RE §3.2); the script token format moves there instead of being written twice.
- D11 (owner, 2026-10-20): the test block runs only the parts this plan changes, because the whole
  workspace costs about 50 minutes warm per review; the whole suite runs once, in T13.
- D10: extend, not rebuild: `Start::Level`/`start_at_with` for the start, `SyncPoint` for the
  original's side, `sf_disk::zip` for reading, `capture::png` for the screenshot, `sf-e2e` for the
  app tests, the Controls window's pattern for native views.

---

## 8. Explicitly out of scope

- Recording from anywhere, or always-on recording: no comparable start exists mid-game (owner).
- An automatic explorer that finds bugs: a later plan that will reuse this one's tools.
- The difficulty-mode comparison and the map editor: their own plans.
- Writing a recording to disk while playing (crash-proof recording): §2.11, not worth it.
- Replay controls for people (seek, fast-forward): D8.
