# T02 — memory-model

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Document the mechanism the whole engine stands on: the game's variables kept as the original's bytes at the original's addresses (`State`), the layout table naming them (`layout::LAYOUT`, `VarId`), the loaded executable image (`mem::Rom`, `loaded_hunk0`, `HUNK_BASES`), and the transplant and probe that let `sf-conform` compare engine and original by address. This is the first decision a native rewrite meets, so the chapter must make the cost of each route plain.

## Design sections this implements

DESIGN §2.3, §2.5. Sources: `plans/rust-engine/DESIGN.md` §3.3, `docs/08-executable-map.md` "Memory map".

## Files

- `docs/architecture/memory-model.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Why byte-exact: the 68000 code overlays words and bytes, keeps pointers to its own tables in variables (the `state.rs` and `audio/mod.rs` headers say so); give concrete examples from the code.
- How ported routines read and write state (`State::byte`, `write_byte` and the named accessors), and how widely: a rough count of call sites per chapter area, found by search.
- Transplant (`Start::Transplant`, `AmigaMemory`) and probe (`Game::probe`) and what `sf-conform` does with them.
- The coupling (DESIGN §2.5): what the comparison needs, and the routes a native rewrite has (keep a mapping layer, a probe adapter that projects native state to addresses, change the checks), with the cost of each. Present, do not choose.
- Save states (`savestate.rs`) depend on the state layout; say how.
- The story's own memory (`front/story/mem.rs`) is a second instance of this mechanism; name it and link `story-and-ending`.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: `small-changes` or `big-changes` is the likely range for the mechanism (another game needs its own layout table); justify the rating you give.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/memory-model.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
