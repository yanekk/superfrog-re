# T190 — chain-world-change

**Split from:** T183 (its "chain-w1-4-to-w2-1" test and "after a world's last level, the world's end
picture, the next world's loading picture and its first level")

**Phase:** 2 · **Depends on:** T183; blocks T21 · **Weight:** medium

## Goal

`engine/conform/cases/chain-w1-4-to-w2-1.toml` (committed by T183, red) green, faithful and alone,
and in `engine/crates/sf-conform/tests/chain.rs`: W1-4's exit forced, the tally, `end_world_pic_w1`
(L1ET, 600 frames), `music_fade_wait`, `world_load_w2` with W2's effects and music loaded under the
end picture, W2's loading picture and the frog's walk in from the left, the progress map, W2-1.

The original's milestones (measured 2026-10-15, frames of play): tally 35, end picture 185, W2's
loading picture 805, map 1964, W2-1 2295.

## What T183 found and fixed but did not commit (redo these first)

All in T183's branch history only as notes; each was measured to move the case on:

1. `front/chain.rs` `WorldLoad` pc 1: after `world_sound`, when `END_PIC_SHOWN` is set, report
   `cx.loaded(LoadEnd::Reached(at(0x411c)))` (the FX/MU loads under the end picture; the original is
   then in h0+$43e0's wait called from `world_intro_show`).
2. `WorldLoad` pc 2: report the tile load as `LoadEnd::Reached(at(0x3df6))` (`progress_map_show`'s
   `overlays_end`, where the original waits in `world_intro_end`), not `Left(top_loop ret)`: under
   the trackloader the runner's `main_chain` (8 return addresses) never sees `top_loop`'s and the
   run never ends. The `ret` field of `WorldLoad` then goes.
3. `front/mod.rs` `Cx::loaded`: `self.line = l.max(1)` instead of `self.line.max(l)`: a load ends
   frames after it began, on the runner's line, not past where this frame had got to (W2's LP load
   began on line 299). `max(1)`: a `Poll` begun on line 0 counts as begun last frame and loops for
   ever in `run_main_split` (seen as a hang).
4. `sf-conform/src/lib.rs` load-line fit: compute `masked` inside `fits` from each try's own
   `out.jobs`, not only the first try's (a fade set-up begun only when the load ends earlier).
5. `layout.rs`: `CloudAnims` widened to h0+$12664..$12a14 (`Bytes(0x3b0)`: every world's cloud
   animations), and `CloudSprites` tagged `presentation` ("the loading picture's clouds' sprite
   records: where they are drawn"): the original's frame ends partway through h0+$12408, its
   clouds' positions a frame later than its walk state. Record this as a deviation.
6. `chain.rs` `walk_setup`: worlds 2 (h0+$1268c: exits at x $ffe0/$fff0 re-attached, walk
   h0+$11e96, anim $10d8e, tables $127d8/$1281c) and 4, 5 (h0+$12860: walk $11f44, anim $105b6,
   tables $1298c/$129d0) as world 1's; remove the `unported` for them.
7. `IntroEnd` pc 0, the wait after an instant load: `unstarted = WALK != at(0x7d4) && (display_timer
   as i16) < 1` (replaces T183's $11f44-only test): W2's walk in from the left sets no timer until
   h0+$1213e, and the walk enters that state a blank before it first sets it. With this the engine
   alone met every original milestone above to the frame (scratch run, not yet the case).

After those, checks 1–3 were green over all 3000 frames; check 4 was being re-run.

## Design sections this implements

DESIGN §2.1 ("loads"), §2.11.

## Files

- `engine/crates/sf-core/src/front/{chain.rs,mod.rs}`, `layout.rs`.
- `engine/crates/sf-conform/src/lib.rs`; `engine/conform/cases/chain-w1-4-to-w2-1.toml`;
  `engine/crates/sf-conform/tests/chain.rs`.

## Interface

No new public API: `front::chain`'s routines as T183 left them.

## Tests

- [ ] `chain-w1-4-to-w2-1` green faithful and alone; added to `tests/chain.rs`.
- [ ] `chain-start-w1-1`, `chain-w1-1-to-w1-2` still green; `tests/front.rs`, `tests/alone.rs`,
      `tests/frog_cases.rs` `tally_w1_1` still green.

## Done when

- [ ] All of the above green; no `unported` on the path.
