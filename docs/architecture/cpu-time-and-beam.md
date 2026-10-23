+++
id = "cpu-time-and-beam"
title = "CPU time and the beam"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 65, game = 0, data = 0, shell = 35 }
reuse = "big-changes"
owns = [
  "engine/crates/sf-core/src/level/beam.rs",
  "engine/crates/sf-core/src/level/timeline.rs",
  "engine/crates/sf-core/src/feed.rs",
]
+++

# CPU time and the beam

## 1. What it is

The original's picture and sound depend on *when* in a frame its CPU does its work, not only on
what the work computes. The vertical blank's interrupt runs from line 0, the main loop's 25 Hz pass
after it; in a level's first frame the pass draws bobs into the buffer being shown, a taken
pickup's tile is redrawn into both buffers while the beam may be below it, the sound driver's
DMACON off and on are lines apart, and in busy stretches the pass runs past the frame's end so the
next interrupt lands in the middle of it ([docs/08 "Lag"](../08-executable-map.md#lag-oracle)).
The engine computes each routine's result at once, so it has to model the time separately.

It does so in three parts, which are the files this chapter owns:

- **The level's beam clock and cost model**, `engine/crates/sf-core/src/level/beam.rs`. Ported
  routines charge 68000 cycles as they run (`LevelCtx::spend`, `LevelCtx::spend_n`, and
  `LevelCtx::blit_words` for a blit the CPU waits on, all in
  `engine/crates/sf-core/src/level/mod.rs`). `Cost` names 77 kinds of work, `Cost::cycles` gives
  each a fitted cycle count, and `COSTS`, `BLANK_COSTS` (a level's entry with the screen blank) and
  `RESTART_COSTS` (the restart after a death) are the three tables. `Clock` turns cycles into a
  beam position: 313 lines of 454 cycles, a fetching line (`$27`..`$12c`) giving the CPU 40 fewer
  (`FETCH_STOLEN_CYCLES`) and the blitter only the slots the five planes and the refresh leave
  (`Clock::blitter_slot`). `Clock::blit_cycles` runs a blit micro-instruction by micro-instruction
  as vAmiga's `SlowBlitter` does, giving each word's D write its position. The same file holds the
  late-pass machinery: `Cut` (where the original's frame ended mid-pass), `Pending` (the pass's
  start state, carried into the next step), `Frames` (the frames one step runs through) and
  `IRQ_PHASE` (a frame ending inside the interrupt handler, T203).
- **The timeline**, `engine/crates/sf-core/src/level/timeline.rs`. Work the original's main program
  does across several frames between levels (`level_init_wN` on a restart or a level's entry) is
  done at once, then `Timeline::of` turns its trace of charged cycles into the gaps between its Paula
  writes, and `Timeline::run` replays those gaps on the front end's line-counting beam, making
  each write on the line it falls, yielding at each frame's end. Built only by
  `init_on_beam_costs` in `engine/crates/sf-core/src/level/exit.rs` and run as
  `Routine::Timeline` in `engine/crates/sf-core/src/front/mod.rs`.
- **The pacing feed**, `engine/crates/sf-core/src/feed.rs`. What `sf-conform`'s engine run alone
  (rust-engine DESIGN §2.1 check 4) gets besides its start: one `FeedFrame` per frame with the
  input, the original's `Pacing` (`Unpaced`, `OnTime`, `PassDue`, `Late { … }`), whether the
  original's step ran two video frames, each load's length (`FeedLoad`: blanks, screen frames,
  end line) and RNG seeds read from the original's clock. `FeedFrame::play` is the one stepping
  function both `sf-conform` and the app (`--script --pacing FILE`) call, so the two cannot drift;
  `frame_hashes` hashes picture and state per frame for comparing them. Test tooling only:
  normal play never reads a feed (rust-engine DESIGN §2.5).

`Pacing` itself and the pass counter `PassClock` live in `engine/crates/sf-core/src/game.rs`
(chapter [game-flow](game-flow.md)); they are described here because they are this mechanism's
interface.

## 2. Kind mix

`amiga` 65, `shell` 35. `beam.rs` (714 lines) and `timeline.rs` (99) are imitation of the
Amiga's timing throughout: the cost table is fitted to the original's beam positions on the oracle,
not copied from it, so it is `amiga`, not `data`. `feed.rs` (449) is test tooling: a text format,
its parser, the run-alone stepping order and a frame hash. Judged by the line counts of the three
files, `feed.rs` counted as `shell` whole although what it carries is the original's schedule.

## 3. Where else the mechanism is used

**Charging cycles in a level.** `spend`/`spend_n` are called from
`engine/crates/sf-core/src/level/frame.rs` (21 calls: the vertical blank's parts in the
original's order, then the pass's), `engine/crates/sf-core/src/level/objects.rs` (18: slots,
hitboxes, collisions, the draw list, bobs), `engine/crates/sf-core/src/level/spawn.rs` (5),
`engine/crates/sf-core/src/level/scroll.rs`, `engine/crates/sf-core/src/level/load.rs`,
`engine/crates/sf-core/src/level/collision.rs`, `engine/crates/sf-core/src/level/pass.rs`,
`engine/crates/sf-core/src/level/panel.rs` and `engine/crates/sf-core/src/level/bobs.rs` (one to
three each), and `call` in `engine/crates/sf-core/src/level/mod.rs` charges every slot routine by
its address (`routine_cost`). `blit_words` is called from `objects.rs`, `scroll.rs`, `panel.rs`
and `pass.rs`. The sound driver charges after each Paula write (`LevelCtx::audio_spent`,
`LevelCtx::sfx`: `Cost::AudioStop`, `AudioMasked`, `AudioWrite`, `SfxRecord`, `SfxWrite`), and
`engine/crates/sf-core/src/frog/anim.rs` splits `frog_anim_tick` around its sprite wait. By
chapter: [level](level.md) carries most of the calls, then
[frog-and-items](frog-and-items.md), [enemies-and-bosses](enemies-and-bosses.md) (through
`routine_cost`: `HoverUpdate`, `WatcherUpdate`), [sound-driver](sound-driver.md) and
[display-hardware](display-hardware.md) (bob and blit set-up). Roughly, the vertical blank's
fixed costs are a few thousand cycles (`VblEntry` 2981, `Scroller` 3322), a pass's per-slot and
per-bob costs tens to hundreds each, a restart's set-up `InitStart` 15 620.

**Where a charge lands.** `LevelCtx::stamp` writes the clock into each new Paula write's
`vpos`/`hpos`, which `sf-paula` honours: each write lands at its beam position
(`engine/crates/sf-paula/src/lib.rs`, chapter [sound-chip](sound-chip.md)). The bus records
`Bus::at = Clock::nominal()` with every write to memory the display reads (`WriteLog` in
`engine/crates/sf-core/src/mem.rs`), and the renderer shows each byte as it was when the beam
fetched it ([display-hardware](display-hardware.md)). `FrameOut::beam_races` marks lines a draw
wrote within `RACE_LINES` of the beam (`engine/crates/sf-core/src/game.rs`).

**Anchoring in conformance.** In checks 1–3 the clock is not left to the cost model:
`Game::anchor_writes` sets it to where the original made each of the frame's Paula writes, and
`Game::anchor_blits` starts each blit where the original started one of its size
(`LevelCtx::scheduled`, `LevelCtx::scheduled_blit`, `LevelCtx::anchor_to`, which stretches the
display writes between two anchors to fit). Normal play and check 4 have the cost model alone.

**Other beam models.** The cost model is the level's. Three other parts of the engine keep their
own, coarser or finer:

- The front end's `Cx` (`engine/crates/sf-core/src/front/mod.rs`): a line counter moved by
  `Cx::busy`, `Cx::blit_wait` (1.23 CPU cycles a fetched slot), `Cx::masked_job` and `Cx::unpack`,
  with fitted constants (`VBL_END_LINE`, `FADE_*`, `STALL_VBL_LINE`). Used by the title, menu,
  attract, tally, slot machine and game-over screens. Chapters [front-end](front-end.md),
  [frame-interrupts-input](frame-interrupts-input.md).
- The slot machine's attract draw (`spend` in `engine/crates/sf-core/src/slot/draw.rs`, T93): CPU
  cycles from the frame's start, 1.4 cycles lost per fetched word. Chapter
  [slot-and-bonus](slot-and-bonus.md).
- **The story's bus simulator**, a second and finer instance of the same mechanism:
  `engine/crates/sf-core/src/front/story/sim.rs` runs the CPU and the blitter on the bus DMA cycle
  by DMA cycle in vAmiga's order (fixed DMA, then blitter, then CPU), with the CPU's instruction
  timings in `engine/crates/sf-core/src/front/story/cpu.rs`. It times the intro's draws exactly
  where the level's model fits costs. Chapter [story-and-ending](story-and-ending.md).

`CODE_TIMES` (`engine/crates/sf-core/src/game.rs`) is not part of this mechanism despite the name:
it is how many frames apart `Game::type_code` presses keys on the options screen.

**Fitting.** `Game::set_costs` and `Game::keep_trace` expose the model to
`engine/crates/sf-conform/examples/fit.rs`, which fits `Cost::cycles` to the original's write
positions and routine landmarks by least squares and prints the match arms to paste back.
`engine/crates/sf-conform/examples/restart_probe.rs` measured the restart's costs (T199, T208).

## 4. What depends on it

- **Picture.** Late redraws (a pickup's tile, the scroller's tiles, the panel) and bobs drawn into
  the shown buffer show above or below the beam as the original's did only because the display
  writes carry a beam position. Check 2 in every level case, and `late_redraw` in
  `engine/crates/sf-conform/src/lib.rs` for the render-only cases.
- **Sound.** Paula restarts a retriggered channel only if DMACON's off and on are far enough apart
  for the channel to go idle (T213). The engine's writes carry the clock's position into the mixer;
  made at one instant, the note is lost. T213 is ✅ in `plans/rust-engine/PROGRESS.md` on this
  branch (the task doc and the project's `CLAUDE.md` still call it open); its open remainder in
  `plans/rust-engine/FINDINGS.md` is the story's `sfx_play`, which makes all its writes at one
  instant (unmeasured).
- **Lag frames.** `Cut`, `Pending` and `Frames` let a conformance run end its frame in the middle
  of a pass where the original's did, so check 1 can compare state at the same pass ends
  (`PassClock`). Without them the engine would compare a pass the original had not finished.
- **Milestones.** In check 4 the engine is fed the original's `Pacing` through the feed; the
  frog's 50 Hz moves fall into the same 25 Hz passes as on the original, so scripts fitted to a
  lagging original play the same way. Unpaced, `complete-w1-1`'s script loses a life at frame 555
  that the original never loses
  (`the_w1_1_replay_loses_its_life_to_pacing_and_the_start_not_a_mechanic` in
  `engine/crates/sf-conform/tests/alone.rs`).
- **Between levels.** The restart's and each level entry's sound and fade writes are placed by the
  `Timeline` on the lines the original's are.

## 5. What a native rewrite must preserve

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| Cost model placing a level's Paula writes | With no anchors, each write within a line of the original's at the median, 70 % within 3 lines, 98 % within 10, over `items-run-w1-1` | `the_cost_model_alone_puts_paula_writes_on_the_originals_lines`, `engine/crates/sf-conform/tests/beam.rs` |
| `sfx_play`'s raster wait and DMA-on time (`SfxRecord`, `SfxWrite`) | Its wait ends three lines on and its off-to-on time is the original's within 150 colour clocks at the median, over `bonus-enter-wb-1` | `sfx_play_waits_three_lines_and_turns_the_dma_on_when_the_original_does`, `engine/crates/sf-conform/tests/beam.rs` |
| Paula writes spread over the frame (all paths) | Each channel's output equals the original's within the owner's tolerance (`channels::OWNER`) over the whole game | `the_whole_games_channels_sound_as_the_original_does` (ignored, ~7 min), `engine/crates/sf-conform/tests/channels.rs`; `sf-conform channels <case>` |
| Late pass (`Cut`, `Pending`, `Frames`) | The engine ends its frame at the original's write, step and point, runs the rest of the pass after the next interrupt, and state matches pass by pass | `items_pad_w1_1`, `items_run_w1_1`, `complete_w4_3` (DMACON/INTREQ on lag frames pinned) in `engine/crates/sf-conform/tests/frog_cases.rs`; `PassClock`'s `the_pass_clock_counts_passes_ended` in `engine/crates/sf-core/src/game.rs` |
| Late frames counted | The original's late frames are counted per case (320 in `frog-run-w1-1`, from 155; none on the title) | `the_originals_late_frames_are_counted`, `engine/crates/sf-conform/tests/frog_cases.rs` |
| Interrupt held behind the handler (`IRQ_PHASE`, T203) and the copper interrupt's tail past a frame's end | A blank held while the handler runs is taken as it returns; a tail outside a pass stays in its frame | `items_pad_w1_1`, `coper_tail_outside_a_pass_kept`, `engine/crates/sf-conform/tests/frog_cases.rs` |
| An unreachable cut | A `Late` cut the engine cannot reach ends the frame as normal play does, never panics | `a_late_cut_the_engine_cannot_reach_ends_the_frame_as_normal_play_does`, `engine/crates/sf-conform/tests/alone.rs` |
| Pacing fed to a run alone | Milestones equal the original's with only inputs, pacing, load lengths and seeds | check 4 on every stepping case (`sf-conform all`); `complete_w1_2_plays_alone`, `engine/crates/sf-conform/tests/alone.rs` |
| The feed shared by `sf-conform` and the app | The app paced from the feed is frame for frame the run alone (picture and state hashes) | `the_feed_replays_the_run_alone_headless`, `the_app_paced_through_the_whole_game_is_the_run_alone` (ignored), `engine/crates/sf-e2e/tests/whole_game.rs`; the format's unit tests in `engine/crates/sf-core/src/feed.rs` |
| Restart and entry timeline (`BLANK_COSTS`, `RESTART_COSTS`, `Timeline`) | The restart's fade, set-up and panel slide on the original's frames, with its sound and picture (T199 measured each set-up write within 3 lines of the original's with `restart_probe`; no test asserts the lines) | `frog_pit_w1_1`, `engine/crates/sf-conform/tests/frog_cases.rs` (frames 277–332, two restarts); a level's entry after a load in the `code-*` and `chain-*` cases |
| `Clock` line and blitter model | A fetching line gives the CPU less; a blit takes longer under the display | `the_cpu_gets_less_of_a_fetching_line`, `a_blit_takes_longer_under_the_display` in `engine/crates/sf-core/src/level/beam.rs` |
| Display writes at their beam position | A redraw or a bob in the shown buffer appears above or below the beam as on the original | check 2 in the level cases above. No check isolates it: the conformance runs anchor the clock at the original's writes and blits, so a drift in the cost model between anchors shows only as tolerated frames. Unmeasured how many frames depend on it; to find out, run `sf-conform all` with `Game::set_costs` zeroed |

## 6. Native sketch (non-binding)

Designing the rewrite is a later plan's job; this is a shape it could start from, not a decision.

**What could be simplified natively while keeping those checks green:**

- The 77 `Cost` kinds could become fewer, coarser costs per routine group, as long as
  `the_cost_model_alone_puts_paula_writes_on_the_originals_lines` keeps its 70 %/98 % bands and the
  `sfx_play` timing stays. Whether coarser costs hold the bands is unknown: `fit.rs` could be
  rerun with merged kinds to find out.
- `Clock::blit_cycles`'s micro-instruction run could be replaced by `slots_per_word` timing if no
  check needs each word's D write position. Unknown; `anchor_to`'s "a blit's writes are placed
  by its own cycles" is what uses it.
- The three beam models outside the level (front end `Cx`, slot draw, story simulator) could share
  one clock type; each is fitted separately today, so a shared one would be refitted per screen.
- `Pending` keeps a whole `State` clone per late pass to run it again from its start. A rewrite
  could keep only what a pass reads, at the cost of proving the pass deterministic from it.
- The feed is shell already and could stay as it is.

**What could not be dropped:**

- A beam position on every Paula write: without it notes are lost (T213).
- A beam position on every display write that can race the beam: without it check 2 fails on
  late redraws and first-frame bobs, as far as the render cases show (`late_redraw`).
- Ending a frame mid-pass where the original did, in conformance: checks 1 and 4 depend on it.
  Normal play never lags (owner, 2026-10-15) and needs none of it.

A native shape: the game logic computes a frame's results with no clock, and emits a list of
timed events (Paula writes, display writes) whose times come from a per-routine cost table; a
separate conformance adapter feeds `Late` cuts and anchors. That keeps the timing out of the
game rules, which today call `spend` inline.

## 7. Reuse

**`big-changes`**, 1262 lines (`python tools/architecture.py totals`). The cycle-charging approach
carries to any Amiga game whose picture or sound depends on where in the frame its CPU works;
the costs are per routine and refitted against the oracle for each game.

- `Clock` (the line model, `blitter_slot`, `blit_cycles`, `slots_per_word`) carries with small
  changes: the fetch window (`FETCH_LINES`, `FETCH_FROM`, `FETCH_TO`) and the planes stolen
  depend on the game's display set-up.
- `Cost`, `COSTS`, `BLANK_COSTS`, `RESTART_COSTS` and `routine_cost` are Superfrog's; another
  game writes its own and fits them with a tool like `fit.rs`.
- `Cut`, `Pending`, `Frames` are shaped by `level_frame_loop`'s 13 pass calls; the idea (end the
  frame where the original's did, run the pass again) carries, the fields do not.
- `Timeline` carries almost as it is, given a trace and a beam.
- `feed.rs` carries with small changes: the fields `load` and `seeds` are Superfrog's needs; the
  stepping order and the hashes are generic.

## 8. Sources

- [docs/08 "Main loop and interrupt" and "Lag"](../08-executable-map.md#main-loop-and-interrupt)
- [plans/rust-engine/DESIGN.md](../../plans/rust-engine/DESIGN.md) §2.1 (the four checks, lag and
  anchors) and §2.5 (timing)
- `plans/rust-engine/tasks/T213-paula-write-timing.md`, and `plans/rust-engine/FINDINGS.md`
  (T213, T216, T217 rows)
- Code: `engine/crates/sf-core/src/level/beam.rs`, `engine/crates/sf-core/src/level/timeline.rs`,
  `engine/crates/sf-core/src/feed.rs`, `engine/crates/sf-core/src/level/mod.rs` (`LevelCtx`),
  `engine/crates/sf-core/src/level/frame.rs` (`main_program`), `engine/crates/sf-core/src/game.rs`
  (`Pacing`, `PassClock`, `FrameOut`), `engine/crates/sf-core/src/front/mod.rs` (`Cx`)
