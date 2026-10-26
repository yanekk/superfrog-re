+++
id = "story-and-ending"
title = "The story and the ending"
tier = "subsystem"
depth = "deep"
kind = "amiga"
mix = { amiga = 53, game = 36, data = 11, shell = 0 }
reuse = "superfrog-only"
owns = [
  "engine/crates/sf-core/src/front/story/",
  "engine/crates/sf-core/src/front/ending.rs",
]
+++

# The story and the ending

## 1. What it is

Two cut-scenes bracket the game. **The story** is what booting disk 3 plays: disk 3's boot program
(`d3_c02`) shows the Team 17 logo while it loads the intro, and the intro (`d3_c04`, Eric
Schwartz's) plays a text page, fifteen bob scenes with their own music and effects, three more text
pages and the "insert disk 1" page, then hands over to the game as a disk-1 power-on. **The ending**
is what follows the final level: `END1` and `END2` from disk 3, each an IFF ANIM, played with fades
and holds, then back to the title. [docs/13 "The disk-3 intro"](../13-front-end.md#the-disk-3-intro-t32-code-oracle)
and [docs/13 "The end sequence"](../13-front-end.md#the-end-sequence-t35-code-oracle) have the
original's side, scene by scene and frame by frame. This chapter covers how the engine copies them.

The two are imitated very differently. The ending is part of the game's executable and runs on the
front end's machinery (`front::Cx`, `State`, the front's fade records and `front::anim`); the
engine does each decode at once and spends its 68000 cycles on the beam. The story is a separate
program with its own memory map, so the engine keeps the whole machine's RAM for it, runs its
vertical-blank handler and main program on their own clock, and times its draws by running the CPU
and the blitter on the bus DMA cycle by cycle. It is the densest Amiga imitation in the project.
The engine reaches the story only from `Start::Disk3` (`engine/crates/sf-core/src/game.rs`,
`Scene::Story`), which conformance and one `sf-e2e` test use. The app always starts as a disk-1
power-on (`engine/crates/sf-app/src/main.rs`), so a player never sees the story today. The ending is
played in the app after the final level.

Files owned:

- `engine/crates/sf-core/src/front/story/mod.rs`: `Story`, the stage machine (`Stage::Boot`,
  `BootDone`, `Intro`, `Over`) and `Story::step`, one frame. It reads the copper list at the frame's
  top, runs the boot program or the intro, then renders through `render::story_screen` with each
  copper word as the copper read it: the snapshot before the handler, the list after the handler
  from `COPPER_SEES_HANDLER` lines into it, and each fade colour word as the handler's last write
  before the copper's read left it (`vbl::colour_writes` on `HandlerBus`). It carries the display
  registers from frame to frame (`Fetch`), follows interlace (BPLCON0's LACE, the long and short
  field `lof`), collects the copper's Paula writes (those before the CPU takes the interrupt,
  `BEFORE_IRQ`, belong to the frame before) and follows COP1LC. `BOOT_LAYOUT` and `INTRO_LAYOUT`
  are the variables conformance compares while each program runs. `Cx` is what a boot routine sees:
  the memory, the beam's line, byte waits on VHPOSR (`Cx::wait_bytes`) and the job and load reports.
- `engine/crates/sf-core/src/front/story/mem.rs`: `StoryMem`, 512 KB of chip RAM at 0 and 512 KB of
  slow RAM at `$C00000`, read and written big-endian by 24-bit CPU address.
- `engine/crates/sf-core/src/front/story/data.rs`: `StoryData`, what the story reads from disk 3:
  the boot program relocated to `BOOT_BASE`, the intro's hunks relocated to `INTRO_H0`..`INTRO_H3`
  as `d3_c02`'s relocator places them, hunk 2 out of `d3_c03` (`data_hunk`, a `HUNK_DATA` and its
  `HUNK_RELOC32` groups), the four picture banks and the raw sector spans.
