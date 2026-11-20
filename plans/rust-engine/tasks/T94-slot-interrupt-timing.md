# T94 — slot-interrupt-timing

**Split from:** T72

**Phase:** 2 · **Depends on:** T72, T91, T93; blocks T73 · **Weight:** medium

Carries T72's "The interrupt's length" and "Sprite DMA" items and its Tests and Done-when: the
300-frame `slot-attract` case green, in the test block.

## Goal

The slot screen's interrupt as long as the original's, so every drawing step lands on its line and
the Paula writes match; the pointer hidden by the slot copper list's DMACON $0020.

## What to build

- `slot::interrupt_lines` (slot screen only, else the title's cases move): 17 lines, +1 when it
  polls the keyboard (INTENA $0008), +4/5 a line per DMACON write of new notes, +2 per effect
  playing (`AudioDriver::sfx_active`), + fade steps (280 cycles a colour, 200 more if moving).
  Measured by T72: new-note frames write DMACON $1,$2,$4,$7 on lines 4–7 and $8007 on 17.
- `render`: `SpriteDma`, opt-in: follow DMACON's SPREN (CPU and copper writes in line order).
- `slot-attract` to 300 frames; `AudioDriver::sfx_active`.

## Files

`engine/crates/sf-core/src/{slot/mod.rs,audio/mod.rs,render/}`,
`engine/conform/cases/slot-attract.toml`, `engine/crates/sf-conform/tests/front.rs`.

## Interface

`slot::interrupt_lines`; `AudioDriver::sfx_active`; `render::SpriteDma` (opt-in).

## Tests

- [ ] `slot-attract` (sync `slot = "slot-none"`, 300 frames) green, in the test block.
- [ ] sf-core unit test: `interrupt_lines`.

## Done when

- [ ] `slot-attract` green; existing `front-*`, `ending` and `slot-load` cases still green.
