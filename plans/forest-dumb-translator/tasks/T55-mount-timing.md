# T55 — mount-timing

**Split from:** T36 (owner, 2026-11-19); carries T36's "every level 1 case passes with `--engine translated --mount level-1`" and "`power-on-level-1` passes with the mount".

**Phase:** 6 · **Depends on:** T36; blocks T40, T42 · **Weight:** heavy

## Goal

Make level 1's frame mount (`game_frame`, `h0+$1fec`, `fd-translated/src/seam/frame.rs`) spend the original's time on the machine, so every level 1 case that passes on the translated program alone also passes with level 1 mounted. Keep the whole-frame mount; do not loosen any comparison (owner, 2026-11-19). Amend DESIGN §2.9's "a mounted routine counts no cycles" with the measured costs and the date.

What T36 measured (2026-11-19, `fd-conform run-all --engine translated --mount level-1`): 32 of 50 cases pass; the frame mount passes `fd-judge --seam 'h0+$1fec'` on 2 000 of 2 000 states. The mount ticks no cycles, so the whole pass happens at the instant `level3_handler` calls it. Eleven cases first differ at pass 2 (oracle frame 6304) on `clxdat_copy`, `$8060` against the original's `$8061`: CLXDAT gathers collisions line by line until read, and the original's `char_hit_check` reads it later in the frame than the mount's read at the pass's start. Those eleven: `camera-bottom-right`, `contact-route`, `death-water`, `enemy-contact`, `enemy-hop`, `enemy-kind4`, `enemy-route`, `mechanic-spring`, `mechanic-vanish`, `route-W1-1`, `route-both-W1-1` (both before T54's pass 4161, so the seam's; T54's difference from 4161 is not T55's). The other seven, not yet diagnosed (timing is the likely cause: `main_loop`'s foreground gets the pass's time, so its redraw cuts move, and the slide sound's four-line wait is not spent):

| Case | First difference mounted |
|---|---|
| `camera-right-left` | pass 360 (frame 6662): `char_x` 1054, original 1052 |
| `character-edge-pit` | pass 169 (frame 6571): `char_anim_ptr` `$c49420`, original `$c45320` |
| `character-run-slide-buttons` | pass 79 (frame 6481): `char_x` 499, original 496 |
| `collect-ledge`, `collect-return` | pass 72 (frame 6374): `joy_up` 1, original 0 |
| `edge-latch-route` | pass 124 (frame 6426): `char_x` 492, original 484 |
| `power-on-level-1` | Paula writes at pass 1045 (frame 6301). It fails translated alone from frame 6301 too (T53), so it passes here only once that is fixed or shown to be the same difference. |

Approach (T36's proposal, approved): the mount waits on the blitter where the original's `blitter_wait` (`h0_3ac2`) waits, reads CLXDAT, JOY1DAT and VHPOSR at the cycle the original reads them, spends the slide and hurt sounds' line waits, and ticks each callee's processor cost, measured on the translated program (each routine's cycles from entry to return on the machine over the level 1 cases) and stated as a rule, not a per-address table. Ticking goes through `Regs::tick` and the bus's `sync`/`poll`, so interrupts above level 3 and the frame's end are taken inside the mount as they are inside the translated routine.

## Design sections this implements

DESIGN §2.7 (the frame bar), §2.9 (the seam; this task amends its no-cycles line).

## Files

- `crates/fd-translated/src/seam/frame.rs` (the time), `crates/fd-core/src/level/mod.rs` only if `Pass` needs a hook at a read the mount must time.
- `crates/fd-conform/tests/mount_level_1.rs`: cases move from `T55` to `PASS_MOUNTED` as they pass.
- `plans/forest-dumb-translator/DESIGN.md` §2.9.

## Tests

- [ ] Full: every case in `mount_level_1.rs`'s `T55` list that passes translated alone passes mounted (`every_case_that_passes_mounted_still_does` with the lists moved).
- [ ] Full: `fd-judge --seam 'h0+$1fec'` still passes 2 000 of 2 000 (`seam.rs`).
- [ ] A made-up check that the mount's CLXDAT read lands at the original's line on one pass where the two differ today (pass 2 of `contact-route`).

## Done when

- [ ] The tests hold; DESIGN §2.9 states the mount's time, measured, dated.
- [ ] FINDINGS has the costs measured and which cases remain, each with its reason.
