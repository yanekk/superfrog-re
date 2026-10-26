# T15 — disks-and-assets

**Phase:** 2 · **Depends on:** T01 · **Weight:** light

## Goal

Write the light chapter for `sf-disk` and `sf-assets`: identifying disks, carving `ATN!` chunks, the decompressor, decoding chunks into `GameData`, and the asset cache. Mostly modern code that reads Amiga formats; the chapter says which formats are Amiga-generic (ADF, hunk executables, planar graphics, OCS palette) and which are Team 17's (`ATN!`, `T7MP`).

## Design sections this implements

DESIGN §2.3 (light). Sources: `docs/01`–`docs/04`, `docs/10-asset-formats.md`, `docs/14-sflevel-format.md`.

## Files

- `docs/architecture/disks-and-assets.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Kind mix and reuse per crate, with the generic/Team 17/Superfrog split by module.
- Where these crates feed the Amiga-shaped parts: `ExeImage` hunks into `mem::Rom`, tables by offset.
- `ATN!` and `T7MP` reuse for other Team 17 games: flag, marked unchecked.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Two ratings in the text (one per crate); the front matter carries the combined one.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/disks-and-assets.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
