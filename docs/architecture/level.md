+++
id = "level"
title = "The level"
tier = "subsystem"
depth = "deep"
kind = "amiga"
mix = { amiga = 50, game = 43, data = 6, shell = 1 }
reuse = "superfrog-only"
owns = [
  "engine/crates/sf-core/src/level/",
]
+++

# The level

## 1. What it is

This is the part of the engine that plays a level once it is entered: the map loaded and unpacked,
the objects spawned from its markers, the camera following the frog, the play field scrolled and
its new tiles drawn, the frog checked against the map's cell markers, the 20 object slots updated,
collided and drawn as bobs, death and the clock, the exit, the exit fade and the tally, the status
panel, the palette fades and the pass's own routines (buffer flip, tile animations, secret
passages, pause). It is the core the map editor exercises, because everything a level does starts
from its `BODY` grid. The frog's own movement is [frog-and-items](frog-and-items.md)'s, the object
kinds are [frog-and-items](frog-and-items.md)'s and [enemies-and-bosses](enemies-and-bosses.md)'s,
and the level's frame (when the vertical blank and the 25 Hz pass run, and where the beam is) is
[frame-interrupts-input](frame-interrupts-input.md)'s and [cpu-time-and-beam](cpu-time-and-beam.md)'s.

Every routine here is a port of the original's routine of the same address, working on the
original's memory as the engine models it ([memory-model](memory-model.md)): `Bus` reads and
writes bytes at original addresses, and a slot, a bob record or the map is a range of those bytes,
not a Rust type. [docs/08 "Main loop and interrupt"](../08-executable-map.md#main-loop-and-interrupt)
and [docs/09](../09-object-system.md) describe the original's side.

Files owned (`engine/crates/sf-core/src/level/`, less `beam.rs`, `timeline.rs`, `frame.rs`,
`clock.rs` and `bobs.rs`, which other chapters own):

- `engine/crates/sf-core/src/level/mod.rs`. `LevelCtx` is what every ported level routine works
  on: the `Bus`, the `Dispatch`, the frame's `Hw` (the joystick as `JOY1DAT`, the fire button as
  CIA-A PRA, the keyboard's serial byte), the sound data and the frame's Paula writes. Most of its
  ~800 lines are the beam machinery: `spend` charges cycles, `custom` records DMACON, INTENA and
  INTREQ writes in order with sound's, and `interrupt_due_at`, `irq_due`, `frame_end`,
  `pass_returns`, `step_done`, `point`, `step_at` and `walk_step` end the original's frame where it
  ended, mid-pass. `sfx` is `sfx_play`'s raster wait with interrupts off, and `request_copper` takes
  the copper interrupt now or once INTENA unmasks it. `Dispatch` maps the original's template
  indices, routine addresses and `(world, marker)` pairs to ported functions (`register`,
  `register_routines`, `register_marker`, `standard_dispatch`, `frog_only_dispatch`). `call` runs
  a slot routine by its original address, and reports one with no port (`LevelCtx::unported`).
  `LevelOutcome` and `Tally` are what the level hands the game flow.
- `engine/crates/sf-core/src/level/load.rs` enters a level (`enter`, `enter_with`): `boot_image`
  (hunk 0 and hunk 2 as the loader leaves them, the frog's sprite frames), `world_sound`,
  `new_game`, then `level_load` (the clock, the world's bob sheet and `TILE_HEIGHTS`, the map file
  into the load buffer, `palette_from_map`, `map_unpack`, `count_coins`, `find_marker_cells`,
  `find_start`), `level_init` (slots freed, the world's tables, `frog_reset`, `bob_setup`,
  `panel_clear`, `start_point` with its first scroll and screen scan) and `level_start`
  (`start_frog`, `fade_in`, the panel, `drop_in`). `current` reads the level the state is in.
- `engine/crates/sf-core/src/level/spawn.rs` brings objects into play from the map's markers:
  `scan_edges` (`map_spawn_scan_edges`, the column and row just off screen on the sides the camera
  moved to), `scan_screen` (the whole screen, from a start point) and `scan_right_column` (the
  bonus level).
- `engine/crates/sf-core/src/level/scroll.rs` scrolls the play field. `scroll_latch` writes the
  camera into the level copper list: BPLCON1's delay, the bitplane pointers into the shown buffer's
  ring row and line, and the ring seam's wait and reload. `scroller` draws the rows or columns that
  came into view into the clean buffer on alternate frames (`draw_rows`, `draw_cols`, the bonus
  level's `shooter_scroll`) and queues their copies, which `copper_irq_blits` blits into both
  display buffers from the copper interrupt.
- `engine/crates/sf-core/src/level/camera.rs` is `camera_follow` (h0+$db92): the lag (`lag`), the
  edges (`keep_inside`), `scroll_dirs`, the bonus level's creep (`shooter`) and the frog's two
  sprites' screen position (`place_sprites`).
- `engine/crates/sf-core/src/level/collision.rs` is the frog against the map: `frog_tiles`
  (h0+$e2e2; gravity, `head_probe`, `ground_probe`, `slope_anim`, `walls`, then the markers in the
  three cells down the frog's middle through `handle`), the marker handlers (walls, springs,
  launcher, bounce pads, fake wall, ice, conveyors, push surfaces, switches, the exit, hazards, tile
  pickups) and `frog_kill`. `cell_at` is the original's cell address arithmetic.
- `engine/crates/sf-core/src/level/objects.rs` is the objects' side of the pass: `obj_run_all`, the
  spawners (`obj_spawn_cell`, `obj_spawn_cell_alt`, `obj_spawn_coin_forced`, `obj_spawn_at_xy`,
  `obj_spawn_at_xy_nodedup`, sharing `fill_slot`), `obj_free`, `obj_free_all`,
  `obj_despawn_if_offscreen`, `obj_build_hitboxes`, `overlapping`, `collide_all` (the spud, then the
  frog), `build_bob_draw_list` and `bob_draw` with its own ascending-only blitter (`blit`, `Blit`),
  erasing and cookie-cutting every bob into the back buffer in two parts split at the ring seam.
- `engine/crates/sf-core/src/level/death.rs`: `frog_hurt` (a heart lost), `time_up_check` and
  `frog_die` (time up ends the game).
- `engine/crates/sf-core/src/level/exit.rs`: what follows the level's loop, run on the front end's
  program (`crate::front`). `LevelOver` is `level_exit_fade` (the palette faded out while DIWSTRT
  closes the window a line a frame) and the choice after it, `outcome_of_loop` reads how the loop
  ended, and `Restart` and `LevelStart` set the level up again and start it on the beam
  (`init_on_beam`, `with_level`, `start_slide`, `start_after_slide`).
- `engine/crates/sf-core/src/level/tally.rs`: `LevelTally` (`level_tally`, h0+$4466), the tally
  picture unpacked from hunk 0, `pages` (the pages earned for time, tile coins and coins),
  `hide_icons`, the text printer `print`, the choice between points and the slot machine.
- `engine/crates/sf-core/src/level/panel.rs`: the status panel's contents. `panel_update` draws the
  lives icon, one score digit and one clock digit a pass. Also `number_to_text`, `extra_life`,
  `lives_count`, the coins' two sprite digits (`sprite_digits`) and the slide-in at the start
  (`slide_begin`, `slide_step`, `draw_at_start`).
- `engine/crates/sf-core/src/level/pass.rs`: `buffer_flip`, `tile_anim_draw` and `redraw_cell`,
  `passage_step` (a secret passage opening, [docs/07](../07-secret-passages.md)), and `pause_check`
  and `pause_run` with `Pause`, the pause kept across frames.
- `engine/crates/sf-core/src/level/fade.rs`: the four fade records the vertical blank steps
  (`fades_tick`: fade-in, fade-out, stepped) and `stepped_start`, the witch's flash.

### What runs each frame

The order is the original's ([docs/08 "Main loop and interrupt"](../08-executable-map.md#main-loop-and-interrupt)).
`frame::vbl` in `engine/crates/sf-core/src/level/frame.rs`, not owned here, calls this chapter's
`scroll_latch`, `frog_tiles`, `camera_follow`, `scroller` and `scan_edges` every frame while the
logic runs. Every second frame `frame::pass` runs the 25 Hz pass: the spud, `obj_run_all`,
`obj_build_hitboxes`, `collide_all`, `build_bob_draw_list`, `bob_draw`, `buffer_flip`,
`tile_anim_draw`, `panel_update`, `pause_check`, `passage_step`, `extra_life`, `time_up_check`,
then the `level_state` countdown that ends the loop (`$190` after a death, 1 at the exit). Each
call is a numbered phase (`LevelCtx::enter_phase`), because a late frame's end is matched to the
phase and the step inside it where the original's was.

### Which parts read the level's file and which read memory copied in

A level is the `T7MP` file and nothing else ([docs/04](../04-t7mp-level-format.md),
[docs/14](../14-sflevel-format.md)). `level_load` puts the file's bytes in the load buffer
(`playfield_buffer_1`, h3+$22f30) as `load_file` would, then reads only two tags from it:

- `PALA`: `palette_from_map` copies the 32 colours `$44` bytes past the tag into
  `level_palette`.
- `BODY`: `map_unpack` turns each of the 300 × 84 cells from `tile | marker << 10` into
  `tile << 6 | marker` in `level_map`, 304 cells a row (four of padding). Every later reader works
  on that unpacked copy: the markers for collision and spawning, the tile for drawing and the tile
  heights. `count_coins`, `find_marker_cells` (switch gates `$30`–`$33`) and `find_start` (marker
  `$35`, the start) are read off it once.

Everything else a level uses comes from the disks and the executable, by world rather than by
level, and is unchanged by a level file:

- The world's tiles (`TILE_PTRS`) and bob sheet (`DisplayData::bob_sheet`, `LnBO`), and
  `TILE_HEIGHTS` (h1+$31e20), each tile's surface height per pixel column. `Rom::tile_heights`
  builds that table from the world's masks file (`LnMS`) as `tile_heights` in
  `engine/crates/sf-core/src/mem.rs` ports h0+$d950. `load.rs` pokes it; the probes in
  `collision.rs` (`height`, `slope_anim`) read it.
- The boot-built tables ([copied-in-data](copied-in-data.md)): `map_rows` (h0+$17334, a pointer
  to each map row from one above the map, which every cell address goes through), `ring_rows`,
  `row_lines`, `ring_row_lines` and `line_offsets` (the display ring's geometry).
- Tables in hunk 0, read by address: `obj_templates` (h0+$1c6e2, 160 of `$ae` bytes), the
  per-world tables `level_init` stores (h0+$2ed2c…, h0+$9f2), the wall probe lists (h0+$f07a,
  h0+$f086), the slope animations (h0+$dee8, h0+$df6c), `passage_tiles` (h0+$7876), the panel's
  digit graphics tables and `powers_of_ten`, the tally's packed picture (h0+$33234) and its font
  (h0+$3414c). The panel's graphics and the slide's picture are in hunk 2.
- What a marker spawns. The original's `obj_flag_jumptab` (h0+$178ec) is not read: the object
  kinds register a ported spawner per `(world, marker)` in `Dispatch`, so a marker's meaning
  depends on the world the level is in.
- The clock (4:00, or 10:00 in the bonus and final levels), the music and the world's effects,
  set by world in `level_load` and `world_sound`.

### What the `.sflevel` path changes

`Game::with_override` in `engine/crates/sf-core/src/game.rs` encodes the file's `T7MP`
(`sf_assets::t7mp_encode`) and puts it in `DisplayData::levels` in place of the disk's map for the
level it `replaces`. The game then starts that level exactly as a disk level, through `enter` and
`level_load` (rust-engine [DESIGN §2.10](../../plans/rust-engine/DESIGN.md)). So a level file
changes exactly the two tags above: every cell's tile and marker (terrain, collision, secret
passages, every object's placement, the start, the exit, the coins counted) and the palette. It
cannot change the world: the tile set, the tile heights, the bob sheet, the object templates, what
each marker spawns, the clock, the music. Those still come from the disks and the executable by
the world `replaces` names. The map must stay 300 × 84, because `map_unpack` unpacks that many
cells whatever `XBLK` and `YBLK` say. A save state carries the override (`Game::restore_display`),
because it does not carry `DisplayData`. The app's menu is `engine/crates/sf-app/src/levelfile.rs`.

## 2. Kind mix

`amiga` 50, `game` 43, `data` 6, `shell` 1. Judged by reading each owned file, splitting it into
the parts below and weighing each part by its lines. It is an estimate, not a count. The
classification puts a rule written over original bytes (`collision.rs` reading markers through
`Bus`) under `game`, because the byte-exact state is [memory-model](memory-model.md)'s mechanism and
counted there; the dominant `amiga` share comes from the beam machinery and the blits this chapter
holds.

- **`amiga`.** In `mod.rs`, `LevelCtx`'s beam machinery (about 800 lines: frame ends inside the
  pass, the schedule and blit anchoring, `sfx_play`'s raster wait, INTENA masking of the copper
  interrupt, the held vertical blank) and `Hw`'s register encodings (`joy1dat`, `ciaa_pra`). In
  `scroll.rs`, nearly all of it: the copper-list writes of `scroll_latch` and the ring-buffer tile
  drawing and blit queue of `scroller` and `copper_irq_blits`. In `objects.rs`, `bob_draw` and its
  blitter (about half the file). In `pass.rs`, `buffer_flip`, `redraw_cell`'s blits and the pause's
  raster waits. In `panel.rs` and `tally.rs`, the panel blits, `sprite_digits`' copper moves, the
  tally's copper list, `hide_icons` and `print` into bitplanes. In `exit.rs`, the window closing
  through DIWSTRT and the measured cycle counts that put `LevelStart` and the exit fade on the
  beam. In `load.rs`, `boot_image`, `panel_clear` and the copper interrupt's masking around the
  first scroll. In `fade.rs`, that the fades step colour moves in a copper list.
- **`game`.** `collision.rs` almost entire (gravity, probes, walls, every marker's rule,
  `frog_kill`), `camera.rs` but its sprite placement, `death.rs`, `spawn.rs`, `objects.rs`'s slot,
  spawn, despawn, hitbox and touch rules, `pass.rs`'s `passage_step` and `tile_anim_draw`'s tile
  stepping, `tally.rs`'s `pages` and choice, `panel.rs`'s digit stepping and `extra_life`,
  `load.rs`'s `map_unpack`, `count_coins`, `find_start`, `start_point` and `drop_in`, `fade.rs`'s
  fade arithmetic, `exit.rs`'s `outcome_of_loop` and what follows each outcome, and `Dispatch`.
- **`data`.** In `load.rs`, the per-world table addresses `level_init` stores and the hunk images
  `boot_image` pokes. Elsewhere, reading hunk 0's tables by address: `obj_templates` in
  `fill_slot`, the probe lists, `passage_tiles`, the digit graphics tables, the tally's packed
  picture and font.
- **`shell`.** `LevelCtx::unported` and `unported_text`, which report a path the engine does not
  follow so a test fails, and the frog-only plumbing (`frog_only_dispatch`, `LevelOver::frog_only`).

## 3. Mechanisms it uses

From the [glossary](glossary.md):

- **Byte-exact state at original addresses** ([memory-model](memory-model.md)). Every variable here
  is a `Bus` access at its original address: the frog record (`FROG` and its `f::` offsets in
  `engine/crates/sf-core/src/frog/`), the camera (h0+$16660, h0+$16664), `level_state`, the map at
  h1+$218e0, the slots at h0+$1ba6c, the hitbox list at h0+$d350, the bob records and draw list,
  the scroller's queue at h0+$1675a. Signed tests follow the 68000's N flag (`n_clear`,
  `n_clear_l`), and address arithmetic wraps as the 68000's words do (`cell_at`, `cell_position`'s
  `divu` overflow).
- **The loaded hunk image and boot-built tables** ([copied-in-data](copied-in-data.md)). Section 1
  lists the tables read.
- **Cycle charging and beam position** ([cpu-time-and-beam](cpu-time-and-beam.md)). Every routine
  charges its work (`LevelCtx::spend`, `spend_n`, `Cost` in
  `engine/crates/sf-core/src/level/beam.rs`), blits charge their words (`blit_words`), and raster
  waits move the clock (`wait_lines`, `sprite_wait`, `wait_blank`). The front-end-side routines in
  `exit.rs` and `tally.rs` charge measured cycle totals (`EXIT_FADE_CYCLES`, `UNPACK_CYCLES`,
  `SLIDE_STEP_CYCLES`) through `Cx::busy`.
- **The vertical-blank and copper interrupts, and the 25 Hz pass**
  ([frame-interrupts-input](frame-interrupts-input.md)). The scroller requests the copper interrupt
  (`custom(0x9c, 0x8010)`), which runs `copper_irq_blits`. The pass's blits mask it
  (`custom(0x9a, 0x0010)`) and a pending one is taken on unmasking. The beam machinery in
  `mod.rs` decides where a late frame's vertical blank interrupts the pass.
- **The copper run, the blitter and the hardware sprites** ([display-hardware](display-hardware.md)).
  `scroll_latch`, `panel_clear`, `sprite_digits`, `clock_at_start`, the fades and the exit fade
  write into the level copper list (`LEVEL_COPPER`, h2+$6bd0). `bob_draw`, the scroller,
  `redraw_cell` and the panel blit into the display buffers and the panel, with `objects::blit` as
  the level's blitter model (ascending area mode only). The frog's sprites are placed by writing
  their records (`FROG_SPRITE_LEFT`, `FROG_SPRITE_RIGHT`). `bob_setup` (`level/bobs.rs`, owned
  there) builds the bob records `bob_draw` uses.
- **Paula writes** ([sound-chip](sound-chip.md), [sound-driver](sound-driver.md)). `LevelCtx::sfx`
  and `sample` call the driver and stamp its writes with the beam position.
- **The CIA keyboard** ([frame-interrupts-input](frame-interrupts-input.md)). `Hw::sdr` and
  `key_in`; the pause reads `key_down` (h0+$16078).

## 4. What depends on it

- **[frog-and-items](frog-and-items.md).** `frame::vbl` and `load::frog_first_step` run the
  frog's physics and animation around `frog_tiles` and `camera_follow`, which read and write the
  frog record. Items register their kinds,
  markers and the `Dispatch` hooks (`bump_block`, `tile_pickup`, `bounce_pad`, `launcher`,
  `bubble`) and spawn through `objects.rs`.
- **[enemies-and-bosses](enemies-and-bosses.md), [slot-and-bonus](slot-and-bonus.md).** They
  register their kinds into `Dispatch` and work through `LevelCtx`, the spawners, `obj_free`,
  `overlapping`, `frog_hurt` and `frog_kill`. The witch uses `stepped_start`. The bonus level
  runs the shooter paths of the camera, scroller and spawner, and `tally.rs` hands over to the
  slot machine.
- **[frame-interrupts-input](frame-interrupts-input.md), [cpu-time-and-beam](cpu-time-and-beam.md).**
  `frame.rs` drives this chapter's routines and `LevelCtx` is the context it runs them in;
  `timeline.rs` replays `level_init`'s trace from `exit.rs`.
- **[game-flow](game-flow.md).** `Game` enters levels through `load::enter`, reads `LevelOutcome`
  and `load::current_of`, carries the `.sflevel` override, and runs `LevelOver`, `Restart`,
  `LevelStart` and `LevelTally` as front-end routines.
- **[front-end](front-end.md).** `front::chain` runs `LevelOver::chained` from `top_loop`'s own
  code, and the attract demo plays a level through `load::demo_load`.
- **[display-hardware](display-hardware.md).** The picture of a level is what this chapter left in
  the copper list, the display buffers, the clean buffer and the panel.
- **[verification](verification.md)** and **[app](app.md).** Every level conformance case and
  routine test runs this code. The app's level-file menu feeds `Game::with_override`.
- **The map editor** (the next plan, project `CLAUDE.md` "Next up"). It edits the `BODY` and
  `PALA` that section 1 says `level_load` reads, and plays them through this chapter unchanged.

## 5. What a native rewrite must preserve

Faithful stays the bar. Each row is behaviour the original shows that the Amiga-shaped parts here
carry, and the check that holds the engine to it today. Routine tests in
`engine/crates/sf-conform/tests/` run the original's routine on the vAmiga oracle and the port on
the same transplanted memory, and compare memory.

| Behaviour to keep | Check that proves it |
|---|---|
| Entering a level leaves memory as the original's entry does: the map unpacked from `BODY` (cells rotated to `tile << 6 \| marker`, 304 a row), the palette from `PALA`, the coins and items counted, the switch gates found, the start and camera from marker `$35`, the panel cleared, the first screen scanned and scrolled. | `every_level_is_entered_as_the_original_enters_it` and `a_game_started_at_a_level_plays_as_the_original` (`engine/crates/sf-conform/tests/level_entry.rs`). |
| A level from a file plays as the same level from the disks, and an edited cell changes only what it touches. | `an_unchanged_export_plays_as_the_disk_level_for_1500_frames`, `a_wall_cell_removed_lets_the_frog_through_and_nothing_else_differs_before` (`engine/crates/sf-assets/tests/sflevel.rs`); `w1_1_exported_from_the_menu_and_played_from_the_menu_has_the_disk_level_s_frames` (`engine/crates/sf-e2e/tests/levelfile.rs`). |
| Cell addresses through the boot's row table with the 68000's word arithmetic (`cell_at`). | `the_boot_s_tables_are_the_original_s` (`engine/crates/sf-conform/tests/frog_routines.rs`); the collision routine tests below. |
| Collision over the marker ladder: ground probe a fall ahead, landing four frames into the air, the `$3f` lift, the `$3d` step-up, the head probe, walls by the probe lists, slopes from `TILE_HEIGHTS`, gravity's tick. | `ground_probe_matches_the_original`, `head_probe_matches_the_original`, `slope_anim_matches_the_original`, `frog_tiles_matches_the_original` (`engine/crates/sf-conform/tests/collision_routines.rs`); `frog_wall_w1_1`, `frog_run_w1_1` (`engine/crates/sf-conform/tests/frog_cases.rs`). |
| Every marker's reaction, the switches and tile pickups included. | `frog_tiles_matches_the_original` (random handled markers); `switches_match_the_original`, `tile_pickups_match_the_original` (`engine/crates/sf-conform/tests/items_routines.rs`); `items_pad_w1_1`, `items_switch_w2_1`, `items_launcher_w2_2` (`engine/crates/sf-conform/tests/frog_cases.rs`). |
| Where the original loops for ever (both wall flags set) or divides by zero (the tally with fewer than five items), the engine's chosen way out (rust-engine [DESIGN §2.11](../../plans/rust-engine/DESIGN.md)). | `both_wall_flags_take_the_side_the_frog_faces_where_the_original_loops` (`engine/crates/sf-core/src/level/collision.rs`); `fewer_than_five_items_give_no_coin_pages_where_the_original_divides_by_zero` (`engine/crates/sf-core/src/level/tally.rs`). |
| The camera's lag of 8, its edges, `scroll_dirs`, the bonus level's creep. | `camera_follow_matches_the_original` (`engine/crates/sf-conform/tests/frog_routines.rs`); `bonus_enter_wb_1` (`engine/crates/sf-conform/tests/frog_cases.rs`). |
| `scroll_latch`'s copper writes: BPLCON1, the pointers into the shown buffer's ring row, the seam's wait and its second reload. | `scroll_latch_matches_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); every `render-*` case ([display-hardware](display-hardware.md)). |
| New rows and columns drawn on alternate frames into the clean buffer and copied to both display buffers from the copper interrupt, split at the ring's wrap; eight raster lines waited when nothing was drawn. | `scroller_matches_the_original`, `copper_irq_blits_match_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`). |
| Objects spawned from markers in the cells scrolling into view, by world; the whole screen from a start point; a cell with a live slot not spawned twice; eviction of an off-screen evictable slot when all 20 are full; the forced coin. | `map_spawn_scan_screen_matches_the_original`, `obj_spawn_cell_and_its_variants_match_the_original`, `obj_spawn_at_xy_and_nodedup_match_the_original`, `freeing_and_despawning_match_the_original` (`engine/crates/sf-conform/tests/items_routines.rs`). `scan_edges` has no routine test; the enemy activation cases cover it under check 1 (`enemies_w4_4_activation` in `engine/crates/sf-conform/tests/frog_cases.rs`). |
| Slot order for updates, hitboxes and the draw list; the spud then the frog against the hitbox list built this pass; stomp, passive touch and hurt. | `obj_run_all_and_collide_all_match_the_original` (`engine/crates/sf-conform/tests/items_routines.rs`); `bob_draw_list_and_hitboxes_match_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); `touch_handlers_match_the_original` (`engine/crates/sf-conform/tests/items_routines.rs`). |
| A late frame's vertical blank lands inside the pass where the original's did: mid-walk of the hitbox list, inside the despawn test, between bobs, inside `panel_update`, inside `sfx_play`'s raster wait. | `the_frog_walk_ends_its_frame_at_the_entry_the_original_had_got_to`, `a_frame_ends_inside_the_despawn_test_at_the_point_the_original_had_got_to`, `a_frame_ends_in_bob_draw_after_the_bobs_the_original_had_drawn` (`engine/crates/sf-conform/tests/items_routines.rs`); `sfx_wait_ends_three_lines_on_by_vposs_low_byte` (`engine/crates/sf-core/src/level/mod.rs`); the completion cases (`complete_w4_3`, `complete_w1_2` in `engine/crates/sf-conform/tests/frog_cases.rs`) under check 1. |
| `bob_draw`: erase from the clean buffer, cookie-cut with the mask, background bobs, clipping to the screen and to the seam's two parts, the animation stepped as drawn; then the buffer flip. | `bob_draw_matches_the_original`, `buffer_flip_matches_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); every `render-*` case on W2–W5. |
| Tile animations and the secret passage opening a column a pass, its edges mended, redrawn in both buffers. | `tile_anim_draw_matches_the_original`, `passage_step_matches_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); `the_fake_wall_opens_as_in_the_original_without_a_re_sync` (`engine/crates/sf-conform/tests/level_entry.rs`). |
| Hurt, death, the lives, time running out ending the game, the `level_state` countdown. | `frog_hurt_and_time_up_match_the_original` (`engine/crates/sf-conform/tests/items_routines.rs`); `frog_pit_w1_1`, `frog_timeup_w1_1`, `frog_esc_w1_1` (`engine/crates/sf-conform/tests/frog_cases.rs`). |
| `level_start` on the beam (the frog's first step, the fade in, the panel sliding in a step a frame on line `$40`, the drop-in); a restart after a death. | `level_start_matches_the_original`, `game_over_check_chooses_as_the_original` (`engine/crates/sf-conform/tests/restart_routines.rs`); `frog_pit_w1_1`'s restart. |
| The exit (marker 54 with no coins left), the exit fade closing the window a line a frame, going on to the next level. | `complete_w1_1_to_exit_fade`, `complete_w1_1`, `complete_w4_1` (`engine/crates/sf-conform/tests/frog_cases.rs`); `w1_1_goes_on_to_w1_2` (`engine/crates/sf-conform/tests/chain.rs`). |
| The tally: pages for time, tile coins and coins, the icons cleared, the texts printed, the choice, the points or the slot machine, the extra life. | `tally_w1_1`, `tally_slot_w1_1` (`engine/crates/sf-conform/tests/frog_cases.rs`); `the_pages_icons_are_cleared_as_the_original_clears_them`, `the_level_number_is_printed_as_the_original_prints_it`, `the_score_is_printed_as_the_original_prints_it`, `the_extra_life_check_after_the_points_matches_the_original` (`engine/crates/sf-conform/tests/tally_routines.rs`); the `pages_*` tests in `engine/crates/sf-core/src/level/tally.rs`. |
| The panel: one score digit and one clock digit a pass, the lives icon when it changes, the coins as sprite digits. | No routine test. `panel_bitmap` is a presentation piece (`presentation_piece` in `engine/crates/sf-core/src/layout.rs`), left out of check 1; the picture check on every stepped level case and the `render-*` cases compare the panel's rows. |
| The fades: fade-in, fade-out and stepped records stepped each vertical blank, components by their step. | `fades_match_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); `final_win_to_the_levels_end` (`engine/crates/sf-conform/tests/bosses.rs`) for the witch's flash stepping back. |
| P pauses the pass where it is, the vertical blank running its paused path, and P or fire resumes it. | `p_pauses_and_p_resumes_as_the_original_does`, `fire_ends_the_pause_as_the_original_does` (`engine/crates/sf-conform/tests/chain.rs`). |
| The joystick as Denise encodes it in `JOY1DAT`, fire on CIA-A PRA bit 7. | `joy1dat_encodes_directions_as_denise_does` (`engine/crates/sf-core/src/level/mod.rs`). |
| An object with no ported kind is reported, never silently skipped. | `a_template_with_no_kind_is_reported_unported` (`engine/crates/sf-conform/tests/frog_cases.rs`). |
| All of it together, every level in order. | `the_whole_game_plays_as_the_original_does` (`engine/crates/sf-conform/tests/chain.rs`, case `engine/conform/cases/game-whole.toml`). |

