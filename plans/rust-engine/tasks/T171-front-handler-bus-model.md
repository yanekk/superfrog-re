# T171 — front-handler-bus-model

**Phase:** 2 · **Depends on:** T130, T129; blocks T21 · **Weight:** large

Added by T130 (owner, 2026-10-15). T130 found what decides `frog-pit-w1-1`'s frame 318, and it is the
bus: no fitted term reaches it. Frames 613 and 658 (the copper reading a colour between the fade's
`andi` and `add`) need the same model.

## Goal

Time the front end's vertical-blank handler on the level's copper list (`level_start`'s fade in,
`level_exit_fade`'s fade out) access by access on the full DMA bus, from the `h0+$12a14` wait to the
acknowledge, so `frog-pit-w1-1` compares the picture on frames 318, 613 and 658 with no exclusion.

**Build one shared bus-level model, not a third.** Start from T46's bus-cycle 68000 model
(`front::story::cpu`: `Cpu`, `Bus`, the `op` table) and the fixed-DMA bus T52 and T96 built for the
story intro (`front::story::sim::Fixed`: refresh, the copper's cycles from `render::copper::run_bus`,
the bitplanes, audio slots from `sf_paula::AudioDma`). T129 (slot lamps) is moving the slot
machine's lamp blits onto the same bus; reuse what it adds. Whatever the level handler needs and
they lack (sprite DMA slots; an `AudioDma` kept across every frame, levels included, in `Game`)
goes into that shared model.

## What T130 measured

- 318 and 648 make exactly the same 3 582 CPU bus accesses from line 17 to the acknowledge;
  318 has 108 audio DMA slots against 648's 67, and acknowledges 32 cycles later (58:36 against
  58:04). By colour 30, 318 runs 23 slots behind 648 (`examples/fade_times`: colour 30's green is
  cleared from a wait at 53:12 against 52:216).
- On 648 the copper reads colour 30's new green with no slots to spare (the normal wait at 52:224
  is the first position that sees it); on 318 it reads about 19 slots before the green is cleared.
  Accuracy to a couple of slots is the bar.
- What an audio slot costs depends on the CPU's bus phase. On the engine's own bus, the
  with-audio against no-audio delay swings 0–65 slots as the start moves ±24 slots, and it does
  not follow T100's per-frame errors (309, as much audio as 318, is right under T100's timing).
- T100's `fade_start` is a fitted point, not the dispatch: the line samples put the `bsr $15456`
  around 18:150 on 318 and 648, about 2.5 lines after it.
- `fade::level_bus_times` (T130: the same code the intro carries, `story::vbl::fade_calls`) on the
  original's own bus (`examples/fade_bus`) makes 3 495 accesses dispatch to acknowledge, but ends
  ~2% late from that dispatch: check its instruction strings against Moira first.
- The `h0+$12a14` wait ends at a line boundary, so modelling from there needs only what follows it:
  `music_dma`, `sfx_tick`, `progress_map_vbl`, `kbd_poll`, `display_timer` and the dispatch
  (`h0+$11cca`–`$11d0c`).

Probes (all `sf-conform` examples): `vbl_probe` (per-line bus owners and PCs of the handler),
`vbl_audio` (acknowledge and audio slots per frame), `fade_times` (when each colour is written, by
moving the copper's wait), `fade_bus` (the model on the original's bus).

## Interface

`fade::level_step_times` and `vbl::fade_start` may be replaced; say so in PROGRESS. The story
intro's timing must not move (its cases stay green).

## Files

- `engine/crates/sf-core/src/front/{mod.rs,vbl.rs,fade.rs}`, `front/story/{cpu.rs,sim.rs,vbl.rs}`
- `engine/crates/sf-core/src/game.rs` (the audio DMA across frames)
- `engine/conform/cases/frog-pit-w1-1.toml`

## Tests

- [ ] `frog-pit-w1-1` with no `picture` exclusion on 318, 613 or 658; every other frame still green.
- [ ] The other cases with a front-end fade on the level's list (`frog-*`, `game-over*`,
  `items-*`, the tally and slot cases) still green.
- [ ] Unit tests of the post-wait code's access strings against the oracle's counts.

## Done when

- [ ] The above, in the test block (`frog_cases::frog_pit_w1_1`).
