# T04 — cpu-time-and-beam

**Phase:** 2 · **Depends on:** T01 · **Weight:** heavy

## Goal

Document how the engine reproduces *when* things happen within a frame: ported routines charge 68000 cycles as they run, which places each Paula write and late redraw at the original's beam position; the CPU-time budget per frame and lag; the level timeline replayed on the front end's beam; the pacing feed that lets a run alone follow the original's schedule. This is the part of the imitation a native rewrite is most tempted to drop, so the chapter must say exactly what observable behaviour depends on it.

## Design sections this implements

DESIGN §2.3. Sources: `docs/08-executable-map.md` "Main loop and interrupt", "Lag"; `plans/rust-engine/DESIGN.md` §2.1, §2.5; T213 in `plans/rust-engine/tasks/`.

## Files

- `docs/architecture/cpu-time-and-beam.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Search by behaviour across all of `sf-core`: `spend`, `CODE_TIMES`, `CodeTimes`, `Cpu`, `Job`, `PassClock`, `Pacing`, `step_paced`, `HandlerEnd`, `LoadEnd`, cycle constants. List which chapters' code charges cycles and roughly how much of it.
- `level/beam.rs`, `level/timeline.rs`, `feed.rs` in full.
- What depends on it observably: picture (late redraws, first-frame bobs), sound (note timing; T213 open), lag frames, milestones. For each, the `sf-conform` check that would fail if timing were dropped.
- What could be simplified natively while keeping those checks green, and what could not. Present, do not choose.
- The story's bus simulator (`front/story/sim.rs`, `cpu.rs`) is a second, finer instance; name it and link `story-and-ending`.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Rate the mechanism for another game: the cycle-charging approach carries, the costs are per routine.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/cpu-time-and-beam.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
