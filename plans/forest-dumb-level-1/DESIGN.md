---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - tools/smoke.sh
---

# Forest Dumb Forever, level 1 — Design

## 1. Purpose

The owner opens the app, lands straight in `W1-1` and can play it from start to end with its collectibles, enemies and special mechanics. It is one plan because nothing less than a working level is tangible (owner, 2026-10-31). It is the template later level plans follow, but it does not generalise for them beyond what level 1 needs.

### Success criteria

- A route found by the planner reaches the end of `W1-1`, with no life lost, on the engine and on the original.
- A test plays that route in the real app from launch to the end of the level.
- Movement, collision, scrolling and picture match the original under the faithful bar, judged by machine.
- A recording made in the app replays headless to the same result and can be compared with the original.
- Superfrog's checks stay green and its planned routes do not change.

### Stance

- The cracked `[cr WT]` set is the measuring stick. The owner never judges faithfulness by playing or looking (`knowledge/self-testing-faithful-bar.md`), because frame-exact chasing cost days on the first game and owner checks do not scale.
- Superfrog's game logic is a pattern for how to port, never a source for how this game behaves. An engine matches its own original.
- `docs/07-executable-map.md` is mostly read and not run, so a name there is a reading until a task measures it.

---

## 2. Behaviour specification

### 2.1 Faithful

Compare state per game pass, with a tolerant picture check, and the engine running alone must meet the original's milestones. Sound is not compared, because this plan is silent. Sub-frame timing is out: it does not change what the bar judges.

### 2.2 What is correct before everything exists

The original always has enemies and collectibles. Until they are modelled, cases compare movement, collision, scrolling and picture on stretches where nothing else interferes (owner, 2026-11-01). Once T00 and T21 have measured the entry points, cases may freeze objects inside the original as Superfrog's `freeze.rs` does, because that compares whole runs. Every exclusion is listed in its case file with the task that removes it, so no gap is silent. By T27 no exclusion remains apart from sound.

### 2.3 Starting

The app opens straight in `W1-1`, silent, recorder off (owner, 2026-11-01). There is no title or `SCENA 1` screen: those belong to the title and menus plan. The start state equals the original's state at the level sync point, the one T03 fixes, so routes and recordings are comparable with the original.

### 2.4 Lives and game over

Lives behave as in the original. At game over the original's game over screen shows, the high score screen is skipped, then the level restarts with full lives and score zero (owner, 2026-11-01). High scores are skipped because the original never saves them and there is no title to return to.

### 2.5 Finishing the level

The original's end-of-level sequence plays where the level ends. Then the app shows "Level 1 complete" until fire is pressed, and the level restarts with full lives and score zero, as after game over (owner, 2026-11-01), because every attempt then starts from the state the original is compared from. The message is not the original's. It lives in the app's level-end scene, marked for removal when world 2 arrives, and is excluded from comparison by ending the comparison at the point the original starts loading world 2.

### 2.6 Status bar

Drawn as in the original from T10, because the picture comparison needs it.

### 2.7 The app

Same as Superfrog's where listed, nothing else (owner, 2026-11-01), because the rest is unrelated to getting the level playing.

- Keys default to the owner's: Up on Space, Fire on Left Option (`knowledge/owner-controls.md`). Rebinding is in the same Controls window. Game pad works.
- Native menu bar with Start Level, Controls, Quit. Esc also quits. Nothing new is drawn in-game, except §2.5's message.
- Start Level opens the same modal as Superfrog's, with a level selector offering only level 1 and "Enable recorder?" (owner, 2026-11-01). More fields are added as the game's options are discovered. Lives, points and such are not added now, because their meanings are not measured.
- No pause, since it is not listed. Losing focus releases held keys and play goes on, as `sf-app/src/input.rs` does.
- A plain picture: no filters, no widescreen.
- Settings live at `<home>/settings.toml`. Home is `--home`, then `FOREST_DUMB_HOME`, then `~/Library/Application Support/Forest-Dumb-RE`.

### 2.8 Recorder