- `engine/crates/sf-core/src/front/story/boot.rs`: the logo program `d3boot_main` as a state machine
  (`Boot`). Two copper lists alternate by field (`LIST_A`, `LIST_B`, each chaining COP1LC to the
  other), a hires four-plane interlaced picture, one fade record stepped below the picture after
  `wait_raster`'s waits for line `$ff` and then `$120`, and `copy_palette` copying list A's colours
  to list B.
- `engine/crates/sf-core/src/front/story/ops.rs`: `Op`, one step of the intro's main program, each a
  library call of the original (`Unpack`, `Load`, `ClearBuffers`, `InitObjects`, `Draw`, `FadeIn`,
  `WaitVbl`, `Music`, `Sfx`, `WaitFire`, the scene specials `MoveLeftUntil`, `Swoop`, `Glide`).
- `engine/crates/sf-core/src/front/story/scenes.rs`: `intro_main` as data. `MAIN` calls each scene's
  four routines (unpack, set up and fade in, play, fade out) with a `SkipCheck` after each; `END` is
  the insert-disk-1 page and the hand-over. `UNPACK_FRAMES` and `UNPACK_WORK` are each unpack's
  measured length by its call's return address.
- `engine/crates/sf-core/src/front/story/intro.rs`: `Intro`, the scheduler that runs `MAIN` on a
  clock in raster lines (`Intro::t`). Its vertical-blank handler runs at the top of each frame and
  its main program runs until it waits past the frame's end (`Wait`: `Vbl`, `Count`, `Fade`,
  `Busy`, `Masked`, `Poll`). A draw (`Intro::draw`, `time_draw`) and a scene's set-up (`SimJob`,
  `run_job`, `retime`) are CPU programs run on `sim::Sim`. Handlers a blit holds off are taken at
  the first instruction end that sees the request (`sim::Sim::irq_seen`), and a blank held through
  is served by the next blank's handler (`merged`). A handler whose 8-line wait runs past the
  frame's end finishes in the next (`wait_lines`, `tail`). The handler's length is the fitted
  `handler_lines`; a fade's set-up with interrupts off is `fade_setup_lines`. Under conformance it
  takes fed values (`Pacing`, the handler's entry and end, merged blanks, what runs during a load);
  in play it uses its own.
- `engine/crates/sf-core/src/front/story/vbl.rs`: the intro's handler `intro_vbl` (h0+$b102):
  `vbl_flag` and `vbl_count`, `planes_show` writing the shown buffer's plane pointers into the
  copper list, the sample stream's tick, the fades' steps (`fades_step` for in, out and across),
  fire or the mouse button setting `intro_skip`, the music, its 8-line wait, the effects and the
  timeline (`to_wait`, `after_wait`). `colour_writes` and `fade_calls` replay the handler's code
  path as 68000 bus accesses (`ENTRY`, `TAIL`, the fade loops) to give the cycle of each write to a
  fade's colour words. The level handler's fade timing reuses `fade_calls`
  (`level_walks` and its callers in `engine/crates/sf-core/src/front/fade.rs`, T130).
- `engine/crates/sf-core/src/front/story/fade.rs`: the palette fades, one algorithm in two copies
  (the boot program's one record, the intro's four): in, out and across, on the colour words of a
  copper list, a component at a time. `Rec::step_walk` returns the path the original's code took
  (`Walk`, `Colour`), which `vbl::colour_writes` times.
- `engine/crates/sf-core/src/front/story/bobs.rs`: the intro's bobs. Objects (`LIST`, `$b0` bytes
  each), frame descriptors (`frame_prepare`, `anim_prepare`), the two display buffers and the clean
  background (`BUFFER_A`, `BUFFER_B`, `CLEAN`), the masks built from the bank (`init_masks`).
  `clear_buffers`, `init` and `draw` do the memory work of `buffers_clear`, `objects_init` and
  `bobs_draw` and, beside it, build the CPU program branch for branch as a `sim::Prog` of bus
  accesses, register writes and blitter waits.
- `engine/crates/sf-core/src/front/story/cpu.rs`: the 68000's bus accesses as letters (`p` program
  word, `r` read, `w` write, `n` idle cycle, `^` the access that polls the interrupt level), in the
  order the reference emulator's CPU core (Moira) makes them, `op`'s instruction table, `refresh`,
  and `Cpu`, a CPU alone on a bus that only refresh and the copper share.
