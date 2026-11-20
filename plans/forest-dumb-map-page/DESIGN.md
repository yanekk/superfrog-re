---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - tools/smoke.sh
---

# Forest Dumb Forever, the map page — Design

Brief: [games/forest-dumb/docs/map-page-brief.md](../../games/forest-dumb/docs/map-page-brief.md). Requirements
settled with the owner on 2026-11-20 (§7).

## 1. Purpose

The owner keeps a map open in Chrome on a second screen while playing Forest Dumb in `fd-app`. The page
shows any of the six levels whole (tiles, every object of the object map in its place, the exit, the items
the exit needs) and, while the app plays, where the character is, the creatures in view, what has been
taken and how many items are left. It is a viewing tool: it never changes what the app plays.

### Success criteria

- `fd-routeview --all` writes one page, opened from disk, listing W1-1 to W6-1; each level's object counts
  equal its `_MapaBobowN` file's, and its exit cells and items needed equal the original's at level start.
- With the app playing any world, the open page shows the character's cell within a second, the items
  left, taken items gone, and switches to the world being played by itself.
- Game ▸ Show Map writes the page and opens it in Chrome.
- A Secrets layer marks each hidden feature whose kind was confirmed on the original.
- Nothing the app does changes because the page exists: its state hashes are the same with live writing on.

### Stance

