# T00 — movement-map-spike

**Phase:** 0 · **Depends on:** — · **Weight:** heavy

## Goal

Find out, on this machine, whether a movement map measured on the engine can be built fast enough and
is right where today's distance field was wrong, before any planner is built on it. Throwaway code: a
single example file, deleted at the end. Its product is a written answer in FINDINGS.md and the
choices recorded in DESIGN §2.2 / §7 by this task's review.

## Design sections this implements

DESIGN §2.2 (the open choices), §5 (the measured speeds), PLAN Phase 0 gate.

## Files

- `engine/crates/sf-conform/examples/zz_map_spike.rs` (created, then deleted before the task ends)
- `plans/smart-planner/FINDINGS.md` (rows)
- `plans/smart-planner/spike/` (committed text results: per-level edge counts, timings, the stall-spot
  verdicts; small text only)

## Questions it must answer

1. Seeding: placing the frog by writing `FROG+f::X/Y` (and clearing `VX/VY`, setting `ON_GROUND`) into
   a level-start state, versus a breadth-first walk over cells actually reached from the start by real
   moves. For each, on W1-4: cells covered, edges whose replay from a real reached state lands
   elsewhere (false edges), build time. Recommend one.
2. Holding the world still: what happens to edges when enemies are frozen (engine-side equivalent of
   `Freeze::Enemies`) versus live, and whether moving platforms and lifts (markers 44/62) are objects
   frozen with enemies. Recommend how the map treats them.
3. Speed: build time for the full move set on W1-4 and W6-3 on 10 threads; profile one second of
   `step_unseen` (`cargo instruments` is absent; use `sample <pid>` or a timing split) and say where the
   time goes.
4. Correctness at the recorded stall spots: W1-4 coins the old field called near (rust-engine FINDINGS
   T149/T205 rows), W6-3 "coins above open air" (T207 row): does the map call them unreachable, or
   reachable by a path that replays on the engine?

## What each answer means

- Build ≤ ~5 min per level and stall spots judged right: proceed to T03 with the recommended seeding.
- Build too slow: report the profile; the owner decides between speeding up `step_unseen` (a new task
  touching sf-core, performance only), a coarser move set, or a lazily built map.
- Stall spots judged wrong: the move set is missing a family; report which; the owner decides before T03.

## Done when

- [ ] FINDINGS rows answer questions 1–4 with numbers, and `spike/` holds the raw counts.
- [ ] A recommended seeding and world-holding rule is written in the task's commit message for the
      reviewer to put into DESIGN §2.2 and §7.
- [ ] The spike example file is deleted; `git status` shows only plan-folder changes.