As Superfrog's (`plans/bug-recorder/DESIGN.md`). A recording begins only at a level start from the Start Level modal, because a start the oracle can reproduce is what makes it comparable. It runs until Stop, through game over and the restarts of §2.4 and §2.5, because a bug may come straight after them (owner, 2026-11-01, as Superfrog's). Start Level and Quit during one ask Save, Discard or Cancel, as Superfrog's. It holds inputs, not snapshots. Stop shows the same bar, note and save window and writes one zip, to `games/forest-dumb/bugs/` (git-ignored) or `~/Documents/Forest Dumb Bugs` when there is no checkout. Under lag, app frames are retimed as `plans/bug-recorder/FINDINGS.md` describes; T15 measures it for this game.

### 2.9 Disks

The app uses `fd-tools`' lookup (`docs/01-disk-location.md`) and a `--disks` path. There is no picker (owner, 2026-11-01), because a picker belongs with the title and menus plan.

### 2.10 Map page

An HTML page of `W1-1`'s map with the character's path, opening from disk. It holds disk graphics, so it stays in `build/` and is never committed.

### 2.11 Route planner

The planner plans the whole of `W1-1` within this plan (owner, 2026-11-01). While mechanics are unbuilt, T20's test is "reaches the furthest point possible", with the expected point written in its test. T27's is the full route. A route passes only when it reaches the level-end milestone with no life lost, because Superfrog's DESIGN §2.6 records a route passing a comparison while dying. When a route stalls, the planner shows where on the map page and asks the owner to play that stretch, rather than adding per-level hints (`games/superfrog/docs/planner-stall-ask-owner.md`).

### 2.12 The unhappy paths

- Disks missing or wrong: the app shows a plain message naming where it looked, and exits non-zero. Tests fail naming the path, never skip, as `first_case.rs` does.
- Kickstart missing: the conformance tools name the path. The app never needs it, because it holds no emulator.
- Second copy: refused by `amiga_app::lock`, so two copies cannot share a settings file.
- Crash during a settings or zip write: `amiga_app::atomic_write` keeps the old file.
- Corrupt `settings.toml`: defaults are used and the problem is named on stderr, as Superfrog's does; the file is not overwritten until the owner changes a setting.
- A recording from a different build or level: replay refuses it and names the difference.
- Planner stall: §2.11. An out-of-budget run is reported, never left running (§5.2).
- Original and engine differ: the case stops at the first difference and names pass, frame and values. The fix is to the engine, never to the comparison.

---

## 3. Architecture

### 3.1 The boundary

```
fd-core, fd-assets   take inputs as parameters, return decisions. No clock, I/O, floats, hash maps or network.
amiga-plan's core   no clock, I/O or network. Hash maps and floats stay, as in sf-plan's core, because replacing them could change routes T02 pins; its purity test is sf-plan's list.
fd-app, fd-conform, fd-e2e, fd-plan's shell, fd-routeview, fd-tools   everything platform-shaped.
```

`fd-conform/tests/purity.rs` enforces it for this game, including the `ALLOWED` dependency list and the bar on tables over 16 integers. That bar stays because game tables must be read from the disks at run time, which keeps copyrighted data out of the repo. If it fails, the fix is to move the code, never to relax the test. New pure crates join its `PURE` list, each new dependency with a why-pure line. The app never links `fd-conform` or `amiga-oracle`, because the shipped game may not contain an emulator. Everything on the pure side can be tested in milliseconds; every rule that leaks across needs a rig.

### 3.2 Modules

| Crate | Owns | Depends on |
|---|---|---|
| `fd-assets` (new, pure) | Level, sprite and table readers over unpacked bytes | none |
| `fd-core` | `Game`, level state, character, camera, objects, scenes, drawing, `LAYOUT` | `fd-assets`, `amiga-chips`, `amiga-clock`, `amiga-input`, `amiga-record` (T14, on `ALLOWED`) |
| `fd-tools` | Disk lookup, extraction, unpack | `amiga-disk` |
| `fd-conform` | Cases, level sync, transplant, routine rig, bug replay and compare | `fd-core`, `amiga-oracle`, `amiga-conform`, `amiga-record` |
| `fd-app` (new) | Window, menu, pacing, recorder wiring | `fd-core`, `fd-tools`, `amiga-app`, `amiga-record` |
| `fd-e2e` (new) | `AppSpec`, tests of the built app | `amiga-e2e` |
| `amiga-plan` (new, toolkit) | Planner core and `PlanGame` | `amiga-input` |
| `amiga-routeview` (new, toolkit) | Map page site, verdict, JSON | `amiga-plan` |
| `fd-plan`, `fd-routeview` (new) | `PlanGame` for this game, oracle confirm, page data | above |

