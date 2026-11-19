# T56 — mount-routine-timing

**Split from:** T55 (owner, 2026-11-19); carries T55's "every level 1 case that passes translated alone passes with `--mount level-1`".

**Phase:** 6 · **Depends on:** T55; blocks T40, T42 · **Weight:** heavy

## Goal

Make level 1's frame mount (`fd-translated/src/seam/frame.rs`) spend each callee's time as the translated routine does on the same state, so the eleven cases in `mount_level_1.rs`'s `T56` list pass mounted where they pass translated alone. Owner's rules (2026-11-19): each timing rule is general, derived from the state the routine reads and measured once against the original, with no per-case or per-address tables beyond the routine itself; each is checked against the translated routine's counted cycles on the routine judge's states; no translated frame runs beside the hand engine. Keep the whole-frame mount; loosen no comparison.

What T55 left (2026-11-19): the mount makes every custom-chip access by the original's instruction at its place and ticks the rest of each callee as `COSTS` (a fixed cost, plus one a blit), measured by `fd-conform/examples/game_frame_costs.rs`. That is exact for callees whose time does not vary and misses by up to 7 000 cycles on busy passes (camera-bottom-right pass 470: `camera_update` 3 284 against 1 240, `h0_2fae` 3 156 against 1 414, the creature cuts' contention about 3 000). JOY1DAT's read falls within 2 000 cycles of a frame's end on about 1 pass in 70, so a few hundred cycles moves it across.

The routines whose time varies (cycles from entry to return, contention taken out, over contact-route's 1 299 passes): `camera_update` `h0_4d7e` (548 to 2 994; its four tests and the step loops `h0+$51a0`..`$51f6` with `h0_51fc` and kin), `h0_2fae` (706 to 2 776), `h0_2dd6` (706 to 1 740), `h0_2c7e` and `h0_2d4c` (by blit, ±300, and where each blit's work falls against the blitter), `h0_62b6`, `char_sprite_copy` `h0_9b06` (four values), `h0_94ba` (the fade), `char_hit_check`'s contact paths, and `char_update` `h0_3e0c` (4 374 to 17 160, which sets the pass's end and so `main_loop`'s redraw cuts).

The eleven cases and their first difference mounted (T55, 2026-11-19):

| Case | First difference mounted | Translated alone |
|---|---|---|
| `camera-bottom-right`, `enemy-kind4`, `mechanic-spring` | pass 474 (frame 6776): `edge_latch_down` 0, original `$ffff` | passes |
| `contact-route`, `enemy-contact` | pass 116 (frame 6418): `char_x` 553, original 545 (JOY1DAT read 830 cycles early, before the frame's end, at pass 114) | passes |
| `death-spikes` | picture at pass 1343 (frame 7744): the four creature cuts start 1 000 to 1 600 cycles early; it passed with T36's instant mount | passes |
| `death-water` | picture at pass 1004 (frame 7306) | passes |
| `mechanic-vanish` | picture at pass 326 (frame 6628) | passes |
| `route-W1-1`, `route-both-W1-1` | picture at pass 945 / 943 | passes to pass 4160 (T54 from 4161) |
| `power-on-level-1` | frame 6301 | fails from 6301 too (T53) |

## Design sections this implements

DESIGN §2.9 (the seam; the mount's time, T55's paragraph).

## Files

- `crates/fd-translated/src/seam/frame.rs` (the rules in place of `COSTS` where a routine's time varies).
- `crates/fd-core/src/level/mod.rs`: `Pass` may hand the mount the typed state at each call, which a rule from the state needs (the pass's state is stored to memory only at its end).
- `crates/fd-conform/examples/game_frame_costs.rs` (the measure), a routine-judge check per rule.
- `crates/fd-conform/tests/mount_level_1.rs`: cases move from `T56` to `PASS_MOUNTED`; `CLX_LINES_OFF` to 0.

## Tests

- [ ] Full: each timing rule equals the translated routine's counted cycles (contention out) on the routine judge's states for that routine.
- [ ] Full: every case in `T56` that passes translated alone passes mounted (`every_case_that_passes_mounted_still_does` with the lists moved).
- [ ] Full: `the_mounts_clxdat_read_lands_on_the_translated_programs_line` with `CLX_LINES_OFF` 0.
- [ ] Full: `fd-judge --seam 'h0+$1fec'` still passes 2 000 of 2 000 (`seam.rs`).

## Done when

- [ ] The tests hold; DESIGN §2.9 states the rules, measured, dated.
- [ ] FINDINGS has which cases remain, each with its reason.
