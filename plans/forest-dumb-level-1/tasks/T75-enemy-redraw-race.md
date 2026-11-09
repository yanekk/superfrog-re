# T75 — enemy-redraw-race

**Phase:** 6 · **Depends on:** T74, T46; blocks T27, T48 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T75` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T46 with the owner's yes (2026-11-09). With creatures drawn the original's passes are longer, and the next pass can cut `main_loop`'s edge redraw before its scan reaches a creature cell. Measured on `collect-ledge` live: at irq_count 59 the interrupted PC is in `h0_3ac2` (a blitter wait inside the redraw), and the original starts W1-1:30,11's kind 5 a pass after the engine; it stays a pass behind until it leaves the view. Eight cases meet it and freeze `h0_681c` on both sides under this task: `camera-bottom-right`, `camera-right-left`, `character-edge-pit`, `collect-ledge`, `collect-return`, `contact-route`, `death-water`, `edge-latch-route` (first late creature at irq_count 93, 93, 354, 58, 58, 641, 571, 1250). T46's enemy cases stop before their first race (`enemy-route` 1250, `enemy-contact` 641, `enemy-hop` 93), and `fd-conform/src/bug.rs`'s known gaps (creature slots, `hits_taken`, `clxdat_copy`) name T75.

## Goal
Apply T74's mechanism to the creature starts: wherever the original's edge redraw is cut before its scan starts a creature, the engine starts it in the same pass, so the eight cases run with creatures live and T46's enemy cases run to their scripts' ends. Do not build a second mechanism (owner, 2026-11-09): T74 owns the choice; the preferred way is the runner handing the measured cut to the engine, in tests only.

## Design sections this implements
§2.1, §2.2, §4.1.

## Reuses
T74's mechanism, T46's `enemy_cases.rs` and the creature slots in `LAYOUT` (`creature_active+$N`, `creature_slots+$8N`, `+$8N+4`), T71's `interrupted_pc`.

## Files
`engine/conform/cases/*.toml` (the eight freezes removed; enemy cases lengthened), `engine/conform/scripts/enemy-*.txt`, `fd-conform/src/**` (the freeze entry and `bug.rs` gaps), `fd-conform/tests/enemy_cases.rs`, `fd-core/src/**` (fixes only), `fd-plan/src/confirm.rs`, docs/10, `smoke.toml`.

## Interface
T74's, extended to the creature start if it needs to be.

## Tests
- [ ] The eight cases pass with no freeze and their creature slots compared.
- [ ] `enemy-route` and `enemy-contact` run their source scripts' full length (1 300 and 1 300 steps).
- [ ] A cut creature start is accepted only at a measured cut: a test that fails without the check.

## Done when
- [ ] `freeze.rs` and `FROZEN_IN_A_CASE` have no entry naming T75; no exclusion or known gap names T75.
- [ ] docs/10 states, measured and dated, what the engine or the runner does about a creature start the next pass cuts.