### 3.3 The decision function

`Game::step(&mut self, input: FrameInput)` is a function of the game data, the start and the inputs, as in `sf-core`. `Game: Clone` is the planner's snapshot. Rendering reads state only.

### 3.4 Data flow

Disks → `fd-tools` extract and unpack → `fd-assets` → `Game::new`. The app feeds key or pad input to `step`, draws, and presents through `amiga_app::video`. `fd-conform` feeds the same input to `Game` and the oracle, compares `LAYOUT` variables and pictures. `fd-plan` searches over `Game` clones, writes a route as an input script, and `fd-e2e` plays it as key presses.

### 3.5 Storage

Settings: TOML, atomic. Recordings: zip of `recording.txt` and `screenshot.png`. Sync snapshots: `~/.cache/amiga-toolkit/` or the game's cache root, keyed by SHA-1 of format, vAmiga build, Kickstart, disks, drives and route; a stale key rebuilds. Everything derived from disks stays in `build/` or the cache, never in git.

### 3.6 What moves to the toolkit

Moved parts must name no game (`tools/check_tier_names.py`). Superfrog keeps working through re-exports where that spares its tests edits, and its sync-cache keys stay byte-stable (`sync.rs:1819-1825` pins them). Moves are made only because this game is the second user, per root CLAUDE.md rule 2.

---

## 4. Testing

| Layer | Proves | Cannot prove |
|---|---|---|
| Unit, in `fd-core`, `fd-assets`, `amiga-plan` | Rules, in milliseconds | That the rules are the original's |
| Routine (`routine.rs` rig, 10 000 generated states) | A ported routine equals the 68k one | Whole-play behaviour |
| Case (`fd-conform`) | Engine equals the original across a run | Anything off the scripted run |
| Milestones | The engine alone reaches what the original reaches | Exact path |
| Route (`fd-plan`) | The level can be finished, on both sides | That a person enjoys it |
| End to end (`fd-e2e`) | The real app takes real keys and matches headless | Hardware-only effects |
| Drill | What nobody thought to assert | Taste decisions, which go to the owner |

A crucial feature adds its smoke test and `[[area]]` in `games/forest-dumb/smoke.toml`; every new test file gets a label. Slow tests are `#[ignore]`d for the nightly full set.

---

## 5. Environment — read this before running anything

| | |
|---|---|
| OS | macOS, arm64 |
| Language / runtime | Rust 1.99.0 pinned in the workspaces; python3 in a fresh venv |
| Toolchain | rustup, cmake; vAmiga at the pinned commit in `~/.cache/amiga-toolkit/vamiga/` |
| Installed here | Ghidra with its MCP server, Kickstart 1.3 at `~/amiga-roms/kick1.3.rom`, `xdftool`, the zips in the main checkout's `games/forest-dumb/` |
| **Deliberately absent** | No uncracked disk set; no WHDLoad or other emulator is part of testing |

**The test command.** The block's lines come from `.pir/settings.json` and are not re-derived. `tools/smoke.sh` runs the smoke set; it warns over 900 s and fails at 1 200 s (`tools/smoke.py`), and 900 s is the budget. The plan adds crates to existing workspaces but no install step, so they were not re-measured. To run this game's suite while working: `cd games/forest-dumb/engine && cargo test -q -p <crate>`; verbose by dropping `-q`. The suites run quiet with `NO_COLOR=1` and nothing here forces colour (`env` checked 2026-11-01); failures print in full.

**Smoke is nearly full.** Recent runs took 503 to 887 s of 900 s (`~/.cache/superfrog-re/smoke-times.tsv`). T01 makes room first. Whoever adds a slow test reads that file first; if crucial tests alone do not fit, ask the owner.

**Setup.** The venv lines leave the copy clean; `venv/` is git-ignored.

**Dependencies.** May be added: crates Superfrog's matching crate already uses (`winit`, `muda`, `libc`, `serde`, `toml`, `png`, `bincode`), permissive only and on the purity list if pure. Anything else asks the owner first. Nothing copyleft, because toolkit crates are linked into the engine and the licence follows the link.

