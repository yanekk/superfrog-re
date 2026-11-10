# T80 — creature-cut-behind-beam

**Phase:** 6 · **Depends on:** T79; blocks T27, T48 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T80` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T79 with the owner's yes (2026-11-10). T79 ran the last three frozen cases with creatures live. Their variables agree with the original on every pass, creature slots included, but the play area's picture does not: with several creatures drawn a pass runs long and `h0_5820` cuts a creature after the beam has passed its lines, so that frame of the original shows it restored and not yet cut, missing, where the engine, drawing a pass whole, shows it. Measured on `mechanic-spring` from pass 475 with five creatures in view: the play screen's bytes equal the engine's where each pass ends, the frame differs (445 pixels at frame 6779), three frames and more in a row, past the tolerance's two (docs/10, "A creature cut behind the beam"). Frames differing so: 41 of `mechanic-spring`'s 3 609, 59 of `mechanic-vanish`'s 1 469, 30 of `death-water`'s 2 364, at any height of the play area.

## Goal
Have the runner measure, on the original, which creatures a frame shows uncut, hand that to the engine for that frame's picture as T71 handed the status bar's cut and T74 the edge redraws', and remove the three `rows 0..194` exclusions naming T80, so the play area's picture is compared with creatures live (DESIGN §2.2: no exclusion remains by T27).

**Open question for the owner, which this task must answer in its report:** the playable game will always show the creature where the original blinks it out. Say whether the engine could reproduce the blink in play (what it would need to model, and what that costs), so the owner can decide whether it should.

## What is measured, and what is not
- Measured (T79): the difference is in the frame and not in the play screen's memory at the pass end (`mechanic-spring`, passes 473 and 475, five slots active); it is a whole creature missing from the original's frame; it comes in runs of three frames and more.
- Not measured: the beam line at each creature's cut (`h0_5820`) against the creature's lines, whether a cut can land mid-creature (half shown), whether the restore (`h0_5e44`) can be caught the same way (a creature shown twice, or its old place not yet restored), and whether animated cells or bobs drawn in the same long pass blink too. The 137 pixels at (112, 171) of `mechanic-vanish` frame 6675 and the 268 at (108, 58) of `mechanic-spring` frame 6824 were not looked at.

## Rules the owner set (2026-11-10)
- Any measurement this task adds on the original must be shown to observe only: the original's side is identical every frame with and without it (chip and slow RAM, picture, CPU registers), the condition T74's review set and `edge_latch.rs`'s `sampling_for_the_edge_cut_leaves_the_original_unchanged_every_frame` holds for the edge cut.
- The handed-over blink applies only on frames the measurement proves: a frame that differs with no measured cut behind the beam still fails the case. Nothing is listed per case.

## Design sections this implements
§2.1, §2.2, §4.1.

## Reuses
T71's `Engine::cut_redraw` and `redraw_cut` (a frame passing only as the engine draws the measured cut), T74's line sampling (`Sampler`, `interrupted_pc`, `edges_cut`), T52's `render::objects::Creatures` (`cut`, `restore`), `plans/forest-dumb-level-1/prototype/t79-route-repair/probe.rs` (`DIFF` writes a diff picture, `BOX` reads a box of the engine's picture a pass).

## Files
`fd-conform/src/lib.rs`, `fd-conform/tests/**`, `fd-core/src/render/objects.rs`, `fd-core/src/game.rs` (the picture hand-over only), `engine/conform/cases/death-water.toml`, `mechanic-spring.toml`, `mechanic-vanish.toml`, `engine/conform/README.md`, `fd-conform/README.md`, docs/06, docs/10, `smoke.toml`.

## Interface
One more optional `Engine` method beside `cut_redraw`, forwarded by `Game`; a wrapper that does not forward it fails a case that meets such a frame.

## Tests
- [ ] The measurement leaves the original unchanged every frame, on every level case the runner takes it in.
- [ ] `mechanic-spring`, `mechanic-vanish` and `death-water` compare the play area's picture with no exclusion and pass; `mechanic_cases.rs` asserts no exclusion again.
- [ ] A frame differing where no cut behind the beam was measured fails (a mutant: the engine leaving a creature out on a frame the measurement does not name).
- [ ] An engine that does not take the hand-over fails `mechanic-spring` at frame 6779.

## Done when
- [ ] No case excludes `rows 0..194`; nothing names T80.
- [ ] docs/10, "A creature cut behind the beam", says what was measured and how it is handed over, dated.
- [ ] The report answers the owner's open question above.
