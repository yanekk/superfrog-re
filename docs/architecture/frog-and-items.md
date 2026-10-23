+++
id = "frog-and-items"
title = "The frog and the items"
tier = "subsystem"
depth = "deep"
kind = "game"
mix = { amiga = 20, game = 70, data = 10, shell = 0 }
reuse = "superfrog-only"
owns = [
  "engine/crates/sf-core/src/frog/",
  "engine/crates/sf-core/src/items/",
]
+++

# The frog and the items

## 1. What it is

The player's frog (its record, its motion, its animation and the two hardware sprites that show
it) and everything in a level that is not an enemy: coins and the exit lock, pickups drawn in the
map's tiles, the power-ups they give, the spud the frog throws, the fixtures it sets off, and the
bubble. Every routine is a one-for-one port of the original's, under its name and address, reading
and writing the original's variables at their original addresses
([docs/09 "The frog, the spud, bobs and sprites"](../09-object-system.md#the-frog-the-spud-bobs-and-sprites)).

**The frog** (`engine/crates/sf-core/src/frog/`):

- `engine/crates/sf-core/src/frog/mod.rs`: the record `FROG` (h0+$7d6) and its field offsets
  (`frog::f`, about sixty named words: position, velocity, speed limits, the air, hurt, dying,
  blink, shield, flight, the slope it stands on, lives, coins left for the exit). `frog_new_game`
  (h0+$8f90) clears the `$200`-byte record and writes the lives and the game mode's tuning (bounce
  speed, gravity delay, air step delay, air speed; base speed 5 easy, 6 otherwise); `frog_reset`
  (h0+$902e) is the level's (re)start; `frog_life_reset` (h0+$9148) is what losing a life resets.
- `engine/crates/sf-core/src/frog/physics.rs`: `frog_move` (h0+$db58, position plus velocity,
  clamped at 0), `frog_walk_input` (h0+$113be: the speed-up's countdown, the blink, facing and
  climb, the walking speed limit stepped every fourth frame, the joystick's eight directions into
  `up`, `up_side`, `walk`, crouch or `no_walk`'s friction, and the bonus level's `shooter_input`),
  `frog_air_anim` (h0+$11056: fire in the air with wings, the shield's and the launcher's
  animations, the special animations, the croak on a pushing surface, and the bubble's spawn), and
  the attract demo's `demo_play` (h0+$11990) and `demo_record` (a development leftover kept as an
  assertion).
- `engine/crates/sf-core/src/frog/anim.rs`: the animation player (`frog_set_anim`,
  `frog_set_anim_free`, `frog_set_anim_at`, `frog_anim_tick`), the animation the motion calls for
  (`frog_input_anim`, with the hurt and dying animations and the landing check over
  `FALL_HISTORY`), the held jump (`frog_jump_float`), the two hardware sprites
  (`place_frog_sprites` h0+$19bbe, `sprite_to_copper` h0+$1b23e), the drop-in at a level's start
  (`frog_level_entry` h0+$6bd4), and `frog_exit_walk`, unported here because the end-of-world walk
  runs in the game flow ([game-flow](game-flow.md)).

Gravity, the ground, ceiling and wall probes, the bounce off a pad (`bounce`) and the markers under
the frog are `frog_tiles` in `engine/crates/sf-core/src/level/collision.rs`, owned by
[level](level.md); the camera that follows the frog is `camera_follow` in
`engine/crates/sf-core/src/level/camera.rs`; hurting and killing the frog is
`engine/crates/sf-core/src/level/death.rs`. The level's vertical blank, `vbl` in
`engine/crates/sf-core/src/level/frame.rs`, calls the frog's routines in the original's order:
`frog_level_entry`, `demo_record`, `demo_play`, `frog_tiles`, `frog_air_anim`, `frog_anim_tick`,
`frog_walk_input`, `frog_input_anim`, `place_frog_sprites`, the level clock, `frog_move`,
`camera_follow`.

**The items** (`engine/crates/sf-core/src/items/`), each the original's object template with its
init, update and touch routines, registered into the level's `Dispatch` by `items::register_all`:

- `engine/crates/sf-core/src/items/mod.rs`: coins (template 0, markers 8 and 9), the falling coin
  (template 2), the hidden coin spring (template 4, marker 45: five coins knocked out by the frog's
  head or the spud), the exit lock (template 5, marker 29, gone once `coins_left_for_exit` reaches
  0), and `take_coin` (score, sparkle, sound, `coins_collected`, the panel's count).
- `engine/crates/sf-core/src/items/pickups.rs`: pickups drawn in the map's tiles (marker 55).
  `tile_pickup` (h0+$ed8a) looks the touched tile up in the world's table at h0+$f382 and
  dispatches on the handler address it finds: checkpoint, tile coin, spud, wings, bottle (hearts
  and two minutes), speed-up, slow-down, extra life, shield, and eleven fruits by their points.
  `take_tile` queues the tile's next frame on the tile animation queue and clears its marker.
- `engine/crates/sf-core/src/items/powerups.rs`: the spud (`spud_update` h0+$80f0, in the 25 Hz
  pass): thrown in one of five ways, steered back to the frog three pixels a pass, splatting on a
  wall, parked off the map at x `$639c`. Wings, speed and the shield have no routines of their own;
  a pickup sets a frog field and the frog's routines act on it.
- `engine/crates/sf-core/src/items/fixtures.rs`: bounce pads (templates 6 and 7, markers 4 and 5),
  launchers (templates 9 and 10, marker 7), and switches (markers 48–51), whose `switch` (h0+$f6be)
  opens the gate in the map cell by cell, world by world, and spawns the pulled lever (templates
  42–51).
- `engine/crates/sf-core/src/items/bubble.rs`: template 86, the bubble that rises every 64 frames
  from a frog on W3-4's pushing surface (rust-engine T204).

**The arithmetic.** The frog's physics has no fixed point: positions are whole pixels and
velocities whole pixels a frame, held in 16-bit words. Sub-pixel motion is made by counters
instead: gravity adds 1 to `vy` every third frame on the ground or every `GRAVITY_DELAY + 1`th in
the air, friction acts every second frame (every sixth while hurt), the walking speed limit steps
every fourth frame, a held jump takes 1 off the rising speed every second frame. The port keeps the
68000's arithmetic exactly: every sum is `wrapping_add`/`wrapping_sub` on `u16`, and every
comparison is `n_clear` (`engine/crates/sf-core/src/level/mod.rs`), which tests the sign bit of the
16-bit result as `bpl` does after a `cmp` or `tst`, not a signed comparison. The two differ on
overflow, and the original branches on N alone. A rewrite that writes `vx < target` as a signed
compare changes the frog's motion on the rare frames a difference overflows, and one frame's
difference in the frog's position shifts the camera, the spawns and the RNG's calls after it; that
is why it must stay bit-exact.

## 2. Kind mix

`amiga` 20, `game` 70, `data` 10, `shell` 0.

- `amiga`: in `frog/anim.rs`, the two hardware sprites (`place_frog_sprites`, `sprite_to_copper`,
  `animate`, the `spr` record, the sprite moves' data pointers that `frog_anim_tick` writes into the
  copper list, the exit sprite pair) and the beam wait `LevelCtx::sprite_wait` both call; in
  `frog_level_entry`, the drop-in played by moving the sprites' screen y. The joystick read as
  `JOY1DAT` bits (`0x303` masks) and fire as CIA-A's port bit; map cells reached by pointer
  arithmetic on `level_map` (`cell - 0x260` is the row above).
- `game`: the rules: walking, running, friction, jumping and the held jump, the slope's effect,
  the wings, shield, speed-up and slow-down, hurt and dying and the blink after, lives and the game
  mode's tuning, coins, the exit lock, pickups and their points, checkpoints, the spud's throw and
  return, pads, launchers, switches and their gates, the bubble, the attract demo's playback.
- `data`: addresses that point into the executable's own tables, used as they are: the frog's
  animation lists (h0+$10492..$10f52) and the per-direction tables of them (h0+$10192..$101fe,
  h0+$11036, h0+$11046), the pickup tables at h0+$f382 whose handler addresses `tile_pickup`
  matches on, the bob animations each item starts (h0+$2ce04, h0+$2d0b0, h0+$318b8, ...), the
  levers' tile words per world in `switch`, and the attract demo's recording read through
  `DEMO_AT`.
- `shell`: none. The routines' checks live in `sf-conform` ([verification](verification.md)).

Judged by reading every function in the eight files and sorting its lines by the parts above:
`anim.rs` is about 40 % `amiga`, `fixtures.rs` and `pickups.rs` about 20 % `data` (the lever
table, the handler addresses), the rest mostly `game`. Field access at original addresses is not
counted as `amiga` here; it is the memory model's mechanism, and the rule written through it is
still the game's.

## 3. Mechanisms it uses

- **Byte-exact state at original addresses** ([memory-model](memory-model.md)). The frog is the
  `FROG` record at h0+$7d6, read and written through `Bus` by offset; `layout::LAYOUT` names each
  field (`frog_x`, `frog_vx`, `frog_gravity_tick`, ...) so conformance compares them. Items keep
  their state in the original's object slots (`+$5c`, `+$60`, `+$68` per kind) and in the map's
  cells, whose marker bits `touch_coin` and `take_tile` clear so a pickup is taken once. The spud
  lives in the frog's record (`+$72`..`+$7c`, `+$ae`).
- **The loaded hunk image** ([copied-in-data](copied-in-data.md)). Animations are pointers into
  hunk 0: a list of 8-byte steps (a frame pointer, negative to loop, and frames to show it at
  `+6`), each frame giving its sprite data pointer, the sprite offset at `+4` and the height at
  `+8`; the sprite data itself is in hunk 3 (`hero_sprite_frames`, `layout.rs`). `frog_anim_tick` and `animate` walk them as the original
  does; `LEFT_ANIMS` (h0+$10b7e, in `level/collision.rs`) tells which way the frog faces from where
  its animation sits. `tile_pickup` reads the world's table at h0+$f382 and matches the handler
  address; an address it does not know is reported through `LevelCtx::unported`.
- **Object templates and dispatch** ([level](level.md)). Each item is an `ObjectKind`, its
  routines keyed by the original's code addresses, registered by template index and by map marker
  per world (`Dispatch::register_marker`), so the level's object loop calls the port where the
  original would jump. `tile_pickup`, `bounce_pad`, `launcher`, `bump_block` and `bubble` are
  `Dispatch` hooks the level's collision calls.
- **Hardware sprites** ([display-hardware](display-hardware.md)). The frog is two attached
  hardware sprites; `sprite_to_copper` turns a record's x and y into `SPRxPOS`/`SPRxCTL` words in
  the copper list (x + `$80` halved, the low bit and the ninth bits of start and stop in the
  control word) and points the attached partner one sprite on. `level/bobs.rs` calls it for two
  more sprite records (h0+$1b128, h0+$1b144) too.
- **Cycle charging and the beam** ([cpu-time-and-beam](cpu-time-and-beam.md)). `vbl` charges
  each frog routine its cost (`Cost::FrogEntry`, `FrogAir`, `FrogAnim`, `FrogAnimTail`, `FrogWalk`,
  `FrogInput`, `FrogSprites`, `FrogMove`), and `frog_anim_tick` and `place_frog_sprites` call
  `sprite_wait`, the original's wait for the beam to leave lines `$1d`..`$21` before rewriting the
  sprite moves. A write that still lands after the copper has passed them shows on the next frame:
  a launcher's throw reaches `frog_anim_tick` on line `$1e` or `$1f`, and the frame shows the pose
  before (rust-engine T86, T127).
- **The vertical-blank interrupt and the 25 Hz pass** ([frame-interrupts-input](frame-interrupts-input.md)).
  The frog runs at 50 Hz in the vertical blank; items' updates and `spud_update` run in the main
  loop's 25 Hz pass. Counters in passes (`+$5c` sparkle `$10` passes, pads six passes) and in
  frames (the shield `$190` frames) are therefore different units.
- **Paula writes** through the sound driver ([sound-driver](sound-driver.md)): `LevelCtx::sfx` and
  `LevelCtx::sample`. The croak and the pad and spud sounds pick from the RNG (`rng_range`), so
  they are part of the RNG's call order ([game-flow](game-flow.md) §3).

## 4. What depends on it

- [level](level.md): `frog_tiles` (gravity, `bounce`, the pickup and fixture hooks), the camera,
  deaths, `load.rs`'s `frog_new_game`, `frog_reset` and `frog_first_step`, `vbl`'s order, and the
  object loop that calls every item routine.
- [slot-and-bonus](slot-and-bonus.md): the bonus level reuses the frog (`shooter_input`, the
  shooter branches of `frog_air_anim`, `hurt_anim` and `frog_level_entry`, `bonus/frame.rs` calling
  `frog_reset`), and `spud_update` hands over to `bonus::shots::shooter_spud` in shooter mode.
- [enemies-and-bosses](enemies-and-bosses.md): enemies read the frog's record (position, hurt,
  shield) and hurt it through `level/death.rs`, which sets the frog's animations here.
- [game-flow](game-flow.md): the end-of-world walk in `front/chain.rs` uses `frog_set_anim`,
  `frog_anim_tick` and the exit sprite pair (`EXIT_SPRITE_LEFT`, `EXIT_SPRITE_RIGHT`).
- [front-end](front-end.md): the attract demo runs a level with `demo_play`'s recorded joystick;
  the options' lives and game mode reach the frog through `frog_new_game`.
- The app's level files, the route planners in `engine/crates/sf-conform/examples/` and the bug
  recorder's `Game::location` read the frog's position and lives from the record.
- [verification](verification.md): every check in §5 compares this code's writes with the
  original's.

## 5. What a native rewrite must preserve

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| The frog's routines, as 68000 arithmetic (`u16` wrapping, `n_clear`) | Every routine's writes bit for bit over any frog state, joystick and fire: walking, friction, speed steps, the jump, the held jump, the air, the shield and launcher animations, the croak | `frog_walk_input_matches_the_original`, `frog_air_anim_matches_the_original`, `frog_air_anim_on_pushing_surfaces_matches_the_original`, `frog_jump_float_matches_the_original`, `frog_move_matches_the_original`, `frog_input_anim_matches_the_original`, `frog_new_game_matches_the_original`, `frog_reset_matches_the_original` in `engine/crates/sf-conform/tests/frog_routines.rs` (10 000 generated states each, on Moira) |
| The order of the frog's routines in the vertical blank | `frog_level_entry`, demo, `frog_tiles`, `frog_air_anim`, `frog_anim_tick`, `frog_walk_input`, `frog_input_anim`, `place_frog_sprites`, clock, `frog_move`, camera, each frame | check 1 on every level case; `frog_idle_w1_1`, `frog_run_w1_1`, `frog_wall_w1_1`, `frog_pit_w1_1` in `engine/crates/sf-conform/tests/frog_cases.rs`, and the `frog-w2-1`..`frog-w6-1` cases (`sf-conform all`) |
| Animation lists read from hunk 0 | The same step, frame, sprite offset and height on each frame, the list looping at its negative entry | `frog_anim_tick_matches_the_original` (`ANIMS`, the 31 lists the routines start); picture comparison (check 2) on the frog cases |
| Two attached hardware sprites through the copper list | `SPRxPOS`/`SPRxCTL` words and data pointers as the original writes them, hidden off screen at x `$150`, below the field, blinking, or under a spring | `place_frog_sprites_matches_the_original`; check 2 on every level case |
| The sprite wait and where in the frame the sprite writes land | Writes held until the beam leaves lines `$1d`..`$21`; a launcher's throw on line `$1e`/`$1f` shows the pose before | `the_frog_sprites_wait_out_lines_1d_to_21` in `frog_routines.rs`; `items_launcher_w2_2` in `frog_cases.rs`. The bounce pad's `FrogSprites` charge is tested by nothing (rust-engine FINDINGS, T86 review) |
| The drop-in at a level's start | The sprites fall from above, faster each frame up to 8, to `$70` (`$c0` in the final level); the bonus level's fly-in from the left | `frog_level_entry_matches_the_original`; `every_level_is_entered_as_the_original_enters_it` in `engine/crates/sf-conform/tests/level_entry.rs`; `bonus_enter_wb_1` |
| Hurt, dying and the landing check (`FALL_HISTORY`) | The animation to its end mark `$4e20`, the blink `$4b` (`$96` easy), the thud once the fall has stopped for eight frames | `frog_input_anim_matches_the_original`; `frog_hurt_and_time_up_match_the_original` in `engine/crates/sf-conform/tests/items_routines.rs`; `frog_pit_w1_1`, `bonus_death_wb_1` |
| Item routines keyed by the original's code addresses | Each template's init, update and touch writing what the original writes, slots and map cells included | `object_updates_match_the_original`, `touch_handlers_match_the_original`, `bubble_update_matches_the_original`, `obj_run_all_and_collide_all_match_the_original` in `items_routines.rs`; `the_items_register_their_markers_in_every_world` |
| Pickup tables at h0+$f382 and their handler addresses | Each world's tile to its effect, points, sound and the taken tile queued | `tile_pickups_match_the_original`; `items_fruit_w2_1`, `items_fruit_w2_2`, `items_fruit_w4_1`, `items_life_w2_1`, `items_wings_w1_2`, `items_checkpoint_w1_1`, `items_shield_w3_1` in `frog_cases.rs` |
| Switches writing the gate into the map | The same cells and tile words per world, the lever's template and cells | `switches_match_the_original`; `items_switch_w2_1` |
| The spud | The throw's five ways, the return, the splat and the catch box, its bob's records | `spud_update_matches_the_original` in `items_routines.rs`; `items_run_w1_1` |
| Pads, launchers and the bubble in play | The throw a pass after landing, the spring, the bubble every 64 frames from W3-4's surface | `items_pad_w1_1`, `items_launcher_w2_2`, `items_bubble_w3_4` (first bubble at frame 212 from cell 285,63) in `frog_cases.rs` |
| The attract demo's recording | The recorded joystick and fire replacing the player's until fire or the recording's end | check 1 on `attract-demo-3` and `front-attract` (`sf-conform all`, [front-end](front-end.md)); no routine test names `demo_play` |
| The RNG calls the frog and items make (croak, pad and launcher twang, spud's throw sound) | Called on the same frames, with the same ranges | check 1 compares `rng_state` on every case; `frog_air_anim_matches_the_original` seeds `rng_state` among its varied fields |

The routine tests and the frog cases run in `sf-conform`'s test block, not `sf-core`'s, because
`engine/crates/sf-conform/tests/purity.rs` keeps the emulator out of the pure crates' tests.

## 6. Native sketch (non-binding)

Designing the rewrite is a later plan's job; this is a shape it could start from, not a decision.

- **A `Frog` struct.** Named fields in place of `FROG + f::...` reads, with a mapping to the
  original's offsets kept for conformance (the choice [memory-model](memory-model.md) §5 sets out:
  a mapping layer, a probe adapter, or changed checks). The arithmetic stays `u16` wrapping with an
  N-flag test; a helper that names it (`n_clear` already does) is enough. Converting to `i16` with
  signed compares is the one simplification that is not safe.
- **Animations as decoded data.** The frog's 31 lists and their frames decoded once from the
  executable at import into a table keyed by name (stand, run, jump-up, hurt-left, ...) in place of
  h0 addresses; `LEFT_ANIMS`'s address comparison becomes a facing field. The frog's `ANIM` field
  would then hold an index, so conformance needs the index-to-address map to compare it.
- **Sprites as a draw call.** The frog would say "show frame F at x, y, hidden or not"; the copper
  words, attached-pair pointers and the sprite wait move to the display layer
  ([display-hardware](display-hardware.md) §6). What must survive is the frame on which a pose
  shows, including the frame-late pose of a launcher's throw.
- **Items as kinds, not code addresses.** `ObjectKind`'s routine table keyed by code address
  becomes an enum per item; `tile_pickup`'s match on handler addresses becomes the decoded pickup
  table mapped to an enum. The lever's per-world tile words become decoded data beside the map.
- **Unchanged:** the rules themselves, the routines' order in the frame, the 50 Hz/25 Hz split and
  the RNG's calls.

## 7. Reuse

**`superfrog-only`**, 2411 lines (`python tools/architecture.py totals`). The frog's record, its
movement rules, Superfrog's pickups, the spud, the switches and every address are this game's.
Another Amiga game keeps only the method: port each routine one for one under the original's name
and address, keep 68000 arithmetic (`n_clear`, wrapping sums), and prove each port against the
original on generated states on the routine rig (`engine/crates/sf-conform/src/routine.rs`,
[verification](verification.md)).

The nearest thing to a generic part is `sprite_to_copper` (about 40 lines): turning a sprite's
screen x and y into `SPRxPOS`/`SPRxCTL` words is the same on any OCS game (`small-changes`: the
display window offsets `$80` and `$2c` and the record layout). The animation list format (8-byte
steps, negative to loop) is the original's own and not general.

Team 17 note (unchecked): Team 17's platformers of the period may share the object template and
sprite-record layouts; nobody has looked.

## 8. Sources

- [docs/09 "The frog, the spud, bobs and sprites"](../09-object-system.md#the-frog-the-spud-bobs-and-sprites),
  ["Types per world"](../09-object-system.md#types-per-world),
  ["The slot record"](../09-object-system.md#the-slot-record)
- [docs/08 "Main loop and interrupt"](../08-executable-map.md#main-loop-and-interrupt) and
  ["Input"](../08-executable-map.md#input)
- [docs/07](../07-secret-passages.md) for the map's markers
- [plans/rust-engine/DESIGN.md](../../plans/rust-engine/DESIGN.md) §2.1 (the four checks)
- `plans/rust-engine/FINDINGS.md` (T86 and T127 on the sprite wait; T122 on tile coins and jump
  heights; T204 on the bubble)
- Code: `engine/crates/sf-core/src/frog/`, `engine/crates/sf-core/src/items/`,
  `engine/crates/sf-core/src/level/frame.rs` (`vbl`), `engine/crates/sf-core/src/level/collision.rs`
  (`frog_tiles`, `bounce`), `engine/crates/sf-core/src/level/load.rs` (`frog_first_step`),
  `engine/crates/sf-conform/tests/frog_routines.rs`, `engine/crates/sf-conform/tests/items_routines.rs`,
  `engine/crates/sf-conform/tests/frog_cases.rs`