- `engine/crates/sf-core/src/front/story/sim.rs`: the bus simulator. `Fixed` is the fixed DMA of
  each frame (refresh, the audio slots, the copper's cycles from `copper::run_bus`, the bitplanes'
  from `Fetch::takes`). `Sim` runs the CPU and the blitter on it a DMA cycle at a time, in vAmiga's
  order: fixed DMA, then the blitter, then the CPU. The blitter is vAmiga's `SlowBlitter` for copy
  blits (`program`, the `micro` flags, the start stages, D's locked first write, BLTPRI and the BLS
  yield). `Prog` and `Step` are a CPU program to run on it.
- `engine/crates/sf-core/src/front/story/sound.rs`: the intro's sound. The driver is the game's at
  other addresses, so `session` loads `AudioDriver` from the intro's memory (`AudioDriver::intro_from`,
  `VARS`) for each call and writes it back. Here is what differs: `music_start`, the sample stream
  (`stream_tick`, `stream_start`, the buffer at h3+$621b0), the timeline of sound events
  (`timeline_start` and the tail of `after_wait`), and `sound_data`, the replayer's tables taken
  from the intro's hunk 0 at `REPLAYER_SHIFT`.
- `engine/crates/sf-core/src/front/ending.rs`: `EndSeqLoad` (`end_seq_load` h0+$a65c) and
  `EndSequence` (`end_sequence` h0+$a6e4) as front-end routines, with `CpuWork`: CPU work done at
  once whose cycles are spent on the beam line by line, fewer on lines a lores fetch past the
  fourth plane steals from (`MAIN_FROM`, `STOLEN_TENTHS_PER_WORD`, `SHOW_CYCLES`). `END1_UNPACK_CYCLES`
  and `END2_UNPACK_CYCLES` are measured. `resume` restarts the ending from the original's call
  chain. The clears of hunks 1 and 3, `load_hero`, the RNG reseed and `password_buf` follow.

### Why the bus-level simulation exists

It was forced by observed divergences in the picture and in the order of register writes, one after
another, each unfixable at a coarser grain. The history is in `plans/rust-engine/FINDINGS.md` (T46,
T52–T56, T96, T97) and the commits named below.

1. **The fade's colour race (T46).** The intro's handler rewrites each fade colour twice (the
   component cleared by `andi`, then set by `add` or `or`) while the copper reads the list near line
   32. Whether the copper sees the old word, the cleared one or the new one decides the colour of a
   line: on frames 540–553, 1244–1256 and 1758–1772 the original showed `$332` where the engine
   showed `$333` (T46's task doc). Fitting a line was not enough, because the writes land within a
   cycle or two of the reads. Commit 132db9d added `cpu.rs` (the handler's path as Moira's bus
   accesses on refresh and copper cycles) and fed the original's interrupt entry to conformance.
2. **Draws that hold the CPU and run into the display (T52, T53).** The intro's bob draws are tens
   of blits each (40 in the first draw, T90). Under BLTPRI the blitter holds the CPU off the bus, and
   in scene 4 a draw overruns the frame's start. So where the draw's DMACON `$8400` and BLTSIZE writes
   land, which blank's handler is held, and which buffer the next frame shows, all depend on the
   blitter's cycles against the copper's and the bitplanes'. The earlier estimate (`BlitClock`'s
   `CPU_PER_BLIT`, `CPU_PER_OBJECT`) missed. Commit 6104a5d built `sim.rs` from vAmiga's
   `SlowBlitter`, and c032267 made `bobs::draw` emit its CPU program to run on it.
