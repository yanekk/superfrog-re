+++
id = "display-hardware"
title = "The display hardware"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 95, game = 0, data = 0, shell = 5 }
reuse = "small-changes"
owns = [
  "toolkit/crates/amiga-chips/",
]
+++

# The display hardware

## 1. What it is

An engine built this way never draws a frame from "what is on screen". It keeps the memory the
display reads (copper lists, bitplanes, sprite data) as bytes at the original's addresses and plays
the display chips over that memory: the copper list run cycle by cycle on the bus model, each line's
bitplanes fetched from wherever the copper pointed them, the result composed as Denise would show
it. The picture is then a function of the state alone, and anything blits or CPU writes left in
memory shows as it did on the Amiga.

`toolkit/crates/amiga-chips/` holds the pieces that name no game:

- `src/copper.rs`: `run_bus` runs a list for one frame on the bus model: even DMA cycles, two per
  instruction word, never cycle `$E0`, losing the cycles the bitplane fetch takes (`Fetch::takes`).
  WAIT and SKIP wake as vAmiga's comparator does (`comparator`, `find_match`, `horizontal_match`). It returns every register `Write` with its beam
  position and optionally every bus cycle it took, for CPU and blitter models that need the free
  cycles. `run`, `run_timed` and `run_beam` are its simpler forms. `Fetch` is the fetch set-up
  (BPLCON0, DIWSTRT/STOP, DDFSTRT/STOP).
- `src/blit.rs`: the blitter in area mode. `Blit::run` covers ascending and descending, both shifts,
  first and last word masks and every minterm (`logic`), over any memory through `BlitMem`.
  `Blit::cycles` is a rough duration. Its timing is in `amiga-clock`
  ([cpu-time-and-beam](cpu-time-and-beam.md)).
- `src/picture.rs`: `Picture` is what Denise produces for one field: an index per lores pixel inside
  the window DIWSTRT and DIWSTOP open (`left`, `top`, `width`, `height` in the hardware's own units),
  the 32 colour registers per line as the copper leaves them at the window's left edge, and the
  colour writes the copper makes inside a line (`Split`, from lores pixel `2h`). `to_rgb12` and
  `Picture::rgb12` give 12-bit colours with extra-half-brite. The oracle reports its picture in the
  same terms, so the two compare pixel for pixel.
- `src/display.rs`: `draw` draws a lores frame from chip memory and the copper's writes (`Planes`,
  `Colours`, `window`, the constants `PIPELINE` 17 and `HIRES_PIPELINE`). It refuses what it does not
  draw: hires, HAM and dual playfield panic.
- `is_paula` in `src/lib.rs` says which custom registers are Paula's.

## 2. Kind mix

`amiga` 95, `shell` 5, an estimate from reading the crate. All of it copies what OCS chips do with
memory and registers; the shell part is `Picture::rewindow`, a presentation helper.

## 3. What the hardware does, for whoever completes this crate

Facts of OCS that a renderer must follow, whether or not they are in this crate yet:

- Hardware sprites: control words fetched on line 25, data between VSTART and VSTOP, the next control
  at VSTOP; a control write disarms from its pixel; shown at `HSTART + 1`; none on a line without a
  bitplane fetch; attached pairs make 16-colour sprites; priority against the playfield by BPLCON2's
  PF2P. DMACON's sprite bit can turn sprite DMA off for a screen.
- Hires fetch: fetch units of four cycles in plane order 4, 2, 3, 1, with `HIRES_PIPELINE`, and a
  lores pixel showing the first of its two hires pixels.
- A picture is the memory as the beam saw it, not as the frame ended. A write after the beam passed a
  line does not show on that line, so a renderer reading finished memory needs a log of the frame's
  writes.

Not modelled anywhere in this crate: sprite drawing, dual-playfield priority, HAM, interlace (a game
alternating two copper lists by field is ordinary lists to the copper; BPLCON0's LACE bit is not
modelled), sprite and playfield collision registers, the blitter's line and fill modes, ECS and AGA
registers, and the copper's danger bit. Each is an addition rather than a rewrite.

## 4. What depends on it

Every picture comparison in `amiga-conform` compares a `Picture`, and `amiga-app`'s `video.rs` shows
one. A game's renderer, its blits and its copper timing build on this crate. `dry-fit` shows the
whole crate used by a made-up game.

The picture comparison has two forms, both on 12-bit colour per pixel inside the display window:
exact, on memory transplanted from the original, and within tolerance on stepped runs
([verification](verification.md)). A differing display window is never tolerated.

## 5. What must be preserved

| Behaviour to keep | Check that proves it |
|---|---|
| Copper timing: two cycles a word on even cycles, never `$E0`, the vertical blank taking cycle 0, one even cycle in four lost to a fifth plane inside the fetch window, WAIT waking with the comparator two cycles ahead. | `moves_take_two_cycles_a_word_and_a_wait_holds_until_its_line`, `the_copper_takes_the_vblank_cycle_and_its_fetches_but_not_a_waits`, `a_fifth_plane_slows_the_copper_inside_the_fetch_window` (`toolkit/crates/amiga-chips/src/copper.rs`) |
| A WAIT reached past cycle `$E0` of its line never fires that frame. | `a_wait_reached_past_cycle_e0_of_its_line_waits_for_the_next_frame` (same file) |
| OCS vertical window, with VSTOP's bit 8 the inverse of bit 7; bitplane DMA only inside it. | `the_vertical_window_and_ocs_vstop_bit_8` (same file) |
| Colours written inside a line change from lores pixel `2h` on. | `a_split_recolours_its_line_from_its_pixel_on` (`toolkit/crates/amiga-chips/src/picture.rs`), `a_colour_write_changes_its_line_on_and_mid_line_is_a_split` (`toolkit/crates/amiga-chips/src/display.rs`) |
| Fetch: modulos by plane, BPLCON1's delay on the odd planes, a pointer rewritten mid-frame moving the fetch from that line on. | `the_modulo_moves_the_next_line_on`, `even_planes_take_bpl2mod`, `bplcon1_delays_the_odd_planes`, `a_pointer_rewritten_mid_frame_moves_the_fetch_from_that_line_on` (`toolkit/crates/amiga-chips/src/display.rs`) |
| Extra-half-brite: indices 32–63 show half the colour. | `extra_half_brite_halves_the_colour`, `six_planes_are_extra_half_brite` |
| The blitter: shifts, masks, minterms, ascending and descending. | `minterms`, `bltsize_decodes_rows_and_words` (`toolkit/crates/amiga-chips/src/blit.rs`) |

## 6. Native sketch (non-binding)

`Picture` with indices, a palette per line and splits is already the form the checks compare, so a
native renderer can emit it from different inputs. The smallest step keeps the display memory and
replaces the cycle-by-cycle copper run with a precomputed schedule of where each write lands. A
scene description (a palette per band of lines, a scroll, tile layers, sprite positions) is the
larger step, and needs a "what the beam had seen by this line" input or frames that raced the beam
fail the picture check.

## 7. Reuse

**`small-changes`**. The crate names no game. What another game may need is an addition: sprites,
hires, dual playfield, HAM, interlace, collisions, line and fill blits. The frame's length is a
value the game passes to the copper run.

## 8. Sources

`toolkit/crates/amiga-chips/`, `toolkit/crates/dry-fit/`, the Hardware Reference Manual.
