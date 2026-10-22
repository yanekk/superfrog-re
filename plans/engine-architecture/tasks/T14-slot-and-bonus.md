# T14 — slot-and-bonus

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Document the slot machine (rounds, drawing, awards) and the bonus level (its enemies, shots, death, frame): two mini-games with their own loops, partly sharing the level machinery.

## Design sections this implements

DESIGN §2.3. Sources: `docs/13-front-end.md` "The slot machine".

## Files

- `docs/architecture/slot-and-bonus.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Every file under `slot/` and `bonus/`.
- What each reuses from `level` and what it has of its own; the RNG's role in the slot.
- Mechanisms used and must-preserve list.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Likely `superfrog-only`.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/slot-and-bonus.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
