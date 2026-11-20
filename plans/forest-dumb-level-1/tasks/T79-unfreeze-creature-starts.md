# T79 — unfreeze-creature-starts

**Phase:** 6 · **Depends on:** T75; blocks T27, T48 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T79` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T75 with the owner's yes (2026-11-09). T75 made the engine start a creature after the pass that cut its redraw, at the measured cut, and ran seven of T46's eight frozen cases live. Its task doc assumed only those eight freeze `h0_681c`. More do, so the entry in `fd-conform/src/freeze.rs` and `fd_core::level::FROZEN_IN_A_CASE` stays, naming this task.

## Goal
Move every remaining user of the `h0_681c` freeze off it, then delete the entry and what only it needs, so no case or test freezes creature starts (DESIGN §2.2: no exclusion remains by T27).

## Every remaining user
- `conform/cases/death-water.toml` (`freeze = true`), T46's eighth case. Live, its inputs lose a life to a creature at pass 200 and meet a kind 5 walker on flat ground near char_x `$71b`, char_y `$448` (pass 473 of the frozen path, a kind 1 on the ledge above at y `$410`) before the pool at W1-1:148,66. `tests/death.rs` and `tests/contact.rs` hold it: two lives lost in the water, `goal_met` 7204. It needs inputs that reach the pool with creatures live.
- `conform/cases/mechanic-spring.toml`, `mechanic-vanish.toml` (`freeze = true`, their comments and `tests/mechanic_cases.rs:6` still say T24) and `tests/mechanic_cases.rs` (`assert!(case.freeze ...)`, the engine through `engine_for`). Not tried live.
- `fd-plan/src/confirm.rs`: its case text sets `freeze = true` and its goal comes from `engine_for`; `fd-plan/tests/route.rs` and the README's gotcha describe it frozen.
- `fd-conform/tests/edge_latch.rs`: `frozen_route()` runs `edge-latch-route` frozen for T73's and T74's row measurements (rows of 9 bobs cut, pass 1212, `ROW_BOBS_CUT`); the case itself is live. Re-measure live or drop what no longer holds, docs/09 with it.
- `fd-conform/tests/object_claims.rs` (`freeze::apply(FROZEN)`, T21's claims), `tests/runner.rs` (`FROZEN` patched, the empty list's error), `tests/mechanic_routines.rs` and `tests/creature_drawing_routines.rs` (`freeze_as_a_case`), `src/case.rs`'s `thaw` tests (`h0_681c`).
- The plumbing: `freeze.rs`, `Case::freeze` and `thaw`, `Engine::freeze` and `thaw`, `Game::freeze_as_a_case` and `thaw_in_a_case`, `redraw::Io::creature_starts_frozen`, `LevelCtx::frozen`; `conform/README.md`, `fd-conform/README.md`, docs/10 "Freezing objects".

## What T75's searches for live inputs tried
The tools are in `plans/forest-dumb-level-1/prototype/t79-route-repair/` (throwaway; copy into `fd-conform/tests/` to run; its README says how).
- `repair.rs`, on the engine alone: the frozen run's path (char_x, char_y, lives per step) is the reference; random moves near the first step the live run leaves it for good (up held or released for 1 to 24 steps, 2 to 260 idle or held steps inserted) are kept when the run follows the path further. It repaired `collect-return` (888 steps for 820, about 2 000 tries) and `camera-bottom-right` with lives left out of the path (2 165 for 1 880), both then agreeing on the original.
- On `death-water` it found two runs reaching the pool on the engine alone (seeds 23 and 27 with held steps). Neither reached it on the original: the original loses a life at pass 200 where the engine alone does not. The engine alone feeds pass `k` the script's step `k`, while on the original a pass reads the step before or after at about 10 passes in 1 000 (T67), and the engine alone uses `Cut::Rows` where the case hands measured cuts, so creature timing differs. The same happened to its `character-edge-pit` and `camera-right-left` results, whose cases instead keep their inputs with the goal both sides reach live.
- `repair_on_original.rs` plays each candidate on the original between search rounds, takes which step each pass read and every measured cut from that run, and searches on with them. Unfinished: with steps removed the path score can be gamed (the run shrinks), and a lag at an input change needs the change moved a step, which it only tries blindly.
- Not tried: `fd-plan level W1-1 --guide` toward the pool, which avoids creatures by search; a route found mid-script from a state taken on the original.

## Design sections this implements
§2.1, §2.2, §4.1.

## Reuses
T75's hand-over (`Edges::deferred`, `fd_conform::edges_cut`), T46's `enemy_cases.rs`, `fd-plan`'s explorer, the prototype tools above.

## Files
`engine/conform/cases/death-water.toml`, `mechanic-*.toml`, their scripts, `fd-conform/src/**`, `fd-conform/tests/**`, `fd-core/src/**` (the freeze plumbing only), `fd-plan/src/confirm.rs`, `fd-plan/tests/route.rs`, both READMEs, docs/09, docs/10, `smoke.toml`.

## Interface
None new. `freeze` and `thaw` leave the case format.

## Tests
- [ ] `death-water` live: the character reaches the pool and loses lives to the water alone from then on, creature slots compared, `death.rs` and `contact.rs` holding it.
- [ ] `mechanic-spring` and `mechanic-vanish` live, their goals met.
- [ ] The planner's confirmation runs creatures live and its route test agrees.
- [ ] The edge-row measurements hold on a live run, or are restated as measured.

## Done when
- [ ] No `freeze = true` in any case, no `FROZEN` entry and no `FROZEN_IN_A_CASE` entry; nothing names T79.
- [ ] docs/10 "Freezing objects" says the freeze is gone, dated.