3. **The handler's interrupt poll and end (T54, T96).** A handler a blit holds off is taken only at
   an instruction end whose polling access asked for the bus after the request (frame 3025's draw),
   and Moira polls in a chosen access, not always the last (frame 3109's `move.l`). Commits 698c3c7
   and afe1e93 added `irq_seen` and the `^` letter.
4. **Audio DMA (T96).** From scene 1's first effect (frame 3034) Paula's slot `$0D` delays blits, so
   `Fixed` takes the channels' slots (`sf_paula::AudioDma`).
5. **Set-ups under the display (T50, T56).** `objects_init`'s mask blits and the masks' CPU clear
   run under the scene's five planes, and scene 4's second mask blit holds the CPU through a whole
   frame (two blanks, one handler). Commit 85e713f replaced the fitted constants with
   instruction-level programs re-timed after each frame's handler, which took the full-intro scan
   from 867 differing frames to 43.

The level never needed this. Its draws are timed by charged costs on a fitted beam
([cpu-time-and-beam](cpu-time-and-beam.md)), which the picture check's tolerance and the
`beam_races` excuse absorb. The story's races land on lines the copper reads, and in vertical-blank
order, so a few cycles change what the picture and the variables show.

## 2. Kind mix

`amiga` 53, `game` 36, `data` 11, `shell` 0. Judged by reading each file and splitting its lines
(tests included) between the parts below, then weighting by file size. It is an estimate, not a
count.

- **`amiga`.** All of `sim.rs`, `cpu.rs` and `mem.rs`. In `vbl.rs`, `colour_writes`, `fade_calls`,
  `ENTRY` and `TAIL` (about three quarters of the file). In `intro.rs`, the beam clock, `SimJob`,
  `time_draw`, `handler_to_draw`, `wait_lines`, merged blanks, the audio DMA and the conformance
  feeds that align them (about 60%). In `mod.rs`, the per-line copper view, interlace, `Fetch`
  carrying, copper Paula writes and COP1LC. In `bobs.rs`, the `Prog` building beside each routine's
  memory work. In `boot.rs`, `wait_raster` on VHPOSR and the field-alternating lists. In
  `ending.rs`, `CpuWork` and its cycle constants.
- **`game`.** `ops.rs`; the bulk of `scenes.rs` (what each scene does, in order); the `Op`
  interpreter in `intro.rs`; the fade algorithms in `fade.rs`; the object, animation and clipping
  logic of `bobs.rs`; the handler's own work in `vbl.rs` (counters, skip, stream, timeline order);
  the stream and timeline in `sound.rs`; the order of `EndSequence` and `EndSeqLoad` (files, holds,
  passes, fades, clears).
- **`data`.** `data.rs`; the measured tables in `scenes.rs` (`UNPACK_FRAMES`, `UNPACK_WORK`) and its
  addresses into the intro's hunks; the fitted constants in `intro.rs` (`HANDLER_BASE`,
  `HANDLER_MUSIC`, `HANDLER_PER_32_COLOURS`, `fade_setup_lines`), `KICKSTART_LOW` in `mod.rs`,
  `sound_data`'s replayer tables; `END1_UNPACK_CYCLES` and `END2_UNPACK_CYCLES` in `ending.rs`.

Addresses everywhere are original addresses in the intro's own memory map, and every `Op` names the
routine it stands for by address. That is the memory-model coupling, counted as `amiga` where it is
the point of the code and left uncounted where it is only a label.

## 3. Mechanisms it uses

- **The story's bus simulator** ([glossary](glossary.md)): documented here. `sim.rs` and `cpu.rs`
  time the draws, the set-ups and the fade writes. Outside the story, the slot machine's draws
  (`engine/crates/sf-core/src/slot/draw.rs`) and the level's fade writes
  (`engine/crates/sf-core/src/front/fade.rs`) use them too.
- **Byte-exact state at original addresses** ([memory-model](memory-model.md)), in its widest form:
  the story has no `State` at all, only `StoryMem`, the whole of RAM. `INTRO_LAYOUT` and
  `BOOT_LAYOUT` name the regions conformance compares; `Game::probe` and transplanting use them
  while the story runs.
- **The loaded hunk image** ([copied-in-data](copied-in-data.md)): `StoryData` relocates a second
  program (`d3_c02`, `d3_c04`, `d3_c03`) where its loaders put it, and the intro's frame tables,
  palettes, copper lists and animation lists are read out of that image by address. The ending
  reads `END1`, `END2` and the palettes at h0+$aa80 and h0+$aa40 of the game's own hunk 0.
- **The copper run, bitplane fetch and the blitter** ([display-hardware](display-hardware.md)).
  `render::story_screen` renders over `StoryMem`; `copper::run_bus` gives the copper's cycles to
  `Fixed` and to `HandlerBus`; `copper::run_timed` gives the next frame's early Paula writes; blits'
  memory effects use `front::blit::Blit`. Hires fetch and the two interlace lists are only used by
  the logo.
- **Cycle charging and the beam** ([cpu-time-and-beam](cpu-time-and-beam.md)). The intro's clock in
  lines is a beam model of its own, the fourth beside the level's `Clock`, the front end's `Cx` and
  the slot machine's (that chapter's list). `ending::CpuWork` is the front end's form of cycle
  charging. It is defined in `ending.rs` but also used by `engine/crates/sf-core/src/front/title.rs`,
  `engine/crates/sf-core/src/front/chain.rs` and `engine/crates/sf-core/src/slot/rounds.rs`.
