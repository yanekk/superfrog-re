# T41 — slot-screen

**Phase:** 2 · **Depends on:** T40 · **Weight:** medium

Added by T19 (owner, 2026-10-13). The slot machine's screen up to and through its attract loop,
and the drawing-on-the-beam model the rest of the slot needs. T19 had this green (`slot-attract`,
300 frames) before it was discarded; the numbers below are its measurements.

## Goal

`slot_machine` (h0+$49dc) from its sync point: loads, fade in, the lamps' attract pattern until
fire, frame-exact in variables, Paula writes and picture (beam races excused, below).

## Design (as T19 found it)

- **Front-end routines**: `crate::slot` with `Routine::Slot(slot::Routine)` and a bottom
  `Routine::SlotOver` (waits, changes nothing) in `front`; `Front::resume` maps a chain with
  h0+$478e to `[SlotOver, Machine(after its first raster wait)]` and transplants the sound driver
  with `files_for(world, module_addr)` (not the title's). `Scene::Slot { coins_in }`.
- **Loads, instant**: overlay as `end_seq_load`; FRUT→buffer 0 (h3+$10600), FRUC→buffer 1,
  FRUM→buffer 0+$f000; music faded and stopped; chip h3+$64c70..+$fa00 (the audio driver's view,
  `mem.b`) copied to h1+$20d00 (over `level_map`; keep a clone of the driver's `Mem` to restore);
  LFFX at the bank, `sfx_bank_init`, LFMU at the new `module_addr`; `cx.loaded(Reached(h0+$b7e2))`,
  `OverlayEnd`, `MusicStart` without moving the module (`keeping_module`).
- **Then**: hi-lo 1, score shown, `$50b6` count, `$661e` init, `$62f4` digits, pointer x,y = $20,$e8,
  six plane pointers into h2+$6708, `sprite::attach(h0+$1b10c)`, fade in (rec 0, list h2+$6688,
  target h0+$6912), raster, list h2+$65e4. Attract each frame: raster; reels set to 0,9,$18 and
  drawn; pattern row (h0+$62ec; `$ff80` ends a pattern: first h0+$3b2e8, from the 10th time
  h0+$3b1ae, reset at the 30th; a row lasts 3 frames, rows $34 apart) into the lamps; lamps drawn;
  fire or mouse ends it.
- **Drawing on the beam** (`slot::draw::Draw`, a routine of steps): the original draws after the
  raster wait at line 299 and on into the next frame. A step is a blit (memory at once), a CPU copy
  or CPU time; between steps inside the picture yield `Busy(line)` (one part per step, so the
  front end's per-part picture shows it from where the beam allowed); past line 313 yield
  `Busy(0)` with `cx.line = 313`, the cut step's remaining CPU carried after the interrupt, plus
  500 cycles (reels) or 700 (lamps). Moira cycles: `$645e` 13 674 (≈520 a lamp + 120 start),
  `$6558` 4 506, `$62f4` 47 502 (byte copies), `$6238` 1 650, `$6258` 1 234, `$6f3a` 25 962 for
  `       0PAGE` (the charset search is ~38 cycles an entry; space is entry 79), `rng_range` 414,
  `sfx_play` ~2 200 real (1 592 CPU). A blit: 2 cycles a word a channel. Under the six lowres
  planes a fetched word takes 1.4 cycles from the CPU, 1.9 from the blitter, 1.35 from byte copies.
- **The interrupt's length** (slot screen only, else the title's cases move): 17 lines, +1 when
  it polls the keyboard (INTENA $0008), +4/5 line per DMACON write of new notes, +2 per effect
  playing (add `AudioDriver::sfx_active`), + fade steps (280 cycles a colour, 200 more if moving).
- **Sprite DMA**: the blank list writes DMACON $0020 on line 107, so the pointer is invisible until
  a round's DMACON $8020; the renderer must follow DMACON's SPREN for this screen (opt-in,
  `SpriteDma`, CPU and copper writes in line order).
- **Beam races** (owner, 2026-10-13): a pixel differing only on lines a step of the engine's wrote
  within 4 lines of the beam passing them is excused; `FrameOut::beam_races`, the runner's
  `picture_except`, and `Outcome::raced` printed as `beam races excused at N frame(s)`. Document in
  `engine/conform/README.md`.
- **Jobs**: each drawing pushes a `Job` (writes its regions) with `inside` = every return address
  of its calls (lamps h0+$4d36, $4e82, $4ea0, $508c, $50ac, $50be, $54ec, $5518, $56bc, $56da,
  $5750, $5768, $5790, $57a4, $57f8, $5886, $5956, $597a, $5a30, $5ef2, $5f52, $6000 and the
  blitter waits $6464, $6518, $6530; reels $5c3c, $6556, $6568, $6578, $65b2, $660a, $6614; digits
  $4c9e, $564a, $5658, $57fc, $59f8, $5b48, $6062; print $5150, $5696); `$6238`/`$6258` likewise
  for `slot_lamps` (their 16 call sites).
- **Calibration aids** T19 used (scratch, uncommitted): a trace feature printing the engine's
  INTENA $10/$8010 lines per frame against the original's Paula `vpos`; this pinned every
  constant above to within 1–3 lines.

## Tests

- [ ] `slot-attract` (sync `slot = "slot-none"`, 300 frames) green, in the test block.
- [ ] sf-core unit tests: `interrupt_lines`, the race-row computation.

## Done when

- [ ] `slot-attract` green; existing `front-*` and `ending` cases still green.