### The object slot record, kept as the original's bytes

A slot is `$7c` bytes at h0+$1ba6c + `$7c` × k ([docs/09 "The slot record"](../09-object-system.md#the-slot-record)).
Its first `$34` bytes are copied from its template in hunk 0 (`fill_slot`), `+$40` is the cell it
came from (non-zero is "live"), `+$44` its position, `+$4c`/`+$50` its bob's free-list entry and
record, and the routines are long words holding the original's code addresses (`+$00` init, `+$04`
update, `+$10` touch). The engine runs a routine by looking that address up in `Dispatch`
(`call`). Every object kind's port reads and writes its slot by offset.

What that costs a native rewrite:

- Replacing the record with a Rust struct means replacing every kind's offset accesses at once
  (items, enemies, bosses, the bonus level), because they share the slot, the hitbox list
  (`+$14` points at a template's hitbox table) and the bob record through raw offsets. There is no
  smaller cut: a struct for one kind still has to sit in the same 20 slots the spawners search.
- Routines named by original address are how a transplanted slot keeps running: conformance copies
  the original's slots into the engine mid-level and the engine finds the ported routine from the
  address it reads. A native dispatch (an enum of kinds and states) needs a mapping from those
  addresses, or the transplanting checks change ([memory-model](memory-model.md) and
  [verification](verification.md) state the options, DESIGN §2.5).
- Some state a kind holds is a pointer into hunk 0 (animation and path tables, `obj_path_xy`'s
  `+$54`), so the templates and their tables must move to native data in the same step.
- The quirks are in the bytes: a slot is live by its cell being non-zero, a parked bob sits at
  `$7d00`, `$7d00`, and immunity is `+$1c` = `$7d00`. A rewrite must keep each as a rule.

## 6. Native sketch (non-binding)

This sketch is non-binding: designing the rewrite is the later plan's job.

The game rules here (collision, the camera, spawning, slots, death, the exit, the tally's pages)
are already plain functions of the state. They would move almost line for line onto native types:
a `Map` of unpacked cells with a `cell(x, y)` that keeps the row table's behaviour above and below
the map, a `Frog`, a `Camera`, a `[Slot; 20]` with a kind enum and its state. The order of the
calls in `frame::vbl` and `frame::pass` is the game and stays.

The display side splits off. `scroll_latch`, the scroller's ring buffers, `bob_draw`, `redraw_cell`
and the panel's blits exist to leave in memory what the original's display reads. A native level
could hand the renderer a scene instead (the camera, the map, the tiles the passage and the
animations changed, the bobs with their frames and draw order), as
[display-hardware](display-hardware.md)'s sketch describes, with the overlay of changed tiles kept
as state with the original buffers' lifetimes. The tally and panel text become drawn text.

The hardest part is the beam machinery in `LevelCtx`. It exists so that a late frame's vertical
blank interrupts the pass at the same call, step and point as the original's, which decides what
the frog and camera were when an object read them. A native engine still needs those cut points
(each `step_done`, `point` and `step_at` is one) if it is to pass the stepped cases, but could
express them as "the pass yields here" in a structured pass rather than as a cycle clock, provided
the moment it yields is still decided by the original's timing, which today comes from the cost
model and the oracle's recorded frame ends. Whether a cheaper rule (yield only at the phase
boundaries) would still pass every stepped case is unknown; running the stepped level cases with
the inner cut points disabled would show how many fail.

## 7. Reuse

Rating: **`superfrog-only`**. 6 144 lines (`python tools/architecture.py totals`).

Everything here is Superfrog's: its marker numbering and the collision ladder over it, the 20-slot
object record, the bob drawer's ring-seam split, the panel's layout, the tally's rules and picture,
and the beam machinery's cut points, which are tied to the order of Superfrog's pass. Only the
method carries:

- A tile-map platformer core on another Amiga game could carry the approach: unpack the map once,
  read collision from per-cell flags plus per-tile height columns, spawn from markers in the cells
  scrolling into view, keep a fixed slot table, and port each routine against the original's memory
  so a routine test can compare them. The code itself would be rewritten for the other game's
  formats and rules (`big-changes` at best, and only if the game is built the same way).
- `LevelCtx`'s frame-end machinery is the same problem any faithful port of a game with a slower
  main loop than its interrupt meets. Its interface (`spend`, `step_done`, `point`, `frame_end`)
  could become a library, but its cuts and its matching to the oracle's recorded frame ends are
  per game.
- `objects::blit` duplicates the generic blitter of `engine/crates/sf-core/src/front/blit.rs`
  ([display-hardware](display-hardware.md)) and carries nothing that one does not.

Another Team 17 game of the period might share `T7MP` with its `BODY` and `PALA` tags, the
marker-in-high-bits cell format, the slot record layout or the bob drawer. This is unchecked.

## 8. Sources

- [docs/04](../04-t7mp-level-format.md) (`T7MP`, `BODY`, `PALA`),
  [docs/07](../07-secret-passages.md) (fake walls, the passages),
  [docs/08 "Main loop and interrupt", "Lag", "Scene flow"](../08-executable-map.md),
  [docs/09](../09-object-system.md) (placement, the slot record, the frog and the spud),
  [docs/12 "Scroll"](../12-display.md#scroll), [docs/14](../14-sflevel-format.md) (`.sflevel`).
- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1 (the checks), §2.10 (the level
  override), §2.11 (unhappy paths).
- Code: `engine/crates/sf-core/src/level/` (the owned files above),
  `engine/crates/sf-core/src/level/frame.rs` (`vbl`, `pass`), `engine/crates/sf-core/src/game.rs`
  (`Game::with_override`, `restore_display`), `engine/crates/sf-core/src/mem.rs` (`TILE_HEIGHTS`,
  `tile_heights`, `boot_tables`), `engine/crates/sf-core/src/render/mod.rs`
  (`DisplayData::levels`), `engine/crates/sf-assets/src/sflevel.rs`.