- **The vertical-blank interrupt** ([frame-interrupts-input](frame-interrupts-input.md)). The intro
  has its own handler (`vbl.rs`), taken where the CPU polls for it. Fire and the mouse button are
  read from CIA-A PRA bits 6 and 7 (`to_wait`, `Op::WaitFire`). The ending runs under the front
  end's handler.
- **Paula writes** ([sound-chip](sound-chip.md), [sound-driver](sound-driver.md)). The intro runs the
  game's driver on its own variables (`AudioDriver::intro_from`, `intro_vars`). `sf_paula::AudioDma`
  gives the channels' DMA slots that `Fixed` takes from the CPU and the blitter.
- **Loads** ([game-flow](game-flow.md)). The intro's loads are instant (`LoadEnd`, `Cx::loaded`); what
  runs on during them (`RUNS_DURING_LOAD`) and the blanks the original's handler ran (`load_blank`,
  T214) are put in place at the load.

## 4. What depends on it

- **[game-flow](game-flow.md).** `Game` runs `Story::step` while `Scene::Story` is up and starts the
  game as a power-on when it is over. It routes conformance's story feeds (`set_handler_entry`,
  `set_handler_end`, `set_blank_merged`, the load state) and `Game::probe`'s layout to it. The
  front end's routine stack (`engine/crates/sf-core/src/front/mod.rs`, `Routine::EndSeqLoad`,
  `EndSequence`, `CpuWork`) runs the ending from `top_loop`.
- **[front-end](front-end.md), [slot-and-bonus](slot-and-bonus.md).** `CpuWork` (above) paces the
  title's, the chain's and the slot machine's CPU work.
- **[level](level.md).** The level handler's fade writes are timed with `vbl::fade_calls` and
  `cpu::Cpu` (`engine/crates/sf-core/src/front/fade.rs`).
- **[slot-and-bonus](slot-and-bonus.md), again.** The slot machine's draws run on `sim::Sim` and
  `sim::Prog` (`engine/crates/sf-core/src/slot/draw.rs`). So the bus simulator is not the story's
  alone, though this chapter owns it.
- **[display-hardware](display-hardware.md).** `render::story_screen`, `DisplayData::story` and
  `Mem`'s view of `StoryMem` exist for the story. `render/mod.rs` builds `StoryData`.
- **[verification](verification.md).** `sf-conform`'s sync point `power-on-disk3`, `sync.rs`'s story
  feeds (`handler_entry`, `handler_end`, `intro_resumes`, `step_inserting`) and the examples
  `unpack_probe`, `fade_bus` and `slot_lamps_sim` call into `front::story`. The tests
  `engine/crates/sf-conform/tests/story_blits.rs` and `engine/crates/sf-conform/tests/story_draws.rs`
  drive `sim::Sim` directly against the oracle's RAM and copper.
- **[app](app.md).** The ending is played in the app. The story is not reachable from it.

### What the checks compare during the story and the ending

