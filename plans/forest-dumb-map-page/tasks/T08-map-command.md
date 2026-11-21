# T08 — map-command

**Phase:** 2 · **Depends on:** T02, T05 · **Weight:** medium

## Goal

One command writes the six-level page with live mode on, so the owner can open it beside the app.
`fd-routeview` gains `--level` and `--all`; W1-1's route drawing with `--script` stays.

## Design sections this implements

DESIGN §2.1 to §2.3, §2.9.

## Files

- `games/forest-dumb/engine/crates/fd-map/src/lib.rs`: `spec()` with `live: Some(..)`, `write_site`.
- `games/forest-dumb/engine/crates/fd-routeview/src/main.rs`, `src/lib.rs`, `README.md`, `Cargo.toml`
  (`amiga-browser` as a dev-dependency).
- `games/forest-dumb/engine/crates/fd-routeview/tests/levels_chrome.rs` (new); `games/forest-dumb/smoke.toml`.
- `games/forest-dumb/CLAUDE.md` ("Seeing a run on the map").

## Interface

```
fd-routeview --all [--out DIR]                 # six levels, map only, default fd_map::default_out()
fd-routeview --level W2-1 [--out DIR]          # one level, map only; with --script it is plan
                                               # forest-dumb-level-2's route replay (DESIGN §3.2)
fd-routeview --script FILE [--out DIR] [--all] # as today; with --all the other five levels map only
```

```rust
pub fn write_site(out: &Path, files: &fd_assets::Files, program: &[u8],
                  route: Option<&dyn amiga_routeview::LevelDump>) -> Result<PathBuf, String>;
```

Exit codes as today: 2 for bad arguments (an unknown level id names the six), 1 for missing disks or a write
failure. Prints `<page>: 6 levels`.

## Tests

- [ ] `--all` into a temp dir: six `data/W?-1.js`; each level's object counts equal its file's (disks).
- [ ] `--level W9-1` exits 2 naming the six ids; `--script` alone writes W1-1 as before (`tests/page.rs` green).
- [ ] `fd-plan --page` unchanged (its tests green).
- [ ] If plan forest-dumb-level-2's `--level W2-1 --script` is merged, its tests stay green beside these.

## End to end (the worker drives this)

- suite: `fd-routeview/tests/levels_chrome.rs` on `amiga-browser` · sizes: 1280×800, 1920×1080, 800×600
- [ ] the switcher lists W1-1 to W6-1; each opens with its map, "Map only", EXIT drawn at its `$ff` cells
- [ ] the items-needed line equals `fd_map::items_needed` per world
- [ ] W6-1 scrolls to column 319
- [ ] each layer toggles; a flame shows on the springs/blocks/exit layer in W3-1
- [ ] no `live.js` yet → "App not running"; no request other than `file://`

## Done when

- [ ] The command and the Chrome test pass and are labelled per DESIGN §5; `fd-routeview`'s README and the game's
  CLAUDE.md say how to write the six-level page.
