# Superfrog RE — toward a map editor

Reverse-engineering the disk, packing, and level formats of **Superfrog**
(Team 17, 1993, Amiga) with the goal of building a **map editor**.

This project works on the four Superfrog floppies as
`Superfrog (1993)(Team 17)(Disk N of 4)[cr CSL].zip` (each containing one 880 KB
`.adf`). **Those disk images are copyrighted and are NOT included** (git-ignored) —
supply your own copies in the repo root before running the tools.

> **Continuing / fresh start:** run the [Quickstart](#quickstart). `build/` (adf,
> decompressed chunks, rendered PNGs) is git-ignored and must be regenerated with
> `extract_disks.py` → `unpack.py` (with your own disk zips in place) before the
> render/secret/annotate tools will work. The reverse-engineering findings are in
> [`docs/`](docs/).

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
| **All 29 levels rendered to PNG** (`build/levels/`, + montage) | ✅ |
| Collision flags decoded; **secret passages (fake walls) found** — 144 across the game | ✅ |
| **Rust engine plays the whole game** natively on macOS, no emulator: intro, title, every level, bosses, slot machine, bonus levels, end sequence, music | ✅ |
| Engine faithful to the original, judged headless on the oracle: every conformance case (`sf-conform all`), and the whole game from power-on to the end sequence through the app (state, picture, milestones) | ✅ |
| The app's sound note for note: a note restarting a channel mid-sample is lost, as the engine's Paula writes have no beam position ([T213](plans/rust-engine/tasks/T213-paula-write-timing.md)) | ⬜ next |
| App: menus, keyboard and gamepad controls, save states, CRT and widescreen, playing a level from a `.sflevel` file | ✅ |
| Bug recorder: start any level with chosen values, record play, save a bug zip an agent replays and compares with the original ([`docs/BUG-FIX.md`](docs/BUG-FIX.md)) | ✅ |
| Map editor (view + edit + repack `ATN!` + rebuild `.adf`) | ⬜ next |

**29 levels** found (all `300 × 84` blocks), stored as `ATN!`-packed `T7MP` files.

## Quickstart

```bash
python3 -m venv venv && . venv/bin/activate
pip install -r requirements.txt

python tools/extract_disks.py      # zips -> build/adf/*.adf
python tools/unpack.py             # -> build/decomp/*.bin  (all chunks, classified)
python tools/t7mp.py build/decomp/d1_c21_level_53588.bin   # inspect one level
python tools/render_levels.py      # -> build/levels/*.png  (all 29 levels + montage)
```

**29 rendered levels** land in `build/levels/` (full-res `4800×1344` PNGs, plus
`_montage.png`). L1–L5 are colour-accurate; L6 / world-map palettes are plausible
(see [`docs/06`](docs/06-graphics-and-rendering.md)).

`build/` is git-ignored and fully regenerated from the disk images by the tools.

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

Full analysis in [`docs/`](docs/) (read in order):
- [`01-disk-format.md`](docs/01-disk-format.md) — trackloader disks, `ATN!` container
- [`02-boot-chain.md`](docs/02-boot-chain.md) — bootblock → stage-2 → trackloader
- [`03-atn-codec.md`](docs/03-atn-codec.md) — the `ATN!` compression & how we decode it
- [`04-t7mp-level-format.md`](docs/04-t7mp-level-format.md) — the level file format (+ `BODY` cell split)
- [`05-chunk-inventory.txt`](docs/05-chunk-inventory.txt) — every chunk on every disk
- [`06-graphics-and-rendering.md`](docs/06-graphics-and-rendering.md) — tilesets, palettes (`0x11300`), world→tileset mapping
- [`07-secret-passages.md`](docs/07-secret-passages.md) — collision flags, fake-wall (flag 38) secret passages
- [`LEVEL-INDEX.md`](docs/LEVEL-INDEX.md) — level ids + tile addressing (`<level-id>:<col>,<row>`)

## Play it