- **The story cases** (`engine/conform/cases/story-*.toml`, sync point `power-on-disk3`) compare, on
  every frame, the variables of `Story::layout` (the boot's fade record and lists, or the intro's
  counters, objects, frame descriptors, fade records, copper list, text page, both display buffers,
  clean background, sound records and timeline), the picture under check 2's tolerance (rust-engine
  DESIGN §2.1), and the Paula writes without their beam position. Writes a job makes past a frame's
  end are compared once the original has left the job (`Job`). Some tests then assert beam lines:
  `the_intro_from_disk_3_matches_the_original` asserts each draw's and scene 1's mask blits' DMACON
  `$8400` line. `the_effects_keep_in_step_through_scenes_4_to_6` runs `--strict` on the Paula writes
  themselves. `channels.rs` compares each channel's output level.
- **The ending case** (`engine/conform/cases/ending.toml`) compares `LAYOUT`, the picture and the
  Paula writes as any front-end case does, and each decode's memory once the original is out of
  `anim_next`.

## 5. What a native rewrite must preserve

Faithful stays the bar. Each row is behaviour the original shows, and the check that holds the
engine to it today. Tests named without a file are in `engine/crates/sf-conform/tests/story.rs`.

| Behaviour to keep | Check that proves it |
|---|---|
| The logo: hires four-plane interlaced picture shown by two lists alternating by field, the first interlaced frame a short one; fade in and out stepped once a frame after line `$120`; the load between them; Kickstart's 320 × 200 window on the first frame. | Case `engine/conform/cases/story-boot.toml` (`story_boot` in `engine/crates/sf-conform/tests/cases.rs`), and frames 0–66 of `the_intro_from_disk_3_matches_the_original`. |
| The fades (in by fifteenths of the target into an accumulator, out to black the same way, across one step a frame), each record stepping every `speed` calls, done on the first step that changes nothing. | `a_fade_in_reaches_the_target_and_a_fade_out_black` (`engine/crates/sf-core/src/front/story/fade.rs`); the `fade_records` variable in every story case. |
| A fade colour as the copper read it: the handler's component writes (clear, then set) timed against the copper's read near line 32, so a line can show a half-written colour. | `the_intro_from_disk_3_matches_the_original` (frames 540–553, case `story-intro`); `the_intro_through_scene_1s_fade_out_matches_the_original` (1244–1256, slow). The fed interrupt entry is conformance's; normal play assumes line 0 cycle 10. |
| A fade's set-up runs with interrupts off and can cross a frame's end; a blank during it is taken when it ends. | `the_intro_through_scene_1s_fade_out_matches_the_original` (frame 1242, slow). FINDINGS T49 notes the job's `inside` ranges are not defended by a test. |
| The blitter's timing: start two cycles after BLTSIZE, a cycle a channel a word, D's first write locked, busy clearing at the last word, waiting for refresh, copper, bitplane and audio cycles, BLTPRI holding the CPU, yielding to a CPU that has waited two cycles without it. | `on_a_free_bus_a_word_takes_one_cycle_per_channel`, `every_program_has_the_hrm_word_length_and_bus_cycles`, `the_blitter_waits_for_the_fixed_dma_and_dma_on`, `the_copper_and_the_bitplanes_take_their_cycles`, `audio_dma_takes_its_slots_from_the_cpu_and_the_blitter`, `without_bltpri_the_blitter_yields_to_a_cpu_that_waited_two_cycles` (`engine/crates/sf-core/src/front/story/sim.rs`); `scene_1s_masks_and_the_first_draw_begin_free_and_end_on_the_originals_cycles` (`engine/crates/sf-conform/tests/story_blits.rs`). |
| A draw: restores from the clean background then cookie-cut blits, last object first, into the buffer not shown, which shows from the blank after the draw ends; every register write on the original's cycle, the draw starting from the handler's acknowledge. | `draws_write_their_blits_on_the_originals_cycles` (`engine/crates/sf-conform/tests/story_draws.rs`: every draw of scenes 1 and 4, scene 3's first 10); the DMACON `$8400` lines in `the_intro_from_disk_3_matches_the_original`; the `display_buffers` and `objects` variables in every story case. |
| A handler held off by a blit is taken at the first instruction end whose polling access asked for the bus after the request; a blank held through is served by the next handler. | `draws_write_their_blits_on_the_originals_cycles` (7 held handlers); `scene_4s_set_up_under_the_display_matches_the_original` (merged blanks, slow). |
| Audio DMA slots taken from the CPU and the blitter. | `audio_dma_takes_the_originals_slots_across_scene_1s_effect` (`engine/crates/sf-conform/tests/story_draws.rs`). Later frames' handler writes are not seen by a draw's audio DMA, and no check covers that (FINDINGS T96). |
| A scene's set-up (`buffers_clear`, `objects_init`, the masks' clear and blits) timed under the list the frame shows, slowed by the fetch. | Scene 1's mask DMACON line in `the_intro_from_disk_3_matches_the_original`; `scene_4s_set_up_under_the_display_matches_the_original` (slow). Later scenes' mask DMACON lines are unasserted (FINDINGS T56 review). |
| The handler's 8-line wait read on line 305 or later ends in the next frame, whose blank then has no handler. | `a_wait_read_on_line_305_or_later_runs_on_to_the_next_frame`, `a_handler_whose_wait_runs_past_the_frames_end_ends_in_the_next_which_has_none_of_its_own` (`engine/crates/sf-core/src/front/story/intro.rs`); `the_effects_keep_in_step_through_scenes_4_to_6` (slow). |
| The handler's length in play (12 lines, 17 with music, plus 34 per 32 colours a fade steps). | No check: conformance feeds the original's handler end, so the engine's own length goes untested (FINDINGS T54, T53 review). It ignores the effects' work, about 3 lines a frame (FINDINGS T48). |
| Each unpack's length, and the insert-disk-1 page's unpack timed as work the handlers push on. | `the_whole_intro_into_the_game_matches_the_original` (slow); the `story-skip-*` tests (slow), which unpack that page after a skip. |
| Skips: fire or the mouse button sets `intro_skip`; a scene runs to its end; a text page ends on fire only; the logo and the insert-disk page ignore the mouse. | `fire_on_a_text_page_skips_the_intro_as_the_original_does` and the other `story-skip-*` tests (slow except the first). |
| The music plays through the intro's loads; the copper's Paula writes before the interrupt belong to the frame before. | `the_storys_music_plays_on_through_its_loads_as_the_original_does` (`engine/crates/sf-conform/tests/channels.rs`, slow); the Paula compare of every story case. |
| A story effect retriggering a channel mid-sample: `sfx_play` makes all its writes at one instant. | No check; a known gap (FINDINGS T213 review sweep). |
| Disk 1's id read, the hand-over as a disk-1 power-on. | `the_whole_intro_into_the_game_matches_the_original` (slow, to the title). |
| The ending: each unpack and decode spent on the beam from line 22, slowed by the fifth plane, so every picture shows on the original's frame; holds, passes and fades; hunks 1 and 3 cleared; the frog's sprites reloaded. | `the_end_sequence_matches_the_original` (`engine/crates/sf-conform/tests/front.rs`, case `ending`); `the_hiscore_name_entry_after_the_end_sequence_matches_the_original` (same file). |
| The RNG reseed from CIA-B's time of day after the ending. | No check: the engine keeps no clock; conformance takes the original's state (`resync`). |
| Widescreen on the story shows the border at the sides. | `widescreen_on_the_title_story_intro_and_slot_machine_shows_the_border_at_the_sides` (`engine/crates/sf-e2e/tests/view.rs`). |

