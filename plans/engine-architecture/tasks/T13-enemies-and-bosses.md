# T13 — enemies-and-bosses

**Phase:** 2 · **Depends on:** T01 · **Weight:** heavy

## Goal

Document the enemies of all six worlds and the bosses: their shared machinery (`enemies/common.rs`, `mod.rs`) and the per-world types. The chapter groups enemies by behaviour pattern rather than listing ninety files one by one, and identifies the shared shape a native rewrite could give them.

## Design sections this implements

DESIGN §2.3. Sources: `docs/09-object-system.md` "Types per world".

## Files

- `docs/architecture/enemies-and-bosses.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- `enemies/common.rs`, `enemies/mod.rs`, each world's `mod.rs`, a sample of types per world, `bosses/`.
- The common pattern a type follows (dispatch from the object record, state in original bytes, animation from copied-in tables, cycle charging) and the exceptions.
- A table of world × type with one line each, grouped by pattern.
- Must-preserve: what conformance compares for enemies (state, picture) and so what a native enemy must reproduce.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Likely `superfrog-only`; the dispatch skeleton may carry. Justify.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/enemies-and-bosses.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
