# T00 — movement-map-spike

**Phase:** 0 · **Depends on:** — · **Weight:** heavy

## Goal

Find out, on this machine, whether a movement map measured on the engine can be built fast enough and
is right where today's distance field was wrong, before any planner is built on it. Throwaway code: a
single example file, deleted at the end. Its product is a written answer in FINDINGS.md and the
choices recorded in DESIGN §2.2 / §7 by this task's review.

## Design sections this implements

DESIGN §2.2 (the open choices), §5 (the measured speeds), §7 D14–D17 (questions 5–7), PLAN Phase 0 gate.

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
5. Enemies with the frog parked (D14): on W1-4 and W6-3, with the frog held at its start for 2 000
   frames, how many of the level's enemy markers ever occupy a slot, against a run that walks the
   level. Confirms or refutes that enemies spawn only as their edge scrolls in (docs/09).
6. Speed bands (D15): build W1-4 and W6-3 twice, cells only and with (cell, speed band
   stopped/half/full, facing) nodes. For each: build time on 10 threads, node and edge counts, and the
   share of edges whose landing node changes with arrival speed. Recommend one by D15's ~5 min rule.
7. Live failure (D17): replay a 500-edge sample per level from states actually reached by real moves
   (not seeded), enemies live: the share that land elsewhere or lose a heart. Also count edges that
   touch a coin on the way.

## What each answer means

- Build ≤ ~5 min per level and stall spots judged right: proceed to T03 with the recommended seeding.
- Build too slow: report the profile; the owner decides between speeding up `step_unseen` (a new task
  touching sf-core, performance only), a coarser move set, or a lazily built map.
- Stall spots judged wrong: the move set is missing a family; report which; the owner decides before T03.
- Q5 finds enemies do exist with the frog parked: D14 still holds (phase and frog-reactive enemies),
  but the row says so and the owner is told before T07.
- Q7's live-failure share over 30 %: the map is a weak guide; report it, the owner decides before T03
  whether the 3-failure / ×4 rule of D17 stands.

## Done when

- [ ] FINDINGS rows answer questions 1–7 with numbers, and `spike/` holds the raw counts.
- [ ] A recommended seeding and world-holding rule, and the speed-band verdict (D15), are written in
      the task's commit message for the reviewer to put into DESIGN §2.2 and §7.
- [ ] The spike example file is deleted; `git status` shows only plan-folder changes.
