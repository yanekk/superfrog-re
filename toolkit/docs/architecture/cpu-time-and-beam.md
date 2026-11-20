+++
id = "cpu-time-and-beam"
title = "CPU time and the beam"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 100, game = 0, data = 0, shell = 0 }
reuse = "small-changes"
owns = [
  "toolkit/crates/amiga-clock/",
]
+++

# CPU time and the beam

## 1. What it is

An Amiga game's picture and sound depend on *when* in a frame its CPU does its work, not only on what
the work computes: a draw into the shown buffer lands above or below the beam, DMACON's off and on
are lines apart, a late pass has the next interrupt land in the middle of it. An engine that computes
each routine's result at once has to model the time separately. The method is cycle charging: ported
routines charge 68000 cycles as they run, and a clock turns the cycles into a beam position.

`toolkit/crates/amiga-clock/` holds the part that is the machine's:

- `src/lib.rs`: the lengths and one position type. A raster line is 227 DMA slots and 454 CPU cycles
  (`LINE_SLOTS`, `LINE_CYCLES`); a long PAL frame is 313 lines, a short one 312 (`FRAME_LINES`,
  `SHORT_FRAME_LINES`). `BeamPos` converts between line and cycle, DMA slots, a nominal count and
  Paula's `hpos`.
- `src/clock.rs`: `Clock<T: Timing>`, configured by a display's `Timing` (`DisplayTiming`). A fetching
  line gives the CPU fewer cycles, and the blitter only the slots the planes and the refresh leave
  (`Clock::blitter_slot`). `Clock::blit_cycles` runs a blit micro-instruction by micro-instruction as
  vAmiga's `SlowBlitter` does, giving each word's D write its position.
- `src/blit.rs`: the blitter's bus timing: `slots_per_word`, `rough_blit_cycles`, `is_refresh`,
  `word_times` (the beam position of each word of a blit with priority).
- `src/policy.rs`: named ways a display's code spends CPU time on the beam: `whole_lines`,
  `exact_on_line`, `wait_lines`, `spend_stolen`, `free_between`.
- `src/timeline.rs`: `Timeline` replays on a beam work done at once that the original's main program
  spread over frames: the gaps between its writes, each write made on the line it falls on, yielding
  at each frame's end. The beam is the caller's, through `TimelineBeam`.

The cost table is not here: costs are per routine and fitted against the oracle for each game by
least squares over the original's write positions. A game keeps its costs, and any finer simulator,
in its own crates.

## 2. Kind mix

`amiga` 100: all of it imitates the Amiga's timing.

## 3. What depends on it

- **Sound.** Paula restarts a retriggered channel only if DMACON's off and on are far enough apart for
  the channel to go idle, so every Paula write needs a position
  ([sound-chip](sound-chip.md)).
- **Picture.** A write that can race the beam needs a position, or late redraws show wrongly
  ([display-hardware](display-hardware.md)).
- **Lag.** A conformance run ends its frame mid-pass where the original's did, so state can be compared
  at the same pass ends. The fields of such a cut are shaped by each game's pass; the idea carries.

## 4. What must be preserved

| Behaviour to keep | Check that proves it |
|---|---|
| The lengths, and positions round-tripping through every representation | `the_lengths`, `nominal_round_trips_every_cycle_of_a_frame_and_past_it`, `dma_and_slot_round_trip_every_slot_of_a_frame`, `paula_hpos_is_the_slot_held_at_the_lines_last` (`toolkit/crates/amiga-clock/src/lib.rs`) |
| The order of positions equals the copper's | `ordering_equals_the_coppers_line_then_h_order`, `plus_slots_matches_the_coppers_beam_plus` (same file) |
| A display's timing decides what a line gives | `a_timing_decides_what_a_line_gives` (`toolkit/crates/amiga-clock/src/clock.rs`) |
| The spending policies | `whole_lines_is_cx_busys_count_with_no_cycle_known`, `exact_on_line_is_cx_busys_count_to_the_cycle`, `wait_lines_is_cx_blit_waits_loop`, `spend_stolen_is_the_slot_draws_spend`, `free_between_is_the_slot_draws_line_sum` (`toolkit/crates/amiga-clock/src/policy.rs`) |

## 5. Native sketch (non-binding)

Game logic computes a frame's results with no clock and emits a list of timed events (Paula writes,
display writes) whose times come from a per-routine cost table; a separate conformance adapter feeds
late cuts and anchors. That keeps the timing out of the game rules. What cannot be dropped: a beam
position on every Paula write and on every display write that can race the beam. Normal play need
not lag, so ending a frame mid-pass is needed only in conformance.

## 6. Reuse

**`small-changes`**. The approach carries to any Amiga game whose picture or sound depends on where in
the frame its CPU works. `Clock` needs the game's display set-up, given as its `Timing`; `Timeline`
needs a trace and a beam. The costs are written and refitted per game.

## 7. Sources

`toolkit/crates/amiga-clock/`.
