# Superfrog RE — toward a map editor

Reverse-engineering the disk, packing, and level formats of **Superfrog**
(Team 17, 1993, Amiga) with the goal of building a **map editor**.

This project works on the four Superfrog floppies as
`Superfrog (1993)(Team 17)(Disk N of 4)[cr CSL].zip` (each containing one 880 KB
`.adf`). **Those disk images are copyrighted and are NOT included** (git-ignored) —
supply your own copies in `games/superfrog/` before running the tools (the repo root also works).

> **Continuing / fresh start:** run the [Quickstart](#quickstart). `games/superfrog/build/` (adf,
> decompressed chunks, rendered PNGs) is git-ignored and must be regenerated with
> `extract_disks.py` → `unpack.py` (with your own disk zips in place) before the
> render/secret/annotate tools will work. The reverse-engineering findings are in
> [`games/superfrog/docs/`](games/superfrog/docs/).

## Repository layout

One repository, three projects: the Amiga toolkit and the games built on it. Knowledge lives in three
tiers, each at the lowest level where it is true (see [CLAUDE.md](CLAUDE.md)).

| Where | What |
|---|---|
| `toolkit/` | Tier 2: the game-free Rust workspace, the reverse-engineering tools, their notes |
| `games/superfrog/` | Tier 3: the engine, tools, docs, symbols and disk home of Superfrog |
| `knowledge/` | Tier 1: how the owner works and decides, and the faithful bar |
| `plans/` | Every plan, sealed as history once finished |
| `tools/` | Shared: `tools/smoke.sh` and `tools/full.sh`, the two test commands (see [Tests](#tests)); `tools/test.sh`, which runs what a branch's changes need (`PIR_TEST_SCOPE=all` runs everything); and the name check |

## Tests

```bash
tools/smoke.sh               # 15 minutes at most: what must work, in the toolkit and every game
tools/smoke.sh --check-only  # only the labels: every test file labelled, every crucial area covered
tools/full.sh                # every test, the slow ones included
```

pir runs `tools/smoke.sh` at the end of every plan and for every single run. A nightly schedule runs
`tools/full.sh`, and pir never does. Each project's `smoke.toml` says which of its test files are smoke and
which crucial areas they cover. `tools/smoke.sh` fails if a test file has no label. A break only the full set
can see shows in the next nightly run, possibly after a merge. Full leaves out the tests that post real
screen input unless `SF_E2E_OS=1` is set.

## Status

| Milestone | State |
|-----------|-------|
| Disk format identified (custom trackloader, no AmigaDOS FS) | ✅ |
| `ATN!` container format cracked | ✅ |
| `ATN!` decompressor working (via 68k emulation of the game's own routine) | ✅ 103/106 chunks |
| `ATN!` decompressor ported to Rust (`sf-disk`), byte-identical to the emulation; stragglers decode with the next sector | ✅ 106/106 |
| All chunks extracted & classified (levels / gfx / exe / anim / text) | ✅ |
| **T7MP level format decoded** (dimensions, palettes, tilemap `BODY`) | ✅ |
| `BODY` cell split: tile index = `cell & 0x3FF`, flags = `cell >> 10` | ✅ |
| Tileset graphics decoded (840× `16×16` 5bpp tiles per world) + palettes | ✅ |
| **All 29 levels rendered to PNG** (`games/superfrog/build/levels/`, + montage) | ✅ |
| Collision flags decoded; **secret passages (fake walls) found** — 144 across the game | ✅ |
| **Rust engine plays the whole game** natively on macOS, no emulator: intro, title, every level, bosses, slot machine, bonus levels, end sequence, music | ✅ |
| Engine faithful to the original, judged headless on the oracle: every conformance case (`sf-conform all`), and the whole game from power-on to the end sequence through the app (state, picture, milestones) | ✅ |
| The app's sound note for note: a note restarting a channel mid-sample is lost, as the engine's Paula writes have no beam position ([T213](plans/rust-engine/tasks/T213-paula-write-timing.md)) | ⬜ next |
| App: menus, keyboard and gamepad controls, save states, CRT filters and widescreen, playing a level from a `.sflevel` file | ✅ |
| Bug recorder: start any level with chosen values, record play, save a bug zip an agent replays and compares with the original ([`games/superfrog/docs/BUG-FIX.md`](games/superfrog/docs/BUG-FIX.md)) | ✅ |
| Engine architecture documented: every file classified as Amiga imitation, game logic, copied-in data or shell, rated for reuse in another Amiga game, with a recommended native rewrite order ([`games/superfrog/docs/architecture/`](games/superfrog/docs/architecture/README.md)) | ✅ |
| [Amiga toolkit](#the-amiga-toolkit): the engine's game-free parts moved into `toolkit/`, a workspace with no Superfrog code or names that the engine uses by path; Superfrog's conformance unchanged | ✅ |
| Map editor (view + edit + repack `ATN!` + rebuild `.adf`) | ⬜ next |

**29 levels** found (all `300 × 84` blocks), stored as `ATN!`-packed `T7MP` files.

## Quickstart

```bash
python3 -m venv venv && . venv/bin/activate
pip install -r requirements.txt

python games/superfrog/tools/extract_disks.py      # zips -> games/superfrog/build/adf/*.adf
python games/superfrog/tools/unpack.py             # -> games/superfrog/build/decomp/*.bin  (all chunks, classified)
python games/superfrog/tools/t7mp.py games/superfrog/build/decomp/d1_c21_level_53588.bin   # inspect one level
python games/superfrog/tools/render_levels.py      # -> games/superfrog/build/levels/*.png  (all 29 levels + montage)
```

**29 rendered levels** land in `games/superfrog/build/levels/` (full-res `4800×1344` PNGs, plus
`_montage.png`). L1–L5 are colour-accurate; L6 / world-map palettes are plausible
(see [`games/superfrog/docs/06`](games/superfrog/docs/06-graphics-and-rendering.md)).

`games/superfrog/build/` is git-ignored and fully regenerated from the disk images by the tools.

## How extraction works (short version)

1. Each disk is a back-to-back sequence of sector-aligned **`ATN!` chunks**
   (`magic`, `decomp_size`, `comp_size`, compressed data + trailer).
2. `ATN!` is a custom cruncher not handled by public tools. The tools **run the
   game's own decompressor** (68k routine at disk-1 offset `0x400+0x2da`, found by
   its `cmpi.l #$41544e21,(a0)+` magic check) on a Musashi CPU core
   (`machine68k`) — exact by construction. The engine uses a Rust port
   (`sf-disk/src/atn.rs`) tested byte-for-byte against it.
3. Decompressed level files are **`T7MP`** — a flat `[tag][u32 len][data]`
   format whose `BODY` tag is an `XBLK × YBLK` grid of 16-bit tile indices.

Full analysis in [`games/superfrog/docs/`](games/superfrog/docs/) (read in order):
- [`01-disk-format.md`](games/superfrog/docs/01-disk-format.md) — trackloader disks, `ATN!` container
- [`02-boot-chain.md`](games/superfrog/docs/02-boot-chain.md) — bootblock → stage-2 → trackloader
- [`03-atn-codec.md`](games/superfrog/docs/03-atn-codec.md) — the `ATN!` compression & how we decode it
- [`04-t7mp-level-format.md`](games/superfrog/docs/04-t7mp-level-format.md) — the level file format (+ `BODY` cell split)
- [`05-chunk-inventory.txt`](games/superfrog/docs/05-chunk-inventory.txt) — every chunk on every disk
- [`06-graphics-and-rendering.md`](games/superfrog/docs/06-graphics-and-rendering.md) — tilesets, palettes (`0x11300`), world→tileset mapping
- [`07-secret-passages.md`](games/superfrog/docs/07-secret-passages.md) — collision flags, fake-wall (flag 38) secret passages
- [`LEVEL-INDEX.md`](games/superfrog/docs/LEVEL-INDEX.md) — level ids + tile addressing (`<level-id>:<col>,<row>`)
- [`architecture/`](games/superfrog/docs/architecture/README.md) — how the Rust engine is built: which parts imitate the Amiga, which are the game's rules, which are data copied from the disks; reuse per part and a suggested order for a native rewrite

## Play it

The Rust engine in [`games/superfrog/engine/`](games/superfrog/engine/) plays Superfrog on a Mac from your own four disk images.

```bash
toolkit/tools/setup/install-rust.sh               # once: rustup via Homebrew + the pinned toolchain
cd games/superfrog/engine && cargo run --release -p sf-app
```

- **First run:** the app asks for the four disks. Choose the four `Superfrog (1993)(Team 17)(Disk N
  of 4)...zip` files (or their `.adf`s); they are read once, checked, and decoded into a cache. The
  disks are opened read-only. If one is missing or not the expected dump, the message says which.
  Later launches start straight from the cache.
- **Controls:** arrow keys move, Space (or either Ctrl) fires; Game ▸ Controls… rebinds them and a
  gamepad. Game ▸ Continue from World N starts at the furthest world reached; Save State and Load
  State are under Game; View has the scale, fullscreen, widescreen and View ▸ Filter: None,
  Basic (the app's own light scanlines), or the RetroArch CRT shaders Lottes, Geom (both with a
  curved tube) and Easymode, at their stock settings, to compare while playing.
- **Widescreen** (View ▸ Widescreen, remembered) widens a level to 16:9 and plays it as a wider
  game: enemies, coins, platforms and the frog are alive and drawn in the 65 extra columns each
  side, as they would be at the screen's own edges, and the camera stops 65 pixels sooner at a
  level's ends so nothing past the map shows. It can be switched mid-level and takes effect at
  once. This is our game, not the original's: with it off the game is the original, and a bug
  recorded in widescreen has no original to compare with
  ([`games/superfrog/docs/12-display.md`](games/superfrog/docs/12-display.md#widescreen), [`games/superfrog/docs/BUG-FIX.md`](games/superfrog/docs/BUG-FIX.md)).
  Screens outside a level (title, map, slot machine) keep black sides.
- **Where data lives:** `~/Library/Application Support/Superfrog-RE/` holds the decoded cache,
  `settings.toml`, `progress.toml`, save states and the remembered disk paths. Delete the folder to
  start over; the next launch asks for the disks again. `--home DIR` (or `SUPERFROG_HOME`) uses
  another folder. Nothing from the disks is ever written into this repository.
- **Levels from files:** File ▸ Export Level writes a disk level as a `.sflevel` file, and File ▸
  Play Level File… plays one in place of the level it replaces: the map editor's target.

## Report a bug

Seen something the original would not do? Record it and hand the recording to an agent; you do not
describe it further or re-test the fix.

- **Start Level:** Game ▸ Start Level… starts any of the 26 levels with the lives (3, 5, 7), points,
  energy, difficulty (easy, normal), spud and wings you pick, the recorder on by default. A game
  started this way never changes "Continue from World N".
- **Record:** while recording, a bar under the picture shows the time and a STOP button. The
  recording runs on through level ends, the tally, the slot machine and game over. STOP (or ⌘., or
  Game ▸ Stop Recording) pauses the game and opens the save window: the screenshot, where you
  stopped, and a one-line note. Load State, Reset, Quit and the other actions that would change the
  game ask first: Save…, Discard or Cancel.
- **The bug zip:** Save writes `games/superfrog/bugs/bug-<level>-<date>.zip` in the main checkout (git-ignored, never
  committed) and copies its path. It holds `recording.txt` (the start, every frame's input, your note)
  and `screenshot.png`.
- **Hand it to an agent:** in `pir`, `@superfrog-re/single fix the bug in games/superfrog/bugs/<zip>`. The agent follows
  [`games/superfrog/docs/BUG-FIX.md`](games/superfrog/docs/BUG-FIX.md): it replays the recording headless, plays the same start and
  inputs on the original, and either fixes the engine with a new conformance case that fails before
  the fix and passes after, or tells you in plain words that the original does the same, with both
  pictures. Limit today: on a level where the original slows down (W1-1 among them) it stops
  following your recording part-way, so a bug late in a long recording may not be comparable yet;
  the agent says so rather than guess.
- **Watch it back:** `superfrog --replay <zip>` (`cargo run --release -p sf-app -- --replay <zip>`)
  plays a recording in the window, watch-only, for agents and tests, in widescreen where it was
  recorded in widescreen whatever your setting (View ▸ Widescreen is greyed out meanwhile); `sf-conform bug replay`,
  `bug compare` and `bug case` are the headless tools ([`games/superfrog/engine/conform/README.md`](games/superfrog/engine/conform/README.md)).

## Rust engine

A faithful Rust reimplementation of the game lives in [`games/superfrog/engine/`](games/superfrog/engine/): every mechanic rewritten
from the original's 68000 code and judged against the original running on an emulator oracle. The
plan, design and progress are in [`plans/rust-engine/`](plans/rust-engine/). Setup and tests:

```bash
toolkit/tools/setup/install-rust.sh                 # rustup via Homebrew + the pinned toolchain (idempotent)
cd games/superfrog/engine && cargo test -q --workspace      # includes the purity test (DESIGN §3.1)
venv/bin/python games/superfrog/tools/check_no_disk_data.py # no disk bytes in the repo (DESIGN §2.3)
```

Both tests read the disks from `SUPERFROG_DISKS`, else from the main checkout root.

The oracle (`toolkit/tools/setup/oracle.sh`, vAmiga built outside the repo, plus a Kickstart 1.3 ROM) runs
the original for the conformance checks: `cd games/superfrog/engine && cargo run -q --release -p sf-conform -- all`
judges every case in [`games/superfrog/engine/conform/`](games/superfrog/engine/conform/README.md) (about 1¾ h), and
[`games/superfrog/engine/crates/sf-e2e`](games/superfrog/engine/crates/sf-e2e/README.md) drives the real app end to end, the whole
game included.

## The Amiga toolkit

[`toolkit/`](toolkit/README.md) holds the parts of the engine that carry to another Amiga game, with
no Superfrog code, data or names in it: the chips and the beam clock (`amiga-clock`, `amiga-chips`
with a display piece that draws a frame from chip memory, `amiga-paula`), input, disk formats and
the recorder's game-free half (`amiga-input`, `amiga-disk`, `amiga-record`), the app's game-free
parts (`amiga-app`), and the test tooling (`amiga-oracle`, the vAmiga reference; `amiga-conform`,
the comparison harness's game-free half; `amiga-e2e`, the app driver). The engine uses these crates
by path, so a change to the toolkit and its use land together. Superfrog's own rules, data and
menus stay in `games/superfrog/engine/`.

```bash
cd toolkit && cargo test -q --workspace   # its own workspace; also checks no sf-* dependency
cd toolkit && cargo run -q -p dry-fit     # one made-up frame from the chips, clock and Paula crates alone
```

The dry fit is the proof the toolkit is enough to start game two on: a blit, a copper list, a drawn
picture, a frame of the clock and a Paula note, each asserted by a test, built from toolkit crates
only. A game now supplies its own frame length, and `amiga-disk` reads AmigaDOS floppies.

Game two, Forest Dumb Forever, has started by the toolkit's [playbook](toolkit/PLAYBOOK.md): its disks
are opened and documented, and its [first conformance case](games/forest-dumb/engine/conform/README.md)
compares one variable of our code with the original on the emulator, frame by frame, at the idle title.
The game itself is not ported yet.

## Planning a route

`sf-plan` finds an input script that plays a level to its exit, for a test case, a whole-game
chain leg or a level you changed. It is the one level-routing tool: the older per-level planners
were retired, and only `plan_bonus` remains, for the bonus level. It explores the level on the Rust engine, then plays what it
found on the original (the emulator oracle) and repairs any stretch where the two part, so a route
it writes is one the original agrees with. One command, no per-level options:

```bash
cd games/superfrog/engine && cargo run --release -p sf-plan -- W2-1 --out route.txt
```

- **A guide path** may steer it: `--guide` takes a committed route (it is played once and the frog's
  path is followed) or a waypoint file, one `col,row` per line in the tile addressing above
  (`W1-1:94,51` is `94,51`). A guide is not a hint: it is allowed everywhere, the benchmark guides
  each level by its committed route.
- **It writes what it got** in the route's own timing (`# timing: original` first line). Exit 0: the
  route reaches the exit; 1: the time (`--budget-min`, 30) or the frames ran out, and the best
  partial route is written with a one-line reason (coins still needed, how close it got); 2: bad
  arguments or a guide it cannot read. `--follow old.txt` repairs an old route that no longer plays;
  `--snap` starts from a whole-game leg; `--engine-only` skips the original (for tuning).
- **Hints are a last resort:** `--via c,r` and `--avoid c0,r0,c1,r1` push it through a spot it
  cannot find on its own, and every run using them is logged to
  [`games/superfrog/engine/conform/plan-hints.log`](games/superfrog/engine/conform/plan-hints.log) as a planner gap to fix.

How it works, and the benchmark against the older tools (`sf-bench`), is in
[`plans/smart-planner/`](plans/smart-planner/DESIGN.md). It completes most levels from every start,
not all: the levels it still fails are listed in
[`bench/failing-levels.md`](plans/smart-planner/bench/failing-levels.md).

**Seeing a route:** `sf-routeview` replays routes on the engine and draws them over the level
map, one page at `games/superfrog/build/route-viz/index.html`: the frog's path, every coin, collectible and enemy,
the events along the way (coins, kills, hits, jumps) and a timeline to play it back. A switcher on
top lists the routes, marked failed (the planner failed, or the route reaches no exit), faults (a
heart or life lost, over +25% of the reference route, a plan that used half its time budget, or a
route timed for the original, which may drift on replay) or clean. A failed route also shows
where it stops making progress.

```bash
cd games/superfrog/engine && cargo run --release -p sf-routeview -- --planner        # every planner route, with verdicts
cd games/superfrog/engine && cargo run --release -p sf-routeview -- my-route.txt --guide waypoints.txt
```

`sf-bench` rebuilds the page at the end of every guided set-C run of the new planner (`--no-view`
skips it). Only app-timed routes (`# timing: app`) replay true. The page holds the disks' graphics,
so it stays in `games/superfrog/build/`.

**When the planner gets stuck, a person plays the hard part.** If a level finds no exit on two
seeds, or stalls at the same spot twice, the session shows the stall on the route page and asks
you to play that stretch. It hands you one command: the app races (silently, keys locked) through
a route to just before the stall, pauses, and records your play from there. You save the
recording with ⌘. as for a bug report; the planner then follows it in order and plans again.

```bash
python3 games/superfrog/tools/handover.py route.txt W6-4 263,66          # writes the app script, prints your command
python3 games/superfrog/tools/recording_guide.py games/superfrog/bugs/bug-W6-4-….zip     # your recording, as the planner's guide
cd games/superfrog/engine && cargo run --release -p sf-plan -- W6-4 --guide-ordered --guide bug-W6-4-…-guide.txt
```

The script presses keys through your Controls mapping, so regenerate it after remapping.

## Tools

| File | Purpose |
|------|---------|
| `games/superfrog/tools/extract_disks.py` | unzip `.adf` images to `games/superfrog/build/adf/` |
| `games/superfrog/tools/atn.py` | `ATN!` container carver + emulation-based decompressor |
| `games/superfrog/tools/unpack.py` | carve + decompress + classify every chunk |
| `games/superfrog/tools/t7mp.py` | parse a `T7MP` level file (tags + tilemap/flags grid) |
| `games/superfrog/tools/render_levels.py` | render all levels to PNG (tileset + palette + tilemap) |
| `games/superfrog/tools/find_secrets.py` | find fake-wall secret passages, overlay + `games/superfrog/SECRETS.md` |
| `games/superfrog/tools/export_annotated.py` | per-level PNG with a coordinate grid + level id (tile addressing) |
| `games/superfrog/tools/check_no_disk_data.py` | fail if any repo file carries a 128-byte run of disk data |
| `games/superfrog/tools/template_sweep.py` | every object template a level's map can spawn, against those the engine ports |
