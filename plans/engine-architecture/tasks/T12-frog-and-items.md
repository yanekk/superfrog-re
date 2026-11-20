# T12 — frog-and-items

**Phase:** 2 · **Depends on:** T01 · **Weight:** light

## Goal

Document the frog (physics, animation, state) and the items (pickups, power-ups, fixtures, the bubble): game logic carried in original variables and driven by copied-in animation tables.

## Design sections this implements

DESIGN §2.3. Sources: `docs/09-object-system.md` "The frog, the spud, bobs and sprites".

## Files

- `docs/architecture/frog-and-items.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Every file under `frog/` and `items/`.
- Physics: which constants and tables are copied in versus written in code; fixed-point arithmetic as the 68000 did it, and why it must stay bit-exact.
- Animation frame lists read from the executable: link `copied-in-data`.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Likely `superfrog-only`.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/frog-and-items.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
