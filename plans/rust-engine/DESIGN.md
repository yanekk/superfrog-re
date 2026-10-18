---
setup:
  - python3 -m venv venv && venv/bin/pip install -q --disable-pip-version-check -r requirements.txt
  - [ ! -x tools/setup/oracle.sh ] || tools/setup/oracle.sh
test:
  - NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py' || [ $? -eq 5 ]
  - [ ! -f engine/Cargo.toml ] || (cd engine && CARGO_TERM_COLOR=never cargo test -q --workspace)
---

# Superfrog in Rust — Design

## 1. Purpose

A hand-written Rust program that plays the complete Superfrog (Team 17, 1993, Amiga) natively on a
modern Mac. The player supplies their own four Amiga disk images once; the program imports them and
plays the game with no emulator involved. It is a clean reimplementation: every mechanic is
understood from the original 68000 code and rewritten as readable Rust, not machine-translated.

It is for the project owner first, and for anyone who owns the disks. It is also the playable target
the later map editor needs, which is why it can play an edited level (§2.10).

### Success criteria

- Given the same inputs from the same starting point, the Rust engine is faithful to the original
  (§2.1) on every scripted conformance run: the same game state pass by pass, a picture and sound no
  player could tell apart, and, run alone with no help from the original, the same moments of play
  in the same order (every coin, death, kill, exit and screen). This includes every level of every
  world completed, and a scripted game from power-on to the end sequence played by the engine alone.
- Every part of the original is present: intro and story, title, options and passwords, every level
  of every world and the final boss, the slot machine and bonus levels, game over, the end sequence,
  music and sound effects.
- Nothing derived from the disks is in the repository or in the built program (§2.3).
- All of the above is established by the agents, headless, against the original on the oracle. No
  criterion needs the owner to play or look (owner, 2026-10-15).

### Stance

- Faithful, judged by a machine (owner, 2026-10-15; replaces "frame-exact"). The game must act and
  feel like the original: the same rules, the same moments of play, a picture and sound a player
  could not tell apart. It need not reproduce where in a frame the original's CPU, blitter or sound
  chip did their work. The original is still the specification, and an automated comparison against
  it is still the only judge of a mechanic; a port that "looks right" to the agent who wrote it is
  not done.
- The original's quirks and bugs are kept. Only behaviour that would crash or hang the original gets a
  safe replacement, each one listed in §2.11, because those cases cannot be compared anyway.
- Our code, their data. The engine contains no Team 17 bytes; it reads everything from the player's
  disks.
- Readable Rust. Each mechanic is a named module with names from our reverse engineering, so the
  editor and later mods can build on it.

---

## 2. Behaviour specification

### 2.1 What faithful means

Owner, 2026-10-15. This replaces the frame-exact definition that held from 2026-10-10. Of the 47
tasks open that day, 37 chased differences no player perceives: a colour one fade step off for one
frame, a sprite drawn a frame late because the original's blitter raced the beam, a sound-chip
write some cycles earlier. Meanwhile the engine could not start a game from the menu
(`unported: "a game's start (T21)"`). The bar moves to what a player perceives. It stays automatic
and it stays measured against the original.

Each step of the engine is one PAL vertical blank (one "frame"). Starting from a sync point (the
original's state captured by the reference emulator and transplanted into the engine, or power-on for
a whole-game run) and feeding the same joystick/keyboard state per frame, a case passes when all four
checks hold. `sf-conform`'s default mode (`run`, `all`, `scan`) is these checks (T180; check 4 is T182).

