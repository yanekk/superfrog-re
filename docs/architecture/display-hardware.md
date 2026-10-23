+++
id = "display-hardware"
title = "The display hardware"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 76, game = 5, data = 9, shell = 10 }
reuse = "small-changes"
owns = [
  "engine/crates/sf-core/src/render/",
  "engine/crates/sf-core/src/level/bobs.rs",
  "engine/crates/sf-core/src/front/blit.rs",
  "engine/crates/sf-core/src/front/sprite.rs",
]
+++

# The display hardware

## 1. What it is

The engine never draws a frame the modern way, from "what is on screen". It keeps the memory the
original's display reads (the copper lists, the status panel's bitplanes, the two display buffers,
the sprite data) as the original's bytes at the original's addresses. Then it plays the display
chips over that memory. The copper list is run cycle by cycle on the original's bus model. Each
line's bitplanes are fetched from wherever the copper pointed them. The eight hardware sprites are
fetched as Agnus's sprite DMA fetches them. The result is composed as Denise would show it. So the
picture is a function of the state alone, and anything the original's blits or CPU writes left in
memory shows up as it did on the Amiga. [docs/12](../12-display.md) has the original's side: the
level copper list line by line, the copper's timing, the fetch pipeline, the scroll, the bobs and
the sprites. This chapter covers how the engine copies that, and which parts are the Amiga's and
which are Superfrog's.

Files owned:

- `engine/crates/sf-core/src/render/mod.rs` is the renderer. `DisplayData` is what the disks
  decide: hunk 2's contents, the frog's sprite frames (`hero_sprites`, the port of `load_hero`'s
  conversion), each world's tiles and bob sheet. It also carries the other subsystems' disk data
  (`rom`, `sound`, `front`, `story`, `levels`) as a convenience. `Mem` reads the hardware's view of
  memory, from the state, the story's flat RAM or `DisplayData`'s regions, and undoes the frame's
  logged writes the beam had not yet passed. `Denise::walk` steps the frame line by line, applying
  the copper's writes at their beam positions. It turns colour writes inside a line into `Split`s,
  advances the bitplane pointers by their modulos and runs each sprite's DMA. `level`,
  `level_as_drawn`, `level_widened`, `screen`, `story_screen` and `copper_writes` are the entry
  points, and `SpriteDma` follows DMACON's sprite bit on screens that turn it off.
- `engine/crates/sf-core/src/render/copper.rs` is the copper. `run_bus` runs a list for one frame
  on the bus model: even DMA cycles, two per instruction word, never cycle `$E0`, losing the cycles
  the bitplane fetch takes (`Fetch::takes`). WAIT and SKIP wake up as vAmiga's comparator does
  (`comparator`, `find_match`, `horizontal_match`). It returns every register `Write` with its beam
  position, and optionally every bus cycle it took, for the CPU and blitter models that need to
  know which cycles are free. `Fetch` is the fetch set-up (BPLCON0, DIWSTRT/STOP, DDFSTRT/STOP).
- `engine/crates/sf-core/src/render/panel.rs` holds `panel::fetch`, which turns one fetched line
  into colour indices. Despite its name it fetches every bitplane line, the play field's included.
  Each plane's words are read at their own fetch slot, so a write the beam is racing is seen on
  the right side of it.
- `engine/crates/sf-core/src/render/playfield.rs` follows the play field. `BUFFERS` are the two
  display buffers. `Lines::line` maps a fetched buffer line through the 19-row ring to its level
  row and level x. `Field` draws the map's tiles for widescreen's extra columns.
- `engine/crates/sf-core/src/render/objects.rs` holds the hardware sprites. `Sprite` is one
  channel (pointer, vertical window, position, control, data, armed), and `compose` builds the
  picture: fetched indices, widescreen's sides, then the sprites by pair priority against BPLCON2's
  PF2P, with attached pairs as 16-colour sprites.
- `engine/crates/sf-core/src/render/view.rs` holds what the renderer hands out. `Picture` is
  palette indices inside the display window, one 32-colour palette per line and the mid-line
  `Split`s. `to_rgb12` and `Picture::rgb12` give the 12-bit colours with extra-half-brite. `View`
  is the widescreen setting, and `Picture::widened`, `narrowed` and `rewindow` reframe a picture.
