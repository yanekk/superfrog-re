# T10 — story-and-ending

**Phase:** 2 · **Depends on:** T01 · **Weight:** heavy

## Goal

Document the disk-3 intro and the end sequence. The intro is a separate program with its own memory map; the engine keeps the whole machine's RAM for it, runs its CPU and blitter on the bus cycle by cycle (`sim.rs`), and models a copper race (`cpu.rs`). It is the densest Amiga imitation in the project, and the chapter must say why that depth was needed and what a native rewrite of a cut-scene would have to reproduce.

## Design sections this implements

DESIGN §2.3. Sources: `docs/13-front-end.md` "The disk-3 intro", "The end sequence".

## Files

- `docs/architecture/story-and-ending.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Every file under `front/story/` and `front/ending.rs`.
- Why the bus-level simulation exists (which observable divergence forced it; check the commit history and `plans/rust-engine/FINDINGS.md` for T46, T52).
- The story's memory (`mem.rs`), ops (`ops.rs`), scenes, bobs, fades, sound, boot: kind mix per part.
- Must-preserve list: what the picture and sound checks compare during the intro and ending.
- How a native rewrite might express a cut-scene (a timeline of draws) while staying faithful; non-binding.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Likely `superfrog-only` for the content, with the bus simulator rated separately in the text as a reusable piece.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/story-and-ending.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