The Rust engine in [`engine/`](engine/) plays Superfrog on a Mac from your own four disk images.

```bash
tools/setup/install-rust.sh               # once: rustup via Homebrew + the pinned toolchain
cd engine && cargo run --release -p sf-app
```

- **First run:** the app asks for the four disks. Choose the four `Superfrog (1993)(Team 17)(Disk N
  of 4)...zip` files (or their `.adf`s); they are read once, checked, and decoded into a cache. The
  disks are opened read-only. If one is missing or not the expected dump, the message says which.
  Later launches start straight from the cache.
- **Controls:** arrow keys move, Space (or either Ctrl) fires; Game ▸ Controls… rebinds them and a
  gamepad. Game ▸ Continue from World N starts at the furthest world reached; Save State and Load
  State are under Game; View has the scale, fullscreen, CRT filter and widescreen.
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
- **The bug zip:** Save writes `bugs/bug-<level>-<date>.zip` in the main checkout (git-ignored, never
  committed) and copies its path. It holds `recording.txt` (the start, every frame's input, your note)
  and `screenshot.png`.
- **Hand it to an agent:** in `pir`, `@superfrog-re/single fix the bug in bugs/<zip>`. The agent follows
  [`docs/BUG-FIX.md`](docs/BUG-FIX.md): it replays the recording headless, plays the same start and
  inputs on the original, and either fixes the engine with a new conformance case that fails before
  the fix and passes after, or tells you in plain words that the original does the same, with both
  pictures. Limit today: on a level where the original slows down (W1-1 among them) it stops
  following your recording part-way, so a bug late in a long recording may not be comparable yet;
  the agent says so rather than guess.
- **Watch it back:** `superfrog --replay <zip>` (`cargo run --release -p sf-app -- --replay <zip>`)
  plays a recording in the window, watch-only, for agents and tests; `sf-conform bug replay`,
  `bug compare` and `bug case` are the headless tools ([`engine/conform/README.md`](engine/conform/README.md)).

## Rust engine

A faithful Rust reimplementation of the game lives in [`engine/`](engine/): every mechanic rewritten
from the original's 68000 code and judged against the original running on an emulator oracle. The
plan, design and progress are in [`plans/rust-engine/`](plans/rust-engine/). Setup and tests:

```bash
tools/setup/install-rust.sh                 # rustup via Homebrew + the pinned toolchain (idempotent)
cd engine && cargo test -q --workspace      # includes the purity test (DESIGN §3.1)
venv/bin/python tools/check_no_disk_data.py # no disk bytes in the repo (DESIGN §2.3)
```

Both tests read the disks from `SUPERFROG_DISKS`, else from the main checkout root.

The oracle (`tools/setup/oracle.sh`, vAmiga built outside the repo, plus a Kickstart 1.3 ROM) runs
the original for the conformance checks: `cd engine && cargo run -q --release -p sf-conform -- all`
judges every case in [`engine/conform/`](engine/conform/README.md) (about 1¾ h), and
[`engine/crates/sf-e2e`](engine/crates/sf-e2e/README.md) drives the real app end to end, the whole
game included.

## Tools

| File | Purpose |
|------|---------|
| `tools/extract_disks.py` | unzip `.adf` images to `build/adf/` |
| `tools/atn.py` | `ATN!` container carver + emulation-based decompressor |
| `tools/unpack.py` | carve + decompress + classify every chunk |
| `tools/t7mp.py` | parse a `T7MP` level file (tags + tilemap/flags grid) |
| `tools/render_levels.py` | render all levels to PNG (tileset + palette + tilemap) |
| `tools/find_secrets.py` | find fake-wall secret passages, overlay + `SECRETS.md` |
| `tools/export_annotated.py` | per-level PNG with a coordinate grid + level id (tile addressing) |
| `tools/check_no_disk_data.py` | fail if any repo file carries a 128-byte run of disk data |
| `tools/template_sweep.py` | every object template a level's map can spawn, against those the engine ports |
