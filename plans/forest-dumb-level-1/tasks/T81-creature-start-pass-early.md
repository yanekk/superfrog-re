# T81 — creature-start-pass-early

**Phase:** 6 · **Depends on:** T79; blocks T27, T48 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T81` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T79's review with the owner's yes (2026-11-10). T79 ran `mechanic-vanish` with creatures live, and the case listed no creature slot variable. With the fifteen compared, the engine and the original differ from pass 330: the engine starts one creature a pass before the original does. The character never meets it, so every other variable agreed and the case passed. The case now compares ten slot variables and excludes the five that differ, naming this task.

## Goal
Find why the engine starts that creature a pass early, fix it so `mechanic-vanish` compares all fifteen creature slot variables with no exclusion naming T81, and list every level case that runs creatures live without comparing their slots (owner, 2026-11-10: T81 checks whether any other run fails to compare enemy state with enemies live, and lists what it finds).

The fix is of T75's reviewed work, so its commit ends with the trailer `Escape: forest-dumb-level-1 T75` (owner, 2026-11-10).

## What is measured, and what is not
- Measured (T79 review, 2026-11-10, a probe run of `mechanic-vanish` with the slot variables added): before the pass the runner calls 330 (oracle frame 6632, the engine's `irq_count` 331) it hands the cut `EdgesCut { next: 1, defer_from: Some(59008) }`. In that step the engine's redraw starts a kind 0 creature at `$429,$482`; after the pass its slots are the hopper (`$430,$450`, second long `$3ff0f`) and the new creature (second long `$ff01`). The original at the same pass end still has slot 0 as the creature freed the pass before (`creature_slots+$0` `$3bb03a2`, `+$4` `$136`, `creature_active+$0` 0) and the hopper in slot 1 (`+$c` `$3ff0f`): it has not started the new one.
- Measured: over the case's 1 469 frames `creature_active+$0`, `creature_slots+$0` and `+$8` differ on 1 pass each, `creature_slots+$4` on 28 and `+$c` on 544; the other ten never. `death-water` and `mechanic-spring` agree on all fifteen.
- Computed, not measured: `defer_from` 59008 is `OBJECT_MAP` (`$cb38`) plus column 72, row 72; the creature's cell, from its place, is column 66, row 72, below it, so the engine does not defer it.
- Not measured: where the original's row scan stood when the pass came, and whether it had passed column 66; whether the pass landed in `h0_681c`'s slot search or `h0_6852` (FINDINGS 2026-11-09, T75 review, read-only); whether `edges_cut` reads the scan's place from the right register there. T75 recorded that every cut it measured came before its scan's creature cell, so the place inside a scan is unproven for creatures.

## Rules the owner set
- Any measurement this task adds on the original must be shown to observe only (owner, 2026-11-09): the original is identical every frame with and without it, as `edge_latch.rs`'s `sampling_for_the_edge_cut_leaves_the_original_unchanged_every_frame` holds for the edge cut.
- No existing task is edited (owner, 2026-11-10). T80's doc asks that `mechanic_cases.rs` assert no exclusion again: until both tasks are done that means no exclusion naming the finished task.

## Design sections this implements
§2.1, §2.2, §4.1.

## Reuses
T75's hand-over (`Edges::deferred`, `redraw::Deferred::Creature`, `Io::defers`, `fd_conform::edges_cut`), T74's line sampling (`Sampler`, `interrupted_pc`), `enemy_cases.rs` (`creature_vars`, `a_creature_start_waits_only_where_the_measured_cut_says`), `plans/forest-dumb-level-1/prototype/t79-route-repair/probe.rs` (`TRACE` prints the slots a pass).

## Files
`fd-conform/src/lib.rs` (`edges_cut`), `fd-core/src/level/mod.rs` and `fd-core/src/level/redraw.rs` (the deferred start only), `fd-conform/tests/enemy_cases.rs`, `fd-conform/tests/mechanic_cases.rs`, `engine/conform/cases/mechanic-vanish.toml`, any case the list below adds slot variables to, `engine/conform/README.md`, docs/10, `smoke.toml`.

## Interface
None new expected. If the runner must hand the engine more than `EdgesCut` holds, it is a field of `EdgesCut`, forwarded as `foreground_cut` is.

## Tests
- [ ] `mechanic-vanish` compares all fifteen creature slot variables with no exclusion naming T81 and passes; `mechanic_cases.rs` drops `T81_SLOTS` and `T81` from `EXCLUDED_FOR`.
- [ ] A test fails without the fix: `mechanic-vanish` at pass 330 on `creature_active+$0`, the engine `$ff`, the original 0.
- [ ] The enemy cases and the seven live cases that compare the slots still pass (`enemy_cases.rs`, `collect_cases.rs`, `camera_cases.rs`).
- [ ] Any new measurement leaves the original unchanged every frame, on every level case the runner takes it in.
- [ ] A test lists the level cases under `conform/cases/` whose run fills a creature slot on the original while their `vars` and `exclude` name no creature slot variable, and fails on one not on its stated list.

## Done when
- [ ] No case excludes anything naming T81; nothing names T81 as pending.
- [ ] docs/10, "A creature the engine starts a pass early", says what the cause was and how it is handled, dated.
- [ ] The report lists every level case running creatures live without comparing their slots (26 level cases list no slot variable on 2026-11-10), and for each whether it now compares them or what differs when it does; a difference found is a finding, and a task proposed to the owner, not an exclusion added silently.
- [ ] The fix commit carries `Escape: forest-dumb-level-1 T75`.
