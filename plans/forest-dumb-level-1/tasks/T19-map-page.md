# T19 — map-page

**Phase:** 4 · **Depends on:** T11, T18 · **Weight:** light

## Goal
Level 1's HTML map with the character's path from a scripted run.
## Design sections this implements
§2.10.
## Reuses
`amiga-routeview`, `fd-assets`, `sf-routeview/src/dump.rs` as pattern.
## Files
`games/forest-dumb/engine/crates/fd-routeview/`, `games/forest-dumb/smoke.toml`, `games/forest-dumb/CLAUDE.md`.
## Interface
```
fd-routeview --script FILE [--out build/route-viz]
```
## Tests
- [ ] Page generated from a scripted run: manifest, level id, track length and end cell match the replay.
- [ ] Output path is under `build/` and git-ignored.
- [ ] Missing script names the path.
## End to end (the worker drives this)
- suite: test above plus a worker's look · sizes: 1280×800
- [ ] Open from disk in a headless browser if available, else judged from the generated data and a saved image: map and track visible, no external requests.
## Done when
- [ ] A test generates the page and checks its contents.
- [ ] Nothing derived from disks is committed.