The faithful bar binds what the page shows (positions, counts, kinds as the original's), not how it draws.
An object value nobody has identified is drawn as `other` with its number, never named by guess, because a
wrong name on a map the owner plays by is worse than an honest unknown.

---

## 2. Behaviour specification

### 2.1 Levels and objects

Six levels, `W1-1` to `W6-1` ([LEVEL-INDEX](../../games/forest-dumb/docs/LEVEL-INDEX.md)), read from their
files with no engine run: the tile map, tiles, palette (one for all worlds, render_levels.py) and object map,
over the used columns (240, W6-1 320). Every non-zero object value is drawn on the layer its group gives:
coins (`$20`-`$27`), creatures (`$01`-`$06`), bonuses (`$40`-`$5f`), water (`$28`, `$2a`), and one layer for
springs, vanishing blocks, flames and the exit. The groups come from `re/objects-W1-1.toml` plus
`re/objects-more.toml` (T01), which holds the values worlds 2 to 6 use and W1-1 lacks: today only `$34`,
the flame (counted 2026-11-20: 31, 10, 25 and 17 cells in W3-1 to W6-1). Reason: the code dispatches on the
value alone, the same in every world (docs/10), so one table names every world.

### 2.2 The exit and the items it needs

The exit is the `$ff` cells (2 by 3 in every world, counted 2026-11-20), labelled EXIT. The items the exit
needs is `items_left` as `level_init` leaves it, not the coin count: W1-1 holds 332 coins and needs 282
(docs/10). How the page gets that number without running the game is T00's question; whatever T00 finds,
the number is checked against the oracle for all six worlds (T02's full test).

### 2.3 Layers and the page

Each layer switches on and off; Secrets starts off so it does not spoil the map (owner, 2026-11-20). A level
with no route shows "Map only" where a route's verdict goes and has no scrubber. W1-1's route drawing with
`--script` stays as today (owner: routes for worlds 2 to 6 are out of scope, §8).

### 2.4 Secrets

A Secrets layer marks (1) hidden passages: cells drawn as wall that the character passes through, (2)
invisible floors: cells drawn empty that are solid, (3) any other hidden feature the code turns out to have
(a warp, a hidden room). A kind is drawn only after T07 confirmed it on the original; the cells are then
listed by the rule T07 writes down, computed from the level files. Reason: the owner asked for secrets but
nothing guessed. A first count (2026-11-20) found tiles that are solid in one place and passable in
another in every world (44 of 328 tiles in W1-1); many will be slopes or edges, which is why the rule needs
research and not a diff.

### 2.5 Live while playing

While the app runs, it writes `live.js` in the page's folder about four times a second (every 12th frame).
The page re-reads it by inserting a script tag every 250 ms (a page opened from disk cannot fetch; a script
tag reload of a file replaced by rename was measured to work in Chrome 157 on 2026-11-20, FINDINGS). It
shows:

- the character at its pixel position, as a dot, and its cell;
- each creature in view (the five creature slots) at its live position, because the game clears a
  creature's map cell while it is in view and writes it back when it leaves (docs/10), so copying the map
  alone would hide the creatures nearest the player;
- every object cell whose value differs from the level file's, redrawn with its current value (0: gone), so
  taken items disappear and a creature written back elsewhere moves;
- `items_left`, live, beside the items needed.

The live data is read from the game's memory at a frame's end (T06): the translated program's RAM, which
holds the truth with level 1's hand engine mounted too, since the seam writes back what it computes; or
`fd-core`'s `Game` state under `--engine hand`.

### 2.6 Following the game

When the world in `live.js` changes, the page switches to that level by itself; a level picked by hand
stays until the next change (owner). The page scrolls to keep the dot in view while "Follow me" is on; it is
on at load, any scroll or drag by hand turns it off, and clicking it turns it on again (owner).

### 2.7 When nothing is playing

Outside play (title, menus, the SCENA screen, a level's load: `irq_count` 0) the app writes
`playing: false` and the page shows no dot and "Not playing". When `live.js` is missing or its time stamp is
more than 3 s older than the page's clock, the page shows "App not running". An old position is never drawn
as current (owner).

### 2.8 Opening it from the app

Game ▸ Show Map writes the page fresh (so it never lags the build) on a thread off the game loop, then opens
it in Chrome (`open -a "Google Chrome" <index.html>`), whatever the default browser is (owner: Chrome is the
browser measured). If the page cannot be written, or Chrome is not installed, a plain message says which and
the game carries on. A scripted run never opens a browser: it prints `show-map: would open <path>` instead,
so tests cannot take the owner's screen.

### 2.9 Where the page lives

`games/forest-dumb/build/map/` of the main checkout (`fd_map::default_out`), found as fd-app finds its disks,
else `<home>/map/`. A scripted app run (`--script`) and every test write under their own home or temp folder,
never the owner's page. The page holds disk graphics, so it stays under `build/` and is never committed.
`fd-routeview --script` keeps writing `build/route-viz/` as today, because `fd-plan --page` and the level-1
docs name that path.

### 2.10 The unhappy paths

- `live.js` cannot be written (disk full, folder removed): the writer drops that frame, says so once on
  stderr, retries on the next, and the game is never blocked, because a viewing tool must not cost play.
- The page reads `live.js` mid-write: impossible, since the writer writes `live.js.tmp` and renames it.
- A world the page does not have (a future level id, a corrupt `world_index`): the page shows "Playing a
  level this map does not have" and no dot.
- Two pages open: both read the same file; nothing breaks.
- The app's second copy is refused already (one per home), so one writer per folder.
- A level file missing or corrupt when the page is written: the command exits 1 naming the file, as
  `fd-assets` reports it; Show Map shows that message.
- An object value in no table: drawn as `other` with its number (§1 stance), and T01's test names it.

---

## 3. Architecture

### 3.1 The boundary

```
pure:   fd-core::live (RAM or Game state -> Live), fd-map's level data (files' bytes -> MapLevel),
        fd-assets::secrets (Level -> cells), the items-needed reader (program bytes -> count, per T00)
shell:  fd-map's site writer, fd-routeview's command, fd-app's live writer thread and menu,
        amiga-routeview's file writers, amiga-browser (spawns Chrome)
```

The pure parts take bytes and return values; no clock (the live time stamp is the writer's, in the app),
no files. The existing guard is `fd-core`'s, whose crate must not do I/O (fd-core's own tests); `fd-map`'s
pure module gets a test scanning `src/level.rs` for `std::fs`, `std::time` and `std::net`. If that test
fails, move the code to the shell side; never relax the test. Everything pure is tested on made-up bytes in
milliseconds; every rule that leaks into the shell needs Chrome or the app to check.

### 3.2 Modules

- `fd-map` (new, game): `MapLevel` per world from `fd_assets` (tiles, palette, objects, exit, items needed,
  secrets), as an `amiga_routeview::LevelDump`; `write_site(out, files, routes)`; `default_out()`. Depends on
  `fd-assets` and `amiga-routeview` only, so `fd-app` may link it (`fd-app/tests/deps.rs`).
- `fd-routeview` (extended): `--level ID`, `--all`; with `--script` W1-1 is the route's level and the others
  map-only. Depends on `fd-map`.
- `fd-core::live` (new module): `Live` from RAM and from `Game`.
- `fd-app` (extended): the live writer thread, Game ▸ Show Map.
- `amiga-routeview` (toolkit, extended): map-only entries; `PageSpec.live`; `live::LiveFrame`, `write_live`.
- `amiga-browser` (toolkit, new, test tooling): headless Chrome over `--remote-debugging-pipe`.

### 3.3 The live frame

```
window.LIVE = { "t": <ms since epoch>, "playing": bool, "level": "W2-1" | null,
                "actor": [x, y] | null, "cell": [c, r] | null,
                "actors": [[x, y, "enemy"], ...], "cells": [[c, r, value], ...],
                "counters": { "items_left": n } };
```

The toolkit owns the shape and the page's reading of it; the game fills it. `actor` is in level pixels at the
spot the game's cell rule uses (`char_x + $e`, `char_y + 9`). `cells` holds only differences from the level
file, so it stays small.

### 3.4 Data flow

```
level files + program ─► fd-map ─► index.html, data/W?-1.js          (command, Show Map)
app frame end ─► RAM / Game ─► fd-core::live ─► channel ─► writer thread ─► live.js (rename)
page ─► every 250 ms <script src=live.js?t> ─► draw dot, actors, cells, counters, status, follow
```

The program thread copies only the bytes `Live` needs every 12th frame, never the megabyte the hash reads,
so play's timing is unchanged; the writer thread does all file I/O.

### 3.5 Storage

`live.js` is replaced whole by rename, so a crash leaves the last complete frame, which the page then
shows as "App not running" once its stamp is 3 s old. The page files are rewritten by each command run;
nothing else is kept.

---

## 4. Testing

- Pure: unit tests on made-up bytes (levels, RAM images, programs).
- Against the files: each level's dump against its `_MapaBobowN` counts (disks needed).
- Against the original (oracle, `fd-conform`): items needed and exit cells after each world's `level_init`
  (the `world-N-start` cases' way in), `$34` placed, each secret kind. About 35 s a world: full.
- End to end: the page in headless Chrome (`amiga-browser`) against made-up `live.js` frames, and the app's
  scripted runs (`fd-e2e`) writing `live.js` into their home.
- None of them can prove the page is pleasant on the owner's real second screen; the drill (T12) judges it at
  that size, worker-driven.

---

## 5. Environment

| | |
|---|---|
| OS | macOS (Darwin 25.5), Apple silicon |
| Rust | 1.99.0, pinned per workspace (`rust-toolchain.toml`) |
| Chrome | Google Chrome 157.0.8103.79 at `/Applications/Google Chrome.app` |
| Python | venv at the repo root (`requirements.txt`); `unpack.py` needs it (`machine68k`) |
| Node | 24.2.0 installed, not used by the project (owner chose Rust, §7) |
| Deliberately absent | Playwright, any npm package, Safari automation |

**The test command** is `tools/smoke.sh`, the repo's stored line (`.pir/settings.json`), unchanged. It ran
842 to 1112 s in its last runs (`~/.cache/superfrog-re/smoke-times.tsv`), at the 15-minute budget, so this
plan's oracle tests are full, and each smoke addition must finish in a few seconds; read that file before
labelling. `SMOKE_VERBOSE=1` shows each entry's output.

**Setup** is the stored venv lines. Tests read the owner's disks from the main checkout (`fd_tools::disk_dir`).

**Dependencies.** `serde_json` (cached in `~/.cargo/registry`, 1.0.150) may be added to `amiga-browser`
only, which only tests use (owner, 2026-11-20). Nothing else new. `fd-app` must keep passing `tests/deps.rs`.

**End to end.** No browser tooling existed; `amiga-browser` (T03) is the rig: Chrome `--headless=new`, a
temp `--user-data-dir`, `--remote-debugging-pipe`, a viewport per test, requests other than `file://`
failed and counted (`Fetch` domain), a hard time limit, the process group killed on drop. Sizes:
1280×800 (laptop), 1920×1080 (the second screen), 800×600 (smallest). The free backend is a test writing
`live.js` frames, or the app's scripted run with `--home`. `fd-e2e` drives the app as today.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| Show Map opening Chrome on the owner's real desktop, beside the app window | it takes the owner's screen; tests use the scripted `would open` path |

### 5.2 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| `amiga-browser` time limit | 30 s a session | Chrome's group killed, test fails |
| headless, temp profile | always | never the owner's Chrome or profile |
| non-`file://` requests | blocked | a page cannot reach the network |
| scripted app run | `--script` | live written to `<home>/map`, Show Map prints instead of opening |
| `fd-app --timeout`, `--max-frames` | as today | bound every app run |

### 5.3 Outside the code — who acts

| Action | Command | Bin | Why this bin | Way back | Cost | Login check |
|---|---|---|---|---|---|---|
| Add `serde_json` to `amiga-browser` | `cargo build` (cached crate) | worker | local, a Cargo.lock line | revert the commit | none | none |

---

## 7. Decisions and rationale

All 2026-11-20 with the owner unless marked.

- Live marker in this plan (the brief proposed a follow-up); the owner chose it.
- Page switches to the level being played; a hand pick holds until the next change.
- Nothing playing: no dot, "Not playing" or "App not running"; never a stale position.
- Taken items vanish; creatures in view drawn live (else they vanish while near the player).
- Follow me on by default, off on a hand scroll, back on by a click.
- Game ▸ Show Map plus the command; always Chrome, the browser measured.
- Routes stay W1-1-only on the hand engine (brief's proposal).
- Secrets: owner asked for them; layer off by default; each kind confirmed on the original.
- Chrome driver in Rust with `serde_json`, test-only (over hand parsing or Node).
- No mock: the page exists and the owner has used it; the additions are controls on it (planner).
- A new `fd-map` crate rather than extending `fd-routeview`, because `fd-app` may not link `fd-routeview`'s
  `fd-conform`/`amiga-conform` (planner).
- The live shape and reader are in the toolkit (`amiga-routeview`), game-free, so Superfrog's page could use
  them; its page is unchanged while `PageSpec.live` is `None` (planner).
- About four writes a second: smooth enough for a dot and cheap (planner, inside the owner's "a few times").

## 8. Explicitly out of scope

- Routes for worlds 2 to 6: needs a replay on the translated program, the level plans' need.
- Editing a map, a route planner for worlds 2 to 6: not asked.
- Widescreen: the app has none for this game.
- Safari and other browsers: Chrome was measured and chosen; another would need its own check.
- Live data on Superfrog's page: the toolkit supports it, no game uses it but this one.
