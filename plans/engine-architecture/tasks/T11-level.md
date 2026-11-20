# T11 — level

**Phase:** 2 · **Depends on:** T01 · **Weight:** heavy

## Goal

Document a level as the engine plays it: loading and composition, spawning, scrolling and the camera, collision against the `BODY` flags, objects, death, the exit and tally, the status panel, level fades and pass logic. This is the core the map editor will exercise, so it also says which parts read the level file's data (`T7MP`, `.sflevel`) and which read copied-in memory.

## Design sections this implements

DESIGN §2.3. Sources: `docs/04-t7mp-level-format.md`, `docs/07-secret-passages.md`, `docs/09-object-system.md`, `docs/12-display.md` "Scroll", `docs/14-sflevel-format.md`.

## Files

- `docs/architecture/level.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Everything under `level/` this chapter owns: `mod.rs`, `load.rs`, `spawn.rs`, `scroll.rs`, `camera.rs`, `collision.rs`, `objects.rs`, `death.rs`, `exit.rs`, `tally.rs`, `panel.rs`, `pass.rs`, `fade.rs` (not `beam.rs`, `timeline.rs`, `frame.rs`, `clock.rs`, `bobs.rs`).
- Collision: game logic over a cell flag scheme; which tables are copied in (`TILE_HEIGHTS`).
- The object slot record (`docs/09`) is kept as the original's bytes; say what that costs a native rewrite.
- What the map editor's `.sflevel` path changes and what it leaves to the disks.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Mostly `superfrog-only`; a tile-map platformer core might carry its approach. Justify.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/level.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