The ignored ("slow") tests are not run by `cargo test` by default; `sf-conform all` and
`cargo test -- --ignored` run them.

## 6. Native sketch (non-binding)

This sketch is non-binding: designing the rewrite is the later plan's job.

A cut-scene could be a timeline of draws: per frame, which picture to show, which palette (with
colour changes at given lines and pixels), which objects at which positions and frames, and which
sound events. `scenes.rs` is already most of this, as data. The fade algorithms, the animation
lists and the frame tables would become plain functions and decoded assets. The ending already
nearly has this shape: `front::anim` decodes, and `CpuWork` only decides on which frame a picture
shows.

Two things make it hard to stay faithful. First, where in the frame something lands decides what
is shown: the colour race, a draw that ends after the blank and shows a frame late, a set-up that
holds the CPU through a blank. A native timeline would need, per frame, the result of that timing
(on which frame each draw shows, which lines see which colour) as a recorded schedule. The bus
simulator could compute it once, offline, from the scripts, and the shipped program would replay
it. That keeps the simulator as a build tool rather than a runtime. Second, the checks compare the
intro's memory region by region (`INTRO_LAYOUT`): the display buffers, objects, descriptors and
fade records. A rewrite without that memory needs the probe adapter or the check change
[memory-model](memory-model.md) and [verification](verification.md) set out, or a picture-and-sound
only comparison for the story. Which of these to take is the rewrite plan's decision.

