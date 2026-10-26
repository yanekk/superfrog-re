# T03 — copied-in-data

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Catalogue every table, memory image and blob the engine reads from the original's executable or disks and uses as it is, rather than reimplementing: hunk images, the tables the boot builds, jump tables, animation frame lists, tile height tables, the front end's packed pictures and overlay programs, the sound bank. For each, who reads it and whether a native rewrite would keep reading it, convert it at import, or replace it with code. This answers the owner's "static memory pushed by the system".

## Design sections this implements

DESIGN §2.1 (`data`), §2.3. This chapter owns no files (DESIGN §2.4); it is a view across them.

## Files

- `docs/architecture/copied-in-data.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Search by behaviour: `Rom`, `regions`, `boot_tables`, `TILE_PTRS`, `TILE_HEIGHTS`, `h0+$`, `ExeImage`, `GameData`, `front/data.rs`, `front/story/data.rs`, `sf-assets/src/tables.rs`, `exe.rs`.
- One table: item, where it comes from (executable hunk and offset, or disk file), what it is, which chapters read it, and the native option (keep reading at import, decode to a typed structure, or regenerate in code).
- The copyright boundary (rust-engine DESIGN §2.3): data is read from the player's disks, never committed; say how that constrains a native rewrite (tables cannot be baked into source).
- Which copied-in data is addressed by original pointer values inside other copied-in data (relocated longs), because that is what ties it to the memory model.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Rate the approach (read the original's data from the player's disks at run time) for another game, and note what is Superfrog-specific (offsets, formats).
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/copied-in-data.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
