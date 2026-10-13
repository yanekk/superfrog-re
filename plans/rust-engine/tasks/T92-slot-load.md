# T92 — slot-load

**Split from:** T72

**Phase:** 2 · **Depends on:** T72 · **Weight:** medium

Carries T72's "Front-end routines", "Loads, instant" and the "Then" list up to the attract loop.
Read T72's doc first: its Design section is the spec. The rest of the slot screen is T93, T94.

## Goal

The engine resumes at the slot sync point and matches the original, frame for frame, through
the loads, the overlay's fade-out, the music start, the count, `slot_init`, `slot_digits` and the
fade-in's set-up, up to the attract loop's first `wait_frame_raster` (h0+$4d24).

## What T72 measured on the oracle (2026-10-13)

From the `slot-none` sync point, original frame by frame (`main_chain`, CPU custom writes):

- Frames 0–757: loading (`access_overlay_show` $b74c, then `load_file`). Frame 758: the load is
  over, chain `151a2<b7e2<4c5e<478e` (overlays_end's fade wait); engine frame 0 stands for it.
  INTENA $4000 at 194 / $c000 at 224 there: the overlay fade-out's set-up.
- 758–772: overlay fading out. 773: COP1LC write line 22, `music_start` INTENA $4000 at 23, DMACON
  $000f at 41, $c000 at 41 (p41_init: 18 lines); then `slot_count_win` ($50b6): `slot_lamps_off`
  and `slot_draw_lamps` (26 blits, lines 45–99), its `wait_frame_raster`; frame ends in
  `print_text` (chain `6fbe<5150<4c80`).
- 774: the count is over at once (win 0); `slot_init`, `slot_digits`, pointer, plane pointers,
  sprite attach, fade-in set-up INTENA $4000 at 182 / $c000 at 224 (h0+$153a4, rec 0, 32 colours),
  COP1LC = h2+$65e4 at line 299; frame ends in the loop's `wait_frame_raster` (`73d8<4d2a`).
- Note `slot_count_win` always waits one raster even with nothing to count (h0+$5102).

## Tooling

T72 used a scratch example `sf-conform/examples/slot_dbg.rs` (rebuild it; ~60 lines): reach the
case's sync point, step the oracle, print per frame the chain (h0 offsets), `irqs()`,
`cpu_writes()` (DMACON/INTENA/COP1LC with line.hpos), `blit_starts()`. A listing:
`venv/bin/python tools/re_listing.py d1_c03 --out /tmp/re` (`slot_machine:` label).

## Files

`engine/crates/sf-core/src/slot/mod.rs` (new), `front/mod.rs` (`Routine::Slot`, `SlotOver`,
`Front::resume`), `scene.rs` (`Scene::Slot { coins_in }`), `game.rs`;
`engine/conform/cases/slot-load.toml` (sync `slot = "slot-none"`, ~17 frames).

## Tests

- [ ] `slot-load` green, in sf-conform's tests (`tests/front.rs`).
- [ ] Existing `front-*` and `ending` cases still green.

## Done when

- [ ] `slot-load` green: vars, picture and Paula through the frame the attract loop starts.