Because the app never plays the story today, a rewrite could also defer it, or make it reachable
first. That too is the rewrite plan's call.

## 7. Reuse

Rating: **`superfrog-only`** for the whole. 7 224 lines (`python tools/architecture.py totals`).

The content is this game's: the scripts, the fades as Team 17 wrote them, the object and frame
formats, the boot program's lists, the end sequence's order, the measured tables. Only the method
carries: transcribe a cut-scene's main program as `Op`s, compare it region by region, measure what
cannot be derived.

**The bus simulator, rated separately: `small-changes`.** `sim.rs`, `cpu.rs` and `mem.rs` (about
1 100 lines) model OCS hardware, not Superfrog: a 68000's bus accesses in Moira's order, vAmiga's
copy blitter, refresh, the copper's and bitplanes' cycles and audio slots, and an Amiga's chip and
slow RAM. They already serve a second user in this engine, the slot machine's draws. Another Amiga
game whose effects race the beam could use them with local changes:

- the instruction table `cpu::op` grows with the instructions that game's timed code uses (it holds
  only what the intro needs);
- the blitter's line and fill modes, absent here, would be added if that game uses them;
- `StoryMem`'s sizes for a machine with other memory (fast RAM, 1 MB chip).

The `Prog` approach (each routine building its own CPU program beside its memory work, as `bobs.rs`
does) carries as a method, but each program is written per routine. The fitted parts of `intro.rs`
(`handler_lines`, the unpack tables) and the conformance feeds are this game's. A generic Amiga
toolkit would take `sim.rs` and `cpu.rs` with `copper::run_bus` ([display-hardware](display-hardware.md)).

Another Team 17 game of the period may share the boot program's logo and fade routine and the sound
driver's intro copy. This is unchecked.

## 8. Sources

- [docs/13 "The disk-3 intro"](../13-front-end.md#the-disk-3-intro-t32-code-oracle) and
  [docs/13 "The end sequence"](../13-front-end.md#the-end-sequence-t35-code-oracle): scene table,
  timings, skips, hand-over, the ending's flow and calibration.
- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1 (the checks and the tolerance).
- `plans/rust-engine/FINDINGS.md` rows for T46, T48, T49, T52–T56, T96, T97, T213, T214; task docs
  `plans/rust-engine/tasks/T46-story-intro-scenes.md` and `plans/rust-engine/tasks/T52-story-blitter-bus-sim.md`;
  commits 132db9d, 6104a5d, c032267, 698c3c7, afe1e93, 85e713f.
- Code: `engine/crates/sf-core/src/front/story/`, `engine/crates/sf-core/src/front/ending.rs`,
  `engine/crates/sf-core/src/front/anim.rs`, `engine/crates/sf-core/src/game.rs` (`Scene::Story`),
  `engine/crates/sf-core/src/render/copper.rs` (`run_bus`), `engine/crates/sf-conform/tests/story.rs`,
  `engine/crates/sf-conform/tests/story_blits.rs`, `engine/crates/sf-conform/tests/story_draws.rs`,
  `engine/crates/sf-conform/tests/channels.rs`, `engine/crates/sf-conform/tests/front.rs`.