- `engine/crates/sf-core/src/level/bobs.rs` is `bob_setup` (h0+$1995a), the level's bob
  preparation. It fills in the frame records from their tables (`frame_records`), gives them their
  blit parameters (`blit_params`), builds the line offsets and the bob records' four sub-records
  (`sub_records`), and builds the sheet's mask with the blitter (`masks`). Then it points the
  frog's and the panel digits' sprite moves in the level copper list (`sprite_attach`).
- `engine/crates/sf-core/src/front/blit.rs` is the front end's blitter in area mode. `Blit::run`
  covers ascending and descending, both shifts, first and last word masks and every minterm
  (`logic`), on `State` or the story's RAM (`BlitMem`). `Blit::cycles` is a rough duration and
  `word_times` the beam position of each word of a blit with priority.
- `engine/crates/sf-core/src/front/sprite.rs` holds the front end's sprite records (h0+$1b208
  `attach`, h0+$1b23e `update`). They put a sprite or an attached pair on screen by rewriting the
  SPRxPTH and SPRxPOS/CTL moves of a copper list, and step its animation.

### What the renderer produces, and what is the app's

`Picture` is Amiga output. Its fields are what Denise produces for one field: an index per lores
pixel inside the window DIWSTRT and DIWSTOP open (`left`, `top`, `width`, `height` in the
hardware's own units), the 32 colour registers per line as the copper leaves them at the window's
left edge, and the colour writes the copper makes inside a line (`Split`, from lores pixel `2h`).
The oracle reports its picture in the same terms, so the two compare pixel for pixel
(section 4).

Widescreen is the app's extension (rust-engine DESIGN §2.7). `View::extra_columns_each_side`
widens the window on both sides, and `left` may then be negative. On a level's lines the extra
columns are drawn from the map in level space (`playfield::Lines`, `Field`), because the display
buffers hold only the original's 23 words. Outside a level the extra columns show the border
(colour 0, `Picture::widened`), and `level_widened` widens the level's own fades from the level
behind them. The centre stays exactly the 4:3 picture. Sprites outside the original window are
not drawn into the sides, and objects the original has not activated are absent: the accepted
cost of DESIGN §2.7.

### Where the mechanism is used

- **The level.** `Scene` in `engine/crates/sf-core/src/scene.rs` calls `render::level_as_drawn`
  with the pass's write log (`WriteLog` in `engine/crates/sf-core/src/mem.rs`). Writes to the
  copper list, the panel and the display buffers made after the beam passed their line are undone
  for the lines above them, so a late redraw shows as the original showed it. The level's own
  blits are a second blitter model, `blit` in `engine/crates/sf-core/src/level/objects.rs`
  (ascending only), used by `bob_draw`, the tile animations and `bob_setup`'s mask. Their cost on
  the beam is `LevelCtx::blit_words` and `Clock::blit` in `engine/crates/sf-core/src/level/beam.rs`
  ([cpu-time-and-beam](cpu-time-and-beam.md)).
- **The front end.** `engine/crates/sf-core/src/front/mod.rs` calls `render::screen` with a
  `copper_word(addr, line)` that reads the list as the copper saw it on each line. The CPU rewrites
  front-end lists during a frame, and the history of those rewrites is kept per line. It re-renders
  the lines the beam drew before a part of the main program ran from the memory before it, and
  carries `Fetch` from frame to frame. The title, menu, high score, credits, progress map and
  attract screens use `front/blit.rs` and `front/sprite.rs`. The credits time their blits against
  the beam with `word_times` (`engine/crates/sf-core/src/front/attract.rs`). The mouse pointer is a
  sprite record updated from the vertical blank (`engine/crates/sf-core/src/front/vbl.rs`).
- **The slot machine.** `engine/crates/sf-core/src/slot/draw.rs` uses `copper::run_bus` for the
  cycles the copper takes, so its draws land where the original's do. It reports the lines it
  writes near the beam as `FrameOut::beam_races`.
- **The story intro: the finer instance.** The story is a separate program whose draws race the
  beam to the cycle. It renders through `render::story_screen` over its own flat RAM. Its blits use
  `front::blit::Blit` and are timed on its bus simulator, which takes the copper's cycles from
  `copper::run_bus` (`engine/crates/sf-core/src/front/story/sim.rs`,
  `engine/crates/sf-core/src/front/story/cpu.rs`, `engine/crates/sf-core/src/front/story/intro.rs`).
  [story-and-ending](story-and-ending.md) documents that race.

## 2. Kind mix

`amiga` 76, `game` 5, `data` 9, `shell` 10. Judged by reading each file and weighing its lines by
the parts below. It is an estimate, not a count.

- **`amiga`.** Nearly all of `render/copper.rs`, the `Mem`/`Planes`/`Denise` half of
  `render/mod.rs`, `render/panel.rs`, `render/objects.rs`, `Picture`, `Split` and `to_rgb12` in
  `render/view.rs`, and all of `front/blit.rs`. In `level/bobs.rs`, the blit parameters, the mask
  blits and the copper sprite moves. In `front/sprite.rs`, the SPRxPOS/CTL encoding. All of it
  copies what OCS chips do with memory and registers, or what a routine writes into chip memory
  for them.
- **`data`.** `DisplayData::new` and `hero_sprites` in `render/mod.rs`, which build hunk 2's
  contents and the frog's sprite frames from the disks as the original's boot and `load_hero`
  lay them out. In `level/bobs.rs`, `frame_records` copying the world's frame tables out of the
  executable (`Rom`, h0+$2c9b4 and the table `level_init_wN` names) and the copied sprite moves
  (h0+$19ac2).
- **`shell`.** Widescreen: `View`, `Picture::widened`, `narrowed` and `rewindow`,
  `render::level_widened`, and `playfield::Field` with the map half of `playfield::Lines`. It is a
  presentation option of the app, not the Amiga's and not the game's.
- **`game`.** In `front/sprite.rs`, the record's animation stepping. In `level/bobs.rs`, which
  bobs exist and how their records start (`park_free`, the first animation step in `sub_records`).
  These are Superfrog's bookkeeping rather than the hardware's.

## 3. Mechanisms it uses, and where else this one is used

This chapter documents the [glossary](glossary.md)'s **copper run**, **bitplane fetch**,
**hardware sprites** and **the blitter**. It leans on:

- **Byte-exact state at original addresses** ([memory-model](memory-model.md)). Everything the
  renderer reads is a `LAYOUT` variable or a `DisplayData` region at its original address:
  `level_copper`, `panel_bitmap`, `playfield_buffer_0` and `_1`, `playfield_clean`, `hero_frames`,
  `bob_records`, `bob_frames`. In `bob_setup` and the sprite records the "registers" are words
  inside a copper list in that memory.
- **The loaded hunk image and boot-built tables** ([copied-in-data](copied-in-data.md)). Hunk 2's
  block, the frog's frame tables (h0+$1a6f4, h0+$19ddc) and the bob frame tables are read out of
  the executable.
- **Beam position and cycle charging** ([cpu-time-and-beam](cpu-time-and-beam.md)). The write log
  that makes `level_as_drawn` exact is stamped with the beam position `Clock` gives each write.
  The level blitter's cost on the beam is charged there, not here.
- **The vertical-blank and copper interrupts** ([frame-interrupts-input](frame-interrupts-input.md)).
  `scroll_latch` sets the play field's pointers and scroll in the copper list at the blank, and the
  copper interrupt's blits scroll new tiles into the buffers. The renderer shows whatever those
  left.

Where else the mechanisms appear, outside the files this chapter owns:

- `engine/crates/sf-core/src/level/objects.rs`: `blit` and `Blit`, the level's own blitter model
  (ascending area mode), with `bob_draw`'s erase and cookie-cut (minterm `$CA`).
- `engine/crates/sf-core/src/level/scroll.rs`: the scroller's tile blits, by its own `blit`.
- `engine/crates/sf-core/src/front/story/sim.rs` and `engine/crates/sf-core/src/front/story/bobs.rs`:
  the story's blits on `front::blit::Blit`, timed on the bus.
- `engine/crates/sf-core/src/slot/draw.rs`, `engine/crates/sf-core/src/slot/mod.rs`: the slot
  machine's blits and copper bus cycles.
- `engine/crates/sf-core/src/front/chain.rs`, `engine/crates/sf-core/src/front/menu.rs`,
  `engine/crates/sf-core/src/front/hiscore.rs`, `engine/crates/sf-core/src/front/vbl.rs`: the
  front end's sprite records (`sprite::update`).
- `engine/crates/sf-core/src/frog/`: the frog's sprites, placed by rewriting the level copper
  list's sprite moves (`place_frog_sprites`, `sprite_to_copper`).

### Generic OCS against Superfrog's own

This split drives the reuse rating (section 7).

| Generic OCS imitation | Specific to Superfrog |
|---|---|
| `copper::run_bus` and its comparator, `Fetch` (fetch window, plane order, OCS VSTOP bit 8), `Write`, `Beam` | `LEVEL_COPPER` (h2+$6bd0) and the list's layout ([docs/12 "The copper list"](../12-display.md#the-copper-list)): two palettes for the panel, the black lines, the ring seam's reload, the second reload past line `$ff` |
| `Denise::walk`: copper writes at beam positions, mid-line colour splits, modulos, the pipeline constant `PIPELINE` (17) and `HIRES_PIPELINE`, sprite clipping to lines with a fetch | The double buffer and its 19-row ring (`playfield::BUFFERS`, `RING`, `RING_OFFSET`, `COLUMN_OFFSET`), which only widescreen needs |
| `panel::fetch`: one line's planes into indices, in lores or hires, each plane at its fetch slot | The panel's five planes of 14 lines, whose last two lines read into the frog's frames (a property of where Superfrog put them, reproduced by reading memory as it is) |
| `objects::Sprite` and `compose`: sprite DMA from line 25, VSTART/VSTOP, a control write disarming, attached pairs, PF2P priority | `hero_sprites` (`load_hero`'s conversion) and `HERO_SPRITES`; `DisplayData`'s hunk 2 block (`accessing-picture`) |
| `Picture`, `Split`, `to_rgb12`, extra-half-brite | `level/bobs.rs` entire: the frame record, sub-record and bob record formats, the sheet (`LnBO`, 320 × 320, five planes) and its mask at h3+$48190 |
| `front::blit::Blit::run`, `logic`, `word_times`, `SpriteDma` | `front/sprite.rs`'s record format (`$1c` bytes) and its routines h0+$1b208, h0+$1b23e |

The generic column is about two thirds of the 2 421 lines.

### What the imitation leaves out

This is what OCS can do that the engine does not model, because Superfrog does not use it. Another
game might. Dual-playfield priority (only single-playfield PF2P is followed), HAM, interlace,
sprite and playfield collision registers, the blitter's line and fill modes, ECS and AGA
registers, and the copper's danger bit. The level's blitter model in
`engine/crates/sf-core/src/level/objects.rs` also lacks descending mode. The front end's has it.

## 4. What depends on it

- **[verification](verification.md).** Check 2 of rust-engine DESIGN §2.1 compares the picture on
  every stepped case. The `render-*` cases compare it exactly on transplanted memory (below).
  Without this renderer neither has an engine side.
- **[app](app.md).** It shows `FrameOut::picture` through `to_rgb12`, scaled, filtered and
  widened (`engine/crates/sf-app/src/video.rs`), and its capture writes the same picture.
  `sf-e2e`'s view tests read it back.
- **[level](level.md), [frog-and-items](frog-and-items.md), [enemies-and-bosses](enemies-and-bosses.md).**
  Their blits and sprite moves only become visible through this renderer. `bob_setup` prepares the
  records `bob_draw` uses.
- **[front-end](front-end.md), [slot-and-bonus](slot-and-bonus.md), [story-and-ending](story-and-ending.md).**
  They draw with `front/blit.rs` and `front/sprite.rs` and render with `render::screen` or
  `story_screen`. The story and slot machine time their CPU and blitter against
  `copper::run_bus`'s bus cycles.
- **[game-flow](game-flow.md).** `Scene` decides which list is shown (`LEVEL_COPPER` or the front
  end's `cop1lc`) and classifies scenes partly by it (`SceneKind::classify`).

### What the picture check compares

There are two comparisons, both on 12-bit colour per pixel (`to_rgb12`) inside the display window,
in `engine/crates/sf-conform/src/compare.rs`.

- **Exact, on transplanted memory: the `render` compare.** Cases
  `engine/conform/cases/render-*.toml` cover every level of W1–W6 (W6-2 to 4 in both map sets), the
  bonus level and the final level, 400 frames each. The original's memory at each frame is
  transplanted into a fresh engine and rendered, and the result must equal the oracle's picture
  (`picture_except`), window included. A frame is left out only as `LATE_REDRAW` or `NOT_A_LEVEL`
  (`engine/crates/sf-conform/src/lib.rs`), listed with the reason. A transplant cannot know a write
  that came after the beam passed.
- **Within tolerance, on stepped runs: check 2.** `classify` and `PictureJudge` with
  `Tolerance::OWNER`. A frame is tolerated when every differing pixel is at most one colour step
  off, or fewer than 64 differ. Any other difference may last at most 2 frames in a row and cover
  at most 1% of a case's frames. A differing display window is never tolerated. Rows in
  `FrameOut::beam_races` are excused (`raced_only`) when a draw wrote within `RACE_LINES` of the
  beam.

So a native renderer must reproduce, pixel for pixel in the exact compare:

- the display window as DIWSTRT/DIWSTOP set it, in hardware units;
- mid-line colour changes where the copper's bus timing puts them (the panel's palette switch on
  line `$28`);
- the fetch boundaries: DDFSTRT, the pipeline delay, BPLCON1's scroll per playfield, the modulos,
  and the pointer reloads at the ring seam;
- sprites as DMA fetches them, including where they are clipped (no sprite on a line without a
  bitplane fetch, so the frog vanishes on the two black lines under the panel);
- the memory as the beam saw it, not as the frame ended (`level_as_drawn`'s write log, the front
  end's per-line copper history).

## 5. What a native rewrite must preserve

Faithful stays the bar. Each row is behaviour the original shows, and the check that holds the
engine to it today.

| Behaviour to keep | Check that proves it |
|---|---|
| Copper timing: two cycles a word on even cycles, never `$E0`, the vertical blank taking cycle 0, one even cycle in four lost to a fifth plane inside the fetch window, WAIT waking with the comparator two cycles ahead. | `moves_take_two_cycles_a_word_and_a_wait_holds_until_its_line`, `the_copper_takes_the_vblank_cycle_and_its_fetches_but_not_a_waits`, `a_fifth_plane_slows_the_copper_inside_the_fetch_window` (`engine/crates/sf-core/src/render/copper.rs`); every `render-*` case. |
| A WAIT reached past cycle `$E0` of its line never fires that frame (the level list's second seam reload, docs/12). | `a_wait_reached_past_cycle_e0_of_its_line_waits_for_the_next_frame` (`engine/crates/sf-core/src/render/copper.rs`); the `render-*` cases whose seam is above line `$100`. |
| Colours written inside a line change from lores pixel `2h` on: the panel's palette switched mid-line on line `$28`. | `copper_palette_effects_match_for_100_frames` (`engine/crates/sf-conform/tests/render_vs_oracle.rs`); `a_split_recolours_its_line_from_its_pixel_on` (`engine/crates/sf-core/src/render/view.rs`). |
| OCS vertical window, with VSTOP's bit 8 the inverse of bit 7; bitplane DMA only inside it, so the panel's pointers advance from line `$27`. | `the_vertical_window_and_ocs_vstop_bit_8` (`engine/crates/sf-core/src/render/copper.rs`); the `render-*` cases. |
| The play field fetched from the shown display buffer through the scroll: BPLCON1 delay, pointer column, ring reload; the pipeline constant 17. | `w1_1_renders_as_the_original_while_it_scrolls_both_ways` (`engine/crates/sf-conform/tests/render_vs_oracle.rs`, camera moving ≥160 px and ≥32 px); `scroll_latch_matches_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); every `render-*` case. |
| The display buffers are state: tiles, tile animations, bobs and scrolled-in rows as the original's blits left them, not drawn from the map. | `bob_draw_matches_the_original`, `tile_anim_draw_matches_the_original`, `buffer_flip_matches_the_original`, `copper_irq_blits_match_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); the `render-*` cases on W2–W5, where drawing from the map was wrong on nearly every frame. |
| The panel: five planes of 14 lines `$2a0` apart, its last two lines read past the fifth plane into the frog's frames. | The `render-*` cases (panel rows). No dedicated test. |
| Sprites: control words fetched on line 25, data between VSTART and VSTOP, the next control at VSTOP; a control write disarms (mid-line, from its pixel); shown at `HSTART + 1`; none on a line without a bitplane fetch; attached pairs; PF2P priority. | `sprite_registers_and_their_numbers` (`engine/crates/sf-core/src/render/objects.rs`); `place_frog_sprites_matches_the_original`, `the_frog_sprites_wait_out_lines_1d_to_21` (`engine/crates/sf-conform/tests/frog_routines.rs`); the `render-*` cases (the frog and the coin digits). |
| The frog's sprite frames as `load_hero` builds them at h3+$4c0b0 (docs/12 calls them byte-identical on the oracle). | `every_level_is_entered_as_the_original_enters_it` (`engine/crates/sf-conform/tests/level_entry.rs`) compares every `LAYOUT` piece after entry, `hero_frames` among them; the `render-*` cases draw the frog from them. |
| `bob_setup`: frame records, blit parameters, sub-records, line offsets, the mask as the OR of the sheet's planes, the sprite moves in the level list. | `every_level_is_entered_as_the_original_enters_it` (`bob_frames`, `bob_records`, `hunk3_gap` holding the mask, `level_copper`). |
| Sprite DMA turned off by DMACON on a screen (the slot machine's list on line 107, the progress map's blinking marker). | The slot machine cases (`engine/crates/sf-conform/tests/front.rs`, `the_slot_machine_plays_its_rounds_as_the_original_does`, case `engine/conform/cases/slot-fire.toml`) under check 2; `game-whole` (`engine/conform/cases/game-whole.toml`) through the progress map. |
| The front end's blitter: shifts, masks, minterms, ascending and descending. | `minterms` (`engine/crates/sf-core/src/front/blit.rs`); every front-end case's check 2 (`power_on_to_the_title_matches_the_original`, `the_attract_cycle_matches_the_original`, `the_level_code_entry_matches_the_original` in `engine/crates/sf-conform/tests/front.rs`). |
| Blits racing the beam land on the original's cycles (the story's masks and draws; the credits' `word_times`). | `scene_1s_masks_and_the_first_draw_begin_free_and_end_on_the_originals_cycles` (`engine/crates/sf-conform/tests/story_blits.rs`); `draws_write_their_blits_on_the_originals_cycles` (`engine/crates/sf-conform/tests/story_draws.rs`); `the_attract_cycle_matches_the_original` for the credits. |
| The front end's sprite records: SPRxPOS/CTL encoding (x + `$80`, y + `$2c`, bit 8 of start and stop), attached pairs' second sprite, animation stepping. | The menu, level code and high-score cases (cursor, eye, pointer) under check 2: `the_level_code_entry_matches_the_original`, `the_hiscore_name_entry_after_game_over_matches_the_original` (`engine/crates/sf-conform/tests/front.rs`). No unit test. |
| The picture as the beam drew it: writes after the beam passed a line not shown on it (a late redraw); lines drawn before part of the main program ran show the older memory. | Check 2 on every stepped level case (frog, enemies, completion cases), which a transplant-only renderer would fail on about 4% of level frames ([docs/12 "Late redraws"](../12-display.md#late-redraws)); `a_frame_ends_in_bob_draw_after_the_bobs_the_original_had_drawn` (`engine/crates/sf-conform/tests/items_routines.rs`). |
| Extra-half-brite: indices 32–63 show half the colour. | `extra_half_brite_halves_the_colour` (`engine/crates/sf-core/src/render/view.rs`). Whether any Superfrog screen uses it is not established here; it costs nothing to keep. |
| Widescreen (the app's, not the Amiga's): the centre equals the 4:3 picture; off the map is colour 0; outside a level the sides are the border. | `widescreen_keeps_the_centre_and_shows_colour_0_off_the_map` (`engine/crates/sf-conform/tests/render_vs_oracle.rs`); `widescreen_mid_level_keeps_the_centre_and_the_game_of_the_4_3_run`, `widescreen_at_a_level_s_left_edge_shows_colour_0_off_the_map_and_nothing_wraps`, `widescreen_on_the_title_story_intro_and_slot_machine_shows_the_border_at_the_sides` (`engine/crates/sf-e2e/tests/view.rs`); `a_buffer_line_maps_to_its_level_row_through_the_ring`, `past_the_map_s_edges_the_level_is_colour_0` (`engine/crates/sf-core/src/render/playfield.rs`). |

## 6. Native sketch (non-binding)

This sketch is non-binding: designing the rewrite is the later plan's job.

The picture contract could stay as it is: `Picture` with indices, a palette per line and splits is
already the form the checks compare, and a native renderer would emit it from different inputs.
What changes is what it reads. One option keeps the display memory (copper list, buffers, panel)
and only replaces the cycle-by-cycle copper run with a precomputed schedule of where each write of
Superfrog's few lists lands. That is the smallest step, because the exact compare already proves
the list's write positions. Another option replaces the display memory with a scene description
(a palette per band of lines, a scroll, tile layers, sprite positions) and draws the play field
from the map plus an overlay of the tiles and bobs the buffers held. The buffers drift from the
map, though (objects write tiles, a bob over a changed tile stays gone until that buffer is
redrawn), so that overlay must be state with the same lifetimes as the original's two buffers and
clean buffer. Either way the per-line beam view (late redraws, the front end's copper history, the
story's races) needs a "what the beam had seen by this line" input, or the picture check's late
redraw frames come back as failures. The two blitter models (level and front end) could be one,
and `bob_setup`'s blit parameters would vanish if bobs were drawn natively. The exact `render`
compare depends on transplanting memory, so a renderer that no longer reads memory needs the
address coupling resolved first ([memory-model](memory-model.md), [verification](verification.md)).

## 7. Reuse

Rating: **`small-changes`** for the whole. 2 421 lines (`python tools/architecture.py totals`).

The generic OCS part, about two thirds (section 3's table: the copper, Denise's walk, the fetch,
sprite DMA and composition, `Picture`, the front end's blitter), carries to another Amiga game.
The changes would be local:

- `Mem`'s regions: what `DisplayData` puts at which address, and which `LAYOUT` variables the
  display reads.
- The entry points that name `LEVEL_COPPER`.
- `level_as_drawn`'s write-log ranges (`WriteLog::shown` in `engine/crates/sf-core/src/mem.rs`).
- Whatever the other game uses that is in section 3's "leaves out" list: dual playfield, HAM,
  interlace, collisions, line and fill blits. Each is an addition rather than a rewrite.

The Superfrog part is `superfrog-only`. That is `level/bobs.rs`, `front/sprite.rs`'s record
format, `hero_sprites`, `playfield.rs`'s ring and widescreen mapping (only the method carries:
widescreen needs the game's own map), and the `DisplayData` fields belonging to other chapters'
disk data. The pacing and placement of writes on the beam lives in
[cpu-time-and-beam](cpu-time-and-beam.md) and is not counted here.

Another Team 17 game of the period may share the bob record layout and the sprite record routines
(h0+$1b208, h0+$1b23e). This is unchecked.

## 8. Sources

- [docs/12 "The level display"](../12-display.md): the copper list, copper timing, fetch, scroll,
  bobs, sprites, palettes, late redraws.
- [docs/06 "Graphics & level rendering"](../06-graphics-and-rendering.md): tiles and palettes.
- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1 (check 2, the tolerance) and §2.7
  (widescreen).
- Code: `engine/crates/sf-core/src/render/`, `engine/crates/sf-core/src/level/bobs.rs`,
  `engine/crates/sf-core/src/front/blit.rs`, `engine/crates/sf-core/src/front/sprite.rs`,
  `engine/crates/sf-core/src/level/objects.rs` (`blit`, `bob_draw`),
  `engine/crates/sf-core/src/scene.rs` (`level_as_drawn` with the pass's log),
  `engine/crates/sf-core/src/front/mod.rs` (`render::screen`, the copper history),
  `engine/crates/sf-core/src/mem.rs` (`WriteLog`), `engine/crates/sf-core/src/game.rs`
  (`FrameOut::beam_races`, `RACE_LINES`), `engine/crates/sf-conform/src/compare.rs`
  (`picture_except`, `classify`, `Tolerance::OWNER`).