**End to end.** Reuse `amiga-e2e` with a new `AppSpec` in `fd-e2e`; there is no rig task because the tooling exists. The script layer opens a real inactive window and needs no grants: its tests run in the test sets, about 500 frames in 10 s. The OS layer needs Accessibility, Screen Recording and an unlocked screen: it is `#[ignore]`d and enabled by `FD_E2E_OS=1`, in the worker's drill runs, as `sf-e2e/README.md` describes. The free backend is the game itself on local disks. Sizes: the app's default window, and scale 1 (320×256-based), its smallest.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| Playing a stretch the planner cannot find, if it stalls | Only a person's play supplies the missing route; used once on the first game |

### 5.2 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| Oracle frame cap | Each run names its own | No emulator run is unbounded |
| App `--timeout` watchdog | 120 s under `--script` | A scripted app kills its process group |
| App `--max-frames` | off | Scripted runs exit at a frame |
| App `--home`, `--bugs-dir` | Tests always pass temp dirs | A test never touches the owner's settings or `bugs/` |
| Planner budget | 30 min a run, memory reported | A search ends and reports |
| Scripted runs | No focus taken, no real pads | The owner's machine stays usable |
| Zips | Opened read-only | The irreplaceable input is never written |

Never run the unbounded version of any of these to find something out.

### 5.3 Outside the code — who acts

| Action | Command (exact, wrapped) | Bin | Why this bin | Way back | Expected cost | Login check |
|---|---|---|---|---|---|---|
| Fetch a crates.io dependency on the policy above | `cargo build` | `worker` | Public read-only fetch, recorded in `Cargo.lock` | Revert the lockfile | none | none |
| Fetch and build vAmiga at the pinned commit | `toolkit/tools/setup/oracle.sh` | `worker` | Public fetch into a local cache; already built here | `rm -rf ~/.cache/amiga-toolkit/vamiga` | none | none |

Credentials measured at plan time: none are needed. No cloud account, paid service, domain or device is touched.

---

## 6. Recovery

Nothing here can lock anyone out. A bad settings file: delete `~/Library/Application Support/Forest-Dumb-RE/settings.toml`. A stale snapshot: delete the sync cache. A Superfrog break from a move: revert that task's commit; T02's pins name it.

---

## 7. Decisions and rationale

All 2026-11-01 unless dated.

- One plan, split into tasks (owner, 2026-10-31).
- No prototype: every screen is the original's, so a mock settles nothing. The requirements were confirmed in words.
- Move the planner, page and app windows to the toolkit rather than copying (owner). A move costs about 1.5 to 2 times a copy, mostly checks, and later levels use one copy. T02 exists because nothing pinned routes across a refactor: determinism tests compare runs within one build.
- Toolkit `amiga-plan` takes the explorer path plus compass and map only as needed. `facts.rs` stays per game.
- `sf-e2e`'s `plan_routes.rs` is labelled smoke while its only test is `#[ignore]`d, so it only compiles there. T28 does not copy that: its short test really runs.
- Add `Amiga::picture_in` and leave `picture()` alone: Superfrog's `level_exit_fade` and `display::draw` rely on the frame-end window, and changing the default would invalidate traces. No vAmiga patch, so caches stay valid.
- Stretches first, freeze later (§2.2), because the freeze needs entry points that are readings today.
- Whether this game uses hardware sprites is unknown, so `objects.rs` is not moved in advance; T00 measures and T10 decides.
- Recording uses Start Level like Superfrog, not always-on (owner). The planner proposed always-on; the owner chose the same modal with a one-level selector, to be extended as options are discovered.
- Planner must finish the level in this plan (owner), and the finish is proved in the real app.
- Disks: known folder, no picker (owner).
- The map page has no drill task: it is a working tool the player never sees, so T19's worker drives it in headless Chrome (owner).
- `amiga-app` gets the windows; a game's `script.rs` stays split, core parser in the toolkit, game events per game, because Superfrog's script events name its levels and menus.

---

## 8. Explicitly out of scope

- Levels 2 to 6: one plan each, later (owner).
- Sound: a later plan after the levels (owner).
- Title screen, menus, `SCENA` screens: their own plan (owner).
- High score screen: no title to return to, and the original never saves it.
- The ending and the whole-game test: last plan (owner).
- Map editing: not asked for.
- Filters, widescreen, a disk picker in this app: unrelated to a playable level (owner).
- Start Level's lives, points and other fields: added when measured and needed (owner).
- Superfrog's review-noted `atn.py`, `sf-conform` overflow and symbol entries: noted in `plans/forest-dumb-start/`, not this plan's.