1. **State, pass by pass.** Every gameplay variable in `LAYOUT` (§3.3) equals the original's at the
   end of each completed main-loop pass, matched by pass count rather than frame number, so a pass
   that ends a frame later on one side is not a difference. Frames ending mid-pass are not compared,
   as before. Variables that only say how the frame was produced (copper lists, bitmap buffers,
   blitter and beam bookkeeping, the sound driver's register shadows) are tagged `presentation` in
   `LAYOUT` and are judged by checks 2 and 3 instead.
2. **Picture, within tolerance.** The picture inside the display window, as 12-bit OCS colour per
   pixel. A frame's difference is tolerated when every differing pixel is at most one step off in each
   of R, G and B, or when fewer than 64 pixels differ (a sprite's edge, a stray line); this is not limited in time
   (owner, 2026-10-15: a small wrong sprite with equal state is accepted). Any other
   difference is tolerated only while it lasts at most 2 frames in a row (one 25 Hz pass), and the
   tolerated frames of this last kind are at most 1% of the case's frames. Every tolerated frame is
   counted and printed: never silent (T120's rule stands).
3. **Sound, as events.** The Paula writes are reduced to sound events per channel: a sample started
   (start address, length, period, volume), a period or volume change, a channel stopped. Each of
   the original's events is matched by the same event on the same channel within ±2 frames, in the
   same order per channel. The order of writes within a frame and their beam positions are not
   compared.
4. **Plays alone.** The engine is started once from the sync point and then runs with nothing from
   the original: no re-syncs, no beam anchors, no transplant. It gets only the inputs and the
   original's pacing (which frames the original's pass was late), so a lag on the original does not
   shift the inputs relative to the passes. The RNG seeds the original reads from its clock (at
   power-on, and `rng_seed_from_cia` at the end sequence's end) count as inputs, since the engine keeps
   no clock. So does each load's length, which is the disk's timing: the vertical blanks the game's
   handler ran while the original loaded (not under a screen the engine shows meanwhile), which the
   engine runs as the original's load ends (`Game::load_blanks`: the sound and the keyboard's poll),
   the line the load ended on, and how long the bonus level's loading picture stayed up for its loads
   (owner, T209). Its milestones must equal the original's, in the same order (milestones within 10 frames of each other may come in either order), each within ±10 frames of
   play (the original's load frames, its waits for a disk swap among them, are not counted): every scene
   change (title, menu, level start, tally, slot machine, bonus level, game over, end sequence),
   every coin and item taken, every enemy killed, every life lost, every level exit, the score at
   each tally, and at the end score, lives and coins equal. This is the check that proves the game
   runs on its own, and the one the whole-level and whole-game cases are judged by.

Checks 1–3 may re-sync after a lag as before, which is why check 4 exists: a re-sync can hide a
divergence, a run alone cannot. A case may name only some checks (`compare = [...]`), but every
case that steps the engine runs check 4 (owner, 2026-10-15: not only the completion and whole-game
cases, so a re-sync can hide nothing anywhere). A `render`-only case steps nothing and is exempt.

The thresholds above (one colour step, 64 pixels, 2 frames, 1%, ±2 frames, ±10 frames) are the
owner's bar. A task that finds one wrong raises it with the owner or the coordinator agent; it is
never changed in code to make a case green.

The frame-exact comparison stays in `sf-conform` as `--strict`, for diagnosis: when a faithful check
fails, the strict run shows the first frame and variable that differ. No task's "Done when" requires
`--strict` green.

How cases line up with the original's timing, which remains how every check aligns the two sides
(owner, 2026-10-11 and 2026-10-13):

Lag and randomness (T07 measured both; owner, 2026-10-11). In a level the original's main loop runs
a 25 Hz pass every second frame and sometimes overruns it (docs/08 "Lag"). Conformance runs feed the
original's schedule to the engine: `Game::step_paced(input, Pacing)`, where `Pacing::Late { writes,
line, hpos, phase, step }` says the original's pass was still running at that frame's vertical blank,
read from the oracle: after the frame's `writes`th Paula write, the last at beam `line`, slot `hpos`,
in the pass's `phase`th call, `step` slots into its loop. The engine ends its frame at the same point
and runs the rest of the pass after the next interrupt (T36). Where in the frame the engine's work
falls is its beam clock (`level::beam`): the ported routines say what they cost, fitted to the
original. Conformance also anchors that clock at the original's own beam positions (owner,
2026-10-13): `Game::anchor_writes` sets it to where each of the frame's Paula writes was made, and
`Game::anchor_blits` starts each blit where the original started one of its size; the cost model
places the work between anchors. Divergence left over is reported as lag and re-synced. Normal play
(`Game::step`) is always `Pacing::OnTime`, never anchored, and uses the cost model alone: it never lags.
So the shipped game runs smoothly where the original slowed down; accepted (owner, 2026-10-15), and
measured: `sf-conform` prints per case how many frames the original was late, and T181 records the
totals per level in docs/08 "Lag".
The RNG is seeded once from a clock, so the seed is part of the start: a transplant copies
`rng_state`, a power-on conformance run takes the oracle's seed, normal play seeds from the host clock.

Where the engine starts, and loads (owner, 2026-10-12). Power-on for the engine is the moment the
game program (`d1_c03`) starts: the Kickstart screens and the crack's intro are not the game's and are
not reproduced. Disk loads are not modelled frame by frame: in the engine a load is instant, and
conformance runs skip the original's frames spent loading and re-sync the engine's variables when
the load ends, as they do after lag.

### 2.2 Disks and import

- Accepted input is exactly the four "[cr CSL]" disk images the owner has, zipped or as raw `.adf`,
  identified by SHA-1 of the ADF contents. Any other image is refused with a message naming the disk
  slot it was offered for and whether it is missing, damaged or a different version. Supporting other
  dumps was declined (§8) because each would need its own verification.
- Import happens once. The disks are decoded into a cache in the app-data folder (§3.5). The cache
  records its format version, the engine version that wrote it, and a hash of its contents.
- On launch the cache is verified. If it is missing, damaged or from another cache format version, the
  engine re-imports from the disk paths it remembered; if those are gone or changed, it asks for the
  disks again. The disks are opened read-only and nothing is written beside them.

### 2.3 Copyright boundary

- Graphics, sound, levels, strings and every numeric table the game uses are read from the player's
  disks at import, located by chunk and offset. None are copied into Rust source, tests, docs or
  fixtures. The reason: the repository and the built program must be shareable without carrying any
  Team 17 data.
- Facts about the original (addresses, offsets, sizes, formats, the meaning of a variable) are ours to
  write down and are committed. A disassembly listing or decompiled output of the original is not; it
  is regenerated outside git (§5).
- Recorded conformance traces and captured frames are derived from the disks and live in a cache
  outside the repository (§3.5).
- Enforced by `tools/check_no_disk_data.py` (T01), which fails if any tracked file contains a run of 128
  or more bytes found in any disk chunk, and by the purity test (§3.1), which flags integer array
  literals of more than 16 elements in the pure crates. A flagged table must be read from the disks
  instead; the check is never relaxed for it.

### 2.4 Unpacking

The `ATN!` decompressor is hand-ported to Rust and proven byte-identical to the emulated original
(`tools/atn.py`) on every chunk that decodes there. The emulated version stays in the repository as
the reference. This replaces the old rule "do not reimplement it", which held while emulation was
acceptable; the shipped game may not contain an emulator (the owner's requirement, 2026-10-10). The
three chunks the Python harness fails on (`d1_c16`, `d2_c39`, `d4_c16`) are checked against the
original's own memory after the game loads them (T07).

### 2.5 Timing

The engine runs at the PAL frame rate, 49.92 Hz, on any display, because game speed, music tempo and
the state check (§2.1) are all defined per PAL frame. On a 60 Hz or variable-rate display the newest
completed frame is presented at each display refresh; frames are never interpolated, because an
interpolated frame is not a frame the original drew. The audio device's sample clock paces the
simulation so sound never drifts from picture.

### 2.6 Input

The original reads one joystick (directions + fire) and some keys. The engine maps keyboard keys and
gamepad buttons onto exactly those original inputs and nothing else, so the simulation cannot tell
which device was used. Bindings are remappable and remembered (§2.9). A gamepad connected or
disconnected mid-game changes nothing but which device feeds the mapping.

### 2.7 Display options

- Integer scaling only (1× to the largest that fits) in a resizable window, or fullscreen with the
  largest integer scale and black borders, because non-integer scaling smears pixel art unevenly.
- CRT filter: an optional shader applied after scaling. It changes presentation only.
- Widescreen: the renderer draws extra columns of the level on each side using the same state. Game
  logic is identical in every mode, so faithfulness (§2.1) holds in all of them (owner's decision,
  2026-10-10). Objects the original has not activated stay invisible or frozen at the edges; this is
  the accepted cost. Enforced by a test: the centre 320 columns of a widescreen frame equal the 4:3
  frame for every frame of the conformance runs.

### 2.8 Save states

Save and load anywhere through the menu. A save state is the serialized engine state, tagged with the
engine build identity and the asset cache hash. A state written by a different build is refused with a
message rather than loaded, because the state layout changes as mechanics are added and a
half-compatible load would silently corrupt a game (owner's decision: no cross-version states). Loading
a state then running N frames must give the same frames as having never saved.

### 2.9 What persists

Settings (scale, fullscreen, CRT, widescreen, key and pad bindings) and progress (the furthest world
reached, offered as a start point so passwords need not be retyped) persist in the app-data folder.
The start point is the menu item Game → "Continue from World N", disabled until World 2 is reached; it
starts that world exactly as its password would (`Game::start_at`), so nothing new is drawn in the game
(owner, 2026-10-10).
High scores last for the session only, as on the Amiga (owner's decision). A damaged settings or
progress file is replaced by defaults with a one-line message, never a refusal to start, since losing
a preference is cheaper than locking the player out.

### 2.10 Level override (the editor's target)

The engine can play a level from a `.sflevel` file in place of the disk level it names. The format is
JSON: `{"format":"sflevel","version":1,"replaces":"W1-1","sections":{"<name>":"<base64>"}}`, where the
sections are the original per-level files in their original formats (at least `t7mp`; T07/T08 settle
whether object placement lives in another file). Reusing the original formats means one loader serves
disk levels and override files, so an unchanged export plays frame-identically by construction. The
engine also exports any disk level to this format (locally, for the owner; never committed).

### 2.11 Unhappy paths

- Wrong, missing or damaged disk: refused at import with a message naming the slot (§2.2).
- Damaged or stale cache: re-import, else ask for the disks (§2.2).
- Disk picker cancelled, or fewer than four disks chosen: a message naming the missing disks with
  "Choose Disks…" and "Quit" (owner, 2026-10-10), so the player always knows what is missing.
- Crash or power loss mid-write of the cache, settings, progress or a save state: every file is written
  to a temporary name in the same folder, flushed, then renamed over the old one, so a reader sees the
  old file or the new one and never half of one.
- Two copies running: the second one shows a message and quits, because two writers to progress and
  settings would race. A lock file in the app-data folder holding the PID decides; a stale lock (dead
  PID) is taken over.
- Original crash or hang conditions (a pointer run off a table, an infinite loop): reproduced up to
  the point of failure, then replaced by the safest nearby behaviour, each listed here with its
  address when found:
  - `level_tally` (h0+$4550): a coin taken in a level with fewer than five items divides by zero;
    the coins give no page (T38; no level on the disks has so few, an edited one may).
  - `frog_tiles`'s wall test (h0+$e3fe..$e412): with both wall flags set (frog `+$0e` and
    `+$10`, the frog lifted into a pillar between two `$3d` edges) it loops for ever; the engine
    takes the probe list of the side the frog faces (T14). Reached on W2-1 with world 2's enemies
    frozen (T105: frog lifted into the pillar at 77-80,36-39). Also met at W2-4's closed gate
    (19-20,55-63, two `$3d` columns; switch 46,66): the frog against its left side, x 303, facing
    left, loops there on the original; the engine falls on (T168, `examples/wall_place`). With the
    gate closed no search here got the frog to that side (FINDINGS); for the owner.
- A save state from another build: refused (§2.8). An `.sflevel` with a bad version, missing section
  or malformed T7MP: refused with the reason, and the disk level is played instead only if the player
  chooses it.
- Audio device missing or lost: the game keeps running silently on a wall-clock pacer, since sound is
  not needed to play.

---

## 3. Architecture

### 3.1 The boundary

```
engine/crates/sf-disk     pure   ADF/zip bytes -> identified disks -> chunks -> ATN! decompression
engine/crates/sf-assets   pure   chunks -> GameData (exe hunks, T7MP, gfx, sound, tables by offset), cache codec
engine/crates/sf-core     pure   the game: State, Game::step, renderer, audio driver, layout table
engine/crates/sf-paula    pure   Paula register writes -> PCM
engine/crates/sf-app      shell  the `superfrog` binary: window, GPU, audio device, input, menus, files
engine/crates/sf-oracle   shell  test tooling: vAmiga full-system reference + Moira routine runner
engine/crates/sf-conform  shell  test tooling: conformance runner, input scripts, trace cache
engine/crates/sf-e2e      shell  end-to-end tests driving the built app
```

The pure crates take everything as parameters and return values. They read no clock, touch no file,
socket, thread, environment or process, use no floating point and no randomly-seeded hash map. The
reason for each: a clock or file makes a test depend on the machine; floating point and hash-map order
are the classic ways two runs of a "deterministic" simulation drift apart, and the state check (§2.1) is
defined bit for bit. Everything on the pure side is testable exhaustively in milliseconds; every rule
that leaks into the shell becomes a rule only the slow rig can check.

Enforced by `engine/crates/sf-conform/tests/purity.rs` (T01): it scans the pure crates' sources for
`std::fs`, `std::time`, `std::net`, `std::thread`, `std::env`, `std::process`, `Instant`, `SystemTime`,
`HashMap`, `HashSet`, `f32`, `f64`, and integer array literals over 16 elements (§2.3), and checks their
`cargo metadata` dependencies against an allowlist. One exception (owner, 2026-10-10):
`sf-paula/src/resample.rs` may use floating point and filter tables, because it only turns Paula's
channel output into host-rate audio; that output feeds no game state and is not compared frame-exactly
(the per-channel taps before it are). The test exempts that one file and nothing else. If that test fails, the fix is to move the code to
the shell side or read the data from the disks, never to relax the test.

### 3.2 Modules

- `sf-disk`: `identify(bytes) -> DiskId`, chunk carving (port of `tools/atn.py`'s carver),
  `atn::decompress`. Depends on nothing but a pure zip reader.
- `sf-assets`: decoders for each chunk class and `GameData`, the complete decoded content the game
  needs, plus its versioned cache encoding. Depends on `sf-disk`.
- `sf-core`: one module per area of the game (frog, items, enemies per world, bosses, slot, front end,
  flow), the renderer, the audio driver and `layout` (§3.3). Depends on `sf-assets` types.
- `sf-paula`: Paula channel model at the Paula clock and a band-limited resampler to the host rate.
- `sf-app`: winit (window, events), wgpu (scaling, CRT shader), cpal (audio), gilrs (gamepad),
  muda (macOS menu bar), rfd (file dialogs), serde/toml (settings). SDL2 is permitted (§5) and held in
  reserve should gilrs fail the 8BitDo or wgpu fail a need.
- `sf-oracle`: a C++ shim over the vAmiga core (MPL-2.0) and its Moira CPU (MIT), built outside the
  repo (§5), exposed to Rust via `cc`/FFI. Never a dependency of `sf-app`; the purity/dependency test
  also asserts that `sf-app`'s dependency tree contains no `sf-oracle`, because the shipped game may
  not contain an emulator.
- `sf-conform`: input scripts, sync points, the per-frame comparison, trace caching, divergence
  reports.

### 3.3 The decision function

```
Game::new(data: &GameData, start: Start) -> Game          // Start::PowerOn { seed } | Start::Transplant(AmigaMemory) | Start::Level(LevelId)
Game::step(&mut self, input: FrameInput) -> FrameOut       // one PAL frame of normal play
Game::step_paced(&mut self, input, pacing: Pacing) -> FrameOut  // on the original's schedule (§2.1)
FrameOut { picture: Picture, paula: Vec<PaulaWrite>, disk_prompt: Option<DiskPrompt> }  // Picture = indexed pixels + per-line palette; disk_prompt added by T21
Game::render(&self, view: View) -> Picture                  // View { extra_columns_each_side }
Game::probe(&self) -> Vec<(VarId, u32)>                     // values at the original's addresses
Game::save(&self) -> Vec<u8>; Game::load(&GameData, &[u8]) -> Result<Game, StateError>
```

`Game` is a function of `GameData`, its start and the input sequence and nothing else. The `layout`
module is the single table pairing each modelled variable with the original's address and width; both
transplant (`Start::Transplant`) and `probe` are driven from it, so a variable is either fully mapped
or not mapped at all.

### 3.4 Data flow

Import: disk files (shell) → `sf-disk` → `sf-assets::GameData` → cache file (shell). Play: cache →
`GameData` → `Game`; each 1/49.92 s the shell reads input, maps it to `FrameInput`, calls `step`,
hands `paula` to `sf-paula` → audio device and `render(view)` → GPU. Conformance: `sf-oracle` runs
the original with the same `FrameInput`s and `sf-conform` compares `FrameOut` and `probe` frame by
frame.

### 3.5 Storage

- App data: `~/Library/Application Support/Superfrog-RE/` containing `cache/assets-v{N}.bin`,
  `settings.toml`, `progress.toml`, `states/*.sfstate`, `disks.toml` (remembered disk paths), `lock`.
  `--home <dir>` or `SUPERFROG_HOME` overrides it; every test sets it (§5.2).
- All writes are temp-file-then-rename in the same folder (§2.11).
- Tool caches outside git: `~/.cache/superfrog-re/vamiga/<commit>/` (built reference emulator),
  `~/.cache/superfrog-re/traces/` (oracle traces keyed by disk set hash, vAmiga commit and script hash),
  `~/.cache/superfrog-re/re/` (generated disassembly and decompilation).
- Committed RE knowledge: `re/symbols.toml` (address, name, kind, signature, note per entry) and
  `docs/08-…` onward.

---

## 4. Testing

- **Unit tests** on the pure crates: decoders, the decompressor, each mechanic's rules, the mixer.
- **Routine conformance**: a ported routine and the original routine (run on Moira from a memory
  snapshot by `sf-oracle`) are given the same inputs, many of them generated; outputs and memory
  effects must match.
- **Frame conformance**: scripted input runs from sync points, judged by the faithful checks of §2.1
  against vAmiga. The short runs are in the test block; the full suite (every level completed, the
  whole game from boot) is `cd engine && cargo run -q --release -p sf-conform -- all`, run by the
  mechanic tasks, the drill and acceptance, because it takes minutes. `--strict` is the frame-exact
  comparison, for diagnosis only.
- **End to end**: the real app driven by `sf-e2e` (§5, End to end), including a scripted whole game
  through the real binary, fed the original's pacing as the headless run is, whose milestones match
  the original's (T29, T30).
- What none of these prove: that the 8BitDo pad works on real hardware, and that sound comes out of
  the speakers (§5.1). Neither is a gate (owner, 2026-10-15).

Input scripts are written by workers (owner's decision, 2026-10-10): text files of `frame: inputs`
lines under `engine/conform/scripts/`, steering by reading the oracle's state. They contain inputs
only, never disk data, so they are committed.

---

## 5. Environment — read this before running anything

| | |
|---|---|
| OS | macOS 26.5.1, Apple Silicon (arm64) |
| Language / runtime | Rust stable via rustup (installed by T01); Python 3.14.8 (Homebrew) for the existing tools |
| Toolchain | Apple clang 21 (Xcode CLT); Homebrew; cmake via brew (T00) |
| Reference emulator | vAmiga core, pinned commit (T00 picks; `899da5dd` was current on 2026-09-30), built headless into `~/.cache/superfrog-re/vamiga/` |
| Kickstart ROMs | `~/amiga-roms/` (`kick1.3.rom`, `KICK204.ROM`, `kick40068.A1200.rom`, `Kick.rom`, all 512 KB). Override with `SUPERFROG_KICKSTART`. Never copied into the repo or a cache |
| Disk images | the four zips in the main checkout root; tests find them via `SUPERFROG_DISKS`, else `$(git rev-parse --path-format=absolute --git-common-dir)/..`. A test that needs them and cannot find them fails with that message rather than skipping |
| **Deliberately absent** | No Ghidra or Java yet (T02 decides; the Ghidra MCP server is configured but nothing serves it). FS-UAE 3.2.35 is installed but is not the oracle: it cannot be scripted per frame. No Windows/Linux machine, so nothing here is verified there |

**The test command.** The `test` lines of the block at the top of this file. The first runs the
Python suite (none exist yet, so exit 5 is accepted). The second runs the Rust workspace once it
exists. `cargo test -q` prints one line of dots and one result line per test binary; for detail run
`cargo test --workspace -- --nocapture` in `engine/`. Colour: the session environment sets
`COLORTERM=truecolor` and nothing forces colour, but `CARGO_TERM_COLOR=never` and `NO_COLOR=1` are set
inside the commands so nothing depends on the caller.

**Setup.** A fresh worktree needs the venv (gitignored, measured clean on 2026-10-10, 3 s) and, once
T05 lands, `tools/setup/oracle.sh`, which builds vAmiga into the shared cache only if that commit is not
already there, so parallel worktrees share one build. Rust itself is a machine install (T01), not a
per-worktree step.

**Dependencies.** Anything is allowed, including C libraries via Homebrew (owner, 2026-10-10). Each
new crate or library is named in the task that adds it with one line of why. The pure crates stay on
the allowlist in the purity test; adding to it needs a reason that the crate is pure.

**End to end.** Nothing exists in the repo today. T22 adds `sf-e2e` with two layers. The script layer
launches the real `superfrog` binary with `--home <tmp> --script <file> --capture <dir> --max-frames N`;
the script's events enter at the point where winit events are translated, so key mapping, menus
commands and the game loop are exercised in the real process, and every presented frame and the mixed
audio are written to the capture folder. The OS layer posts real key events with CGEvent, clicks menu
items through System Events and captures the window, so the real macOS input and menu path is proven
for each control at least once; it needs Accessibility and Screen Recording granted once (§5.1).
Sizes: window scale 1×, 2× and 3× (320×256, 640×512, 960×768 game area), widescreen at 2×, and
fullscreen on the built-in display. The free backend is the asset cache built from the owner's disks
into the temp home. The script layer is in the test block once it exists, bounded by `--max-frames`.
The OS layer runs only with `SF_E2E_OS=1`, set by the worker of the task that adds an OS-layer test and
by the drill (owner, 2026-10-10): it posts real keys and clicks to whichever app is in front, so in the
shared test block it would type into the owner's apps and parallel workers would fight over focus.
Without the variable those tests report themselves skipped, never passed.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| The 8BitDo Ultimate 2 drives the game | A physical pad cannot be simulated at the HID level here. Gamepad support is built and tested with simulated pad input; the hardware check is not a gate and stays marked unverified (owner, 2026-10-15) |
| Sound comes out of the speakers | The speakers are hardware. The mixed output is checked in captures; not a gate (owner, 2026-10-15) |
| Granting Accessibility and Screen Recording to the terminal that runs the OS-layer tests | A macOS privacy grant only the user can click |

Whether the game feels like Superfrog is no longer on this list: §2.1's checks are its definition,
and the owner is not asked to play (owner, 2026-10-15).

### 5.2 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| `--max-frames N` (app) / frame cap (oracle) | required under `--script`; oracle 20 000 | process exits after N frames; a test without it is refused |
| `sf-conform record` output cap (`--max-mb N`); entries over 1 KB left out unless `--all-vars` | 200 MB | stops with an error naming the frame and the flag; a recording can no longer fill the disk (2026-10-15: a 38 GB one crashed the run) |
| `--timeout S` | 120 under `--script` | wall-clock kill of the app and any child |
| `--home DIR` / `SUPERFROG_HOME` | the real app-data folder | tests always pass a temp dir, so a test never touches the owner's settings, progress or states |
| windowed under `--script` | on | a scripted run cannot go fullscreen unless it also passes `--allow-fullscreen`, which requires `--max-frames ≤ 300` |
| read-only disks and ROMs | always | the importer and the oracle open them read-only and write nothing beside them |

Never run the app fullscreen without a frame cap during a task, and never run the oracle without one.

### 5.3 Outside the code — who acts

Nothing in this plan touches a cloud account, paid service or another person. The actions are
installs on this Mac, which the owner moved to `worker` on 2026-10-10 ("workers install, no prompt")
and confirmed at plan review the same day.
No credentials are needed by any action (measured: none of the rows uses a login).

| Action | Command (exact, wrapped) | Bin | Why this bin | Way back | Expected cost | Login check |
|---|---|---|---|---|---|---|
| Install Rust | `tools/setup/install-rust.sh` | worker | owner's decision, 2026-10-10 | `rustup self uninstall` | free | — |
| Install brew build deps (cmake) | `tools/setup/install-brew-deps.sh` | worker | owner's decision, 2026-10-10 | `brew uninstall cmake` | free | — |
| Install Ghidra (only if T02 adopts it) | `tools/setup/install-ghidra.sh` | worker | owner's decision, 2026-10-10 | `brew uninstall --cask ghidra; brew uninstall openjdk` | free | — |
| Fetch and build vAmiga at the pinned commit | `tools/setup/oracle.sh` | worker | builds into a cache outside the repo | delete `~/.cache/superfrog-re/vamiga` | free | — |
| Download crates during `cargo build` | (cargo) | worker | ordinary build | `cargo clean` | free | — |
| Grant Accessibility + Screen Recording to the terminal | System Settings | person | macOS privacy grant | revoke in System Settings | — | — |
| Use the 8BitDo pad (optional, not a gate) | — | person | physical device | — | — | — |

---

## 6. Recovery

The game cannot lock anyone out. Fullscreen exits with Esc-then-menu, Cmd+Ctrl+F or Cmd+Q. To reset
everything, quit and delete `~/Library/Application Support/Superfrog-RE/`; the next launch asks for
the disks. The tool caches in `~/.cache/superfrog-re/` can be deleted at any time and are rebuilt.

---

## 7. Decisions and rationale

- **Clean reimplementation, not static recompilation or a hybrid** (owner, 2026-10-10). Readable,
  moddable code that pairs with the editor; faithfulness is bought with the conformance suite instead.
- **Frame-exact** (owner, 2026-10-10), **replaced by faithful** (owner, 2026-10-15). Defined in
  §2.1. Both need a scriptable reference emulator. Frame-exact cost five days of sub-frame timing
  work (beam races, bus-cycle models, copper colour races) while the engine still could not chain a
  menu into a level; faithful keeps the automated comparison and drops what a player cannot see or
  hear. Fourteen tasks that only chased sub-frame timing were dropped (PLAN.md, amendment
  2026-10-15).
- **No hand verification as a gate** (owner, 2026-10-15). The agents prove the game headless; the
  owner's play session and the 8BitDo hardware check are optional.
- **vAmiga as the reference, not FS-UAE or a homemade harness** (planner). FS-UAE has no per-frame
  scripting or memory access. A homemade harness would need its own blitter, copper and display
  emulation to produce frames, i.e. writing an emulator. The vAmiga core is MPL-2.0 with an MIT CPU,
  has a headless build (`Core/Headless.cpp`), and can be linked into test-only tooling. T00 proves it.
- **Moira for routine-level tests rather than the Python `machine68k`** (planner). One C++ tree builds
  both oracles and is callable in-process from Rust tests; the Python tools stay as the `ATN!`
  reference only.
- **Python tools kept as references, not rebuilt or retired** (planner, Stage 4 survey). `atn.py`,
  `t7mp.py` and `render_levels.py` are the yardsticks for the Rust decoders; `find_secrets.py` and
  `export_annotated.py` serve the editor and are untouched. The Rust versions exist because the
  shipped game cannot run Python.
- **Port `ATN!` to Rust** rather than shipping a tiny 68k core for import (owner). §2.4.
- **Everything from the disks, nothing embedded** (owner). §2.3.
- **Only the owner's disk set** (owner). §2.2.
- **One-time import to a cache** rather than decoding every launch (owner).
- **Native macOS menus and dialogs**, nothing new drawn in-game (owner). The original's screens stay
  untouched; no mock was built, since the in-game look is the original's and the menus follow macOS
  conventions.
- **Widescreen is visual only** (owner). §2.7.
- **Keep all original bugs, fix crashes only** (owner). §1 Stance, §2.11.
- **Persist settings and progress; not high scores; save states are build-specific** (owner). §2.8,
  §2.9.
- **Workers write the input scripts** (owner). §4.
- **winit/wgpu/cpal/gilrs/muda/rfd over SDL2** (planner). Pure-Rust crates avoid a Homebrew runtime
  dependency for the shipped app; SDL2 stays permitted as the fallback for gamepad or windowing.
- **PAL rate 49.92 Hz, no interpolation, audio-clock pacing** (planner). §2.5.
- **RE notes committed as our own symbols file; listings regenerated** (planner). §2.3.

---

## 8. Explicitly out of scope

- The map editor itself: a later plan; this one only gives it a target (§2.10).
- Windows, Linux and web builds: macOS only (owner); nothing here can verify the others.
- Other disk versions (IPF/original release, CD32, other cracks): each needs its own verification (owner).
- Saved high scores: session-only like the original (owner).
- Save states across engine builds: refused instead (owner).
- Gameplay changes in any mode, including widescreen and an optional "fixes" toggle: faithfulness
  is the product (owner).
- Sub-frame timing of the original (where in a frame its CPU, blitter, copper or sound chip did
  their work) beyond what §2.1's checks need (owner, 2026-10-15).
- Shipping or bundling any disk data, ROM or the reference emulator.
