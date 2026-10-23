+++
id = "enemies-and-bosses"
title = "Enemies and bosses"
tier = "subsystem"
depth = "deep"
kind = "game"
mix = { amiga = 30, game = 55, data = 15, shell = 0 }
reuse = "superfrog-only"
owns = [
  "engine/crates/sf-core/src/enemies/",
  "engine/crates/sf-core/src/bosses/",
]
+++

# Enemies and bosses

## 1. What it is

This chapter covers the object types that worlds 1–6 spawn from their maps, apart from the items:
the enemies, the hazards, the scenery that only animates or writes solid cells into the map, and
the one boss, the final level's witch. The original's side is in
[docs/09 "Types per world"](../09-object-system.md): a level holds no object list, each map cell's
6-bit marker selects a stub in `obj_flag_jumptab` (h0+$178ec, one row of 64 per world), and the
stub spawns a template into one of 20 slots. This chapter describes the engine's port of those
stubs and of every type's routines.

The slot machinery itself is not here. That covers `Dispatch`, `ObjectKind`, `SlotFn`, `register`,
`register_routines` and `call` in `engine/crates/sf-core/src/level/mod.rs`, and `obj_spawn_cell`,
`obj_run_all`, `bob_anim`, `bob_follow`, `obj_path_xy`, `obj_free` and `obj_despawn_if_offscreen`
in `engine/crates/sf-core/src/level/objects.rs`. It belongs to [level](level.md). The bonus level's
enemies (`engine/crates/sf-core/src/bonus/`) are [slot-and-bonus](slot-and-bonus.md)'s, and the
coins, springs and locks every world shares are [frog-and-items](frog-and-items.md)'s.

Every type is a port of the original's routines over the original's memory. A slot is the
original's `$7c`-byte record in `obj_slots` (h0+$1ba6c). It names its init, update and touch
routines by their 68000 addresses (`+$00`, `+$04`, `+$10`). The engine finds the Rust function for
an address in the dispatch, so a slot transplanted from the original runs on in the engine as it
would there. There are no tests in the owned files. The checks live in `sf-conform` (§5), because
the purity test keeps the emulator out of `sf-core`'s tests.

Files owned (5 849 lines):

- `engine/crates/sf-core/src/enemies/mod.rs`: `register_all`, which registers `common::ROUTINES`
  and then each world's `register_all`, in world order.
- `engine/crates/sf-core/src/enemies/common.rs`: what most types share. The touch handlers are
  `touch_kill_1hit` (h0+$23582), `touch_spud_damage` (h0+$2360e) and `touch_hurt` (h0+$236ac).
  `kill_fall`, `stomped` and `touch_stomped` are what a killed or stomped enemy turns into. The map
  tests are `marker_at`, `marker_right_of`, `solid`, `map_cell`, `cell_at` and `row_base`, and
  `walk_and_turn` is the walkers' step. `follow_and_despawn` is the end of most updates.
  `direction` (h0+$da62) is the 16-way direction the watchers and cannon face by, and
  `obj_path_y` is a y-only path. `partner_right_free` and `partner_left_free` are the paired
  spawn's partner search (`find_marker_right`, h0+$1b350, and `find_marker_left`).
- `engine/crates/sf-core/src/enemies/w1/` … `w6/`: one `mod.rs` per world, with its row index
  (`W1` = 0 … `W6` = 5), its `register_all` and its marker stubs, and one file per type or family
  of types. Each world's `mod.rs` lists the templates its markers spawn, walked from its row of
  `obj_flag_jumptab`.
- `engine/crates/sf-core/src/bosses/mod.rs`: `register_all` for the witch (template 134) and her
  marker (row 5, marker 1). `engine/crates/sf-core/src/bosses/witch.rs`: `WITCH`, `spawn_134`,
  her init, wait, rise, bob, loop and beaten updates, and `touch_boss_hit` (h0+$236b4).

### The common pattern

Every type follows one shape:

1. **A marker stub spawns it.** Each world's `register_all` calls `Dispatch::register_marker(row,
   marker, stub)`. The stub is almost always `obj_spawn_cell(c, template(t), cell, dx, dy)`: the
   template's `$ae`-byte block from `obj_templates`, a pixel offset into the cell, and the cell's
   address as the slot's identity (`+$40`), so the same cell never spawns twice. The exceptions
   are below.
2. **Its routines are registered by address.** A `pub static` `ObjectKind` lists
   `(original address, fn)` pairs for its init, updates and touch handler, and `register(d,
   template, &KIND)` adds them. A state change is the original's: the update writes a new address
   into `+$04` or `+$10` (`b.set_l(slot + 4, KILL_FALL)`), and the next pass calls whatever is
   there.
3. **Its state is the original's bytes.** Positions, speeds, timers and scratch are words and
   longs at the slot's offsets (`+$08` x, `+$0c` vx, `+$5c` timer, `+$64`…`+$72` per type). They
   are read and written through `Bus` with the 68000's wrapping arithmetic. The branch conditions
   are the 68000's flags (`n_clear` is "N clear after this subtraction"). A few types also keep
   state inside the code, as the original does (§2).
4. **Its look comes from the original's animation tables.** `bob_anim(b, slot, h0(…))` points the
   slot's bob at an animation record in hunk 0 (`w1::walker`'s `LEFT` is h0+$31080). Some
   routines read the bob's step back to decide something: the walkers step only on certain frames
   (`w3::shuffler`, `w4::walker`), the fountain throws at h0+$2e36c, the splat ends at `ANIM_END`
   (h0+$2af0a), and `w4::spikes` hurts only while the bob's `+$1a` counter is below 1. Paths are
   the original's tables too: `w1::hover::PATH` (h0+$24454), `w3::swing::PATH` (h0+$26398), and
   the witch's `WITCH_BOB_PATH` and `WITCH_LOOP_PATH`.
5. **Its cycles are charged in one place.** No owned file calls `spend`. `level::call` charges
   `beam::routine_cost(addr)` before each routine: a flat `Cost::Routine` (500 cycles), and the
   measured `Cost::HoverUpdate` (946) and `Cost::WatcherUpdate` (1070) for W1's hover and watcher,
   about twice any other enemy's. The cost exists so the pass ends on the original's beam line
   ([cpu-time-and-beam](cpu-time-and-beam.md)).
6. **It ends by following and despawning.** Most updates end with `follow_and_despawn`: the bob
   takes the slot's position, and the slot is freed once the camera has left it.
7. **Death is shared.** The touch handler takes `$20` hit points from `+$1c` (a template's
   `$7d00` makes it unkillable). At none, `touch_kill_1hit` makes it fall and splat for 200
   points (`kill_fall`), and `touch_spud_damage` flattens it into its death animation
   (`stomped`), then a reward (`touch_stomped` → `bonus::death::become_reward`). Both clear the
   cell's marker so it does not respawn, and both play an effect, `touch_kill_1hit`'s chosen by
   the RNG.

The exceptions:

- **Paired spawns.** The ends of a pacer's run are two markers on one row, and only one spawns:
  each end's stub looks along the row (up to `$98` cells) for the other and spawns only while that
  one has no live slot. W1's patrollers do this with their own `other_end_free` in
  `enemies/w1/mod.rs`. W2's bouncer, W4's pacer and crawler, and W5's pacers and shooter use
  `common::partner_right_free` and `partner_left_free`.
- **Spawns elsewhere than the marker.** Some types spawn into a neighbouring cell: W3's pad
  (`spawn_87`, up and left, and only from a pad's right cell), `spawn_89` and `spawn_90` (the cell
  below), and the witch (`spawn_134`, 8 rows up).
- **Multi-slot types.** A swinging mace's stub spawns the ball and then five links in the cells
  below it, numbering them through `chain_link_count` (h0+$262ba, a long inside the code;
  `w3::swing::spawn`, `w2::mace::spawn`, `w4::mace::spawn_69`, `w6::mace::spawn_67`). The links
  place themselves in sixths between the anchor and the ball.
- **Children at pixel positions.** Shooters and throwers spawn their shots at a pixel position
  rather than a cell (`w1::spout`, `w1::dropper`, `w4::fountain`, `w5::shooter`, `w6::cannon`,
  `w6::flier`, `w6::thrower`).
- **Map writers.** Ledges and gates write solid markers (`$3e`) or 0 into the map cells below
  them as their animation runs, so the frog's collision sees them (`w2::crumbling_ledge`,
  `w3::crumble`, `w4::ledge`, `w4::gate`).
- **State inside the code.** The original keeps some counters in words between its routines, so
  the engine does too, at those addresses: `w1_hover_phase` (h0+$243b4, also stepped by W4's
  swooper), `w1_spout_turn`, `chain_link_count`, and W6's `thrower_vx`, `flier_facing_right` and
  `cannon_aim`. Each is a `LAYOUT` variable, so conformance compares it. `w1::crawler` is
  stompable only while h0+$2d128, a word inside the bobs' animation records, is 2 or more.
- **A write through a null pointer.** When `w3::swing::spawn` finds no slot for a link, it writes
  the ball's address to `0 + $64`, the level 1 interrupt vector, as the original does (`layout.rs`
  `Level1Vector`). `w2::mace::spawn` does the same.
- **The witch** is a type like the others with more states: wait (`$32` passes), rise, bob on a
  y path for 100–199 passes (the RNG's), loop round the arena on an (x, y) path, and beaten. Both
  paths are relative to the camera. `touch_boss_hit` ignores `d7`, so each pass the spud overlaps
  her takes one of her 128 hit points. At none it starts fade record 0 from white
  (`level::fade::stepped_start`) and sets `level_state` to `$64`.

### Types by pattern

One line per type, grouped by what it does. The template is the original's index; the marker is
its cell's marker in its world's row. Unless named, a type's file is `enemies/w{n}/{file}.rs`.

**Walkers**: walk their row and turn at an invisible marker `$25` (`walk_and_turn` or a
variant that looks from x + 3 or the row above).

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W1 | 13 (3) | `w1::walker` | Walks and turns; a stomp flattens it into a reward. |
| W1 | 12 (2) | `w1::sitter` | Shares the walker's init but stands still; stomped into a reward. |
| W1 | 22 (12) | `w1::crawler` | Walks; stompable only while h0+$2d128 is 2 or more. |
| W1 | 21 (15) | `w1::spiker` | Walks; hurts on any touch, not stompable. |
| W2 | 39 (12) | `w2::walker` | Steps on one frame of its walk, turning at `$25` in the row above; stunned, then stomped into a reward. |
| W2 | 40 (11) | `w2::hopper` | Sits and now and then (RNG) jumps left or right; stunned, then stomped into a reward. |
| W3 | 91 (26) | `w3::walkers::STOMPABLE` | Walks; stomped into a reward. |
| W3 | 96 (25), 97 (13) | `w3::walkers::SPIKER`, `SPIKER_STAYING` | Walk; hurt on any touch. 97 is never despawned. |
| W3 | 92 (20) | `w3::shuffler` | Walks only on animation steps whose word `+6` is 0; stomped into a reward. |
| W3 | 94 (21) | `w3::hopper` | Walks bobbing on a short path and leaps now and then (RNG); cannot be killed. |
| W3 | 95 (28) | `w3::jumper` | Walks, leaps to a ceiling, circles on the hover's path, drops; stompable walking, one hit kills it in the air. |
| W4 | 93 (12) | `w4::walker` | Steps on its stepping frames; stunned by the spud, stompable. |
| W6 | 104 (15), 106 (20) | `w6::walker` | Step on their stepping frames; stomps bounce off. |
| W6 | 102 (13) | `w6::charger` | Paces, and after 8 passes left may (RNG) attack: a frog level with it within `$80` is hurt. |
| W6 | 105 (16) | `w6::thrower` | Walks like 104, stops and throws a falling shot (108). |

**Paired pacers**: go to and fro between two markers on their row; one spawns per pair.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W1 | 78 (16), 77 (17) | `w1::patroller` | 4 pixels a pass; hurts, unkillable. |
| W2 | 75 (2), 74 (3) | `w2::bouncer` | Bounces along on a hop table; immune, hurts. |
| W4 | 137 (2), 136 (3) | `w4::pacer` | Bobs along a wave; one hit kills it, the spud cannot. |
| W4 | 139 (13), 138 (14) | `w4::crawler` | Crawls; out of the spud's reach, hurts. |
| W5 | 80 (2), 79 (3); 82 (13), 81 (14) | `w5::pacer` | 8 or 3 pixels a pass, as W1's patroller; unkillable. |
| W5 | 84 (20), 83 (21) | `w5::shooter` | 3 pixels a pass; stops after turning left and fires a shot (85). |

**Runners and floaters**: move one way until the map stops them.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W1 | 19 (14), 20 (13) | `w1::runner` | Runs right or left at 4 a pass until a solid cell or the map's left edge, then is freed. |
| W4 | 143 (22), 144 (24) | `w4::floater` | Drifts sideways and sinks, pulled up every fourth pass; at marker `$17` its drift dies away. |

**Path fliers**: follow a path table from hunk 0 around their spawn point.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W1 | 8 (1) | `w1::hover` | Loops on one of four phases (`w1_hover_phase`), drifting its centre up to 42 pixels after the frog; one hit kills it. |
| W1 | 23 (11) | `w1::dropper` | Flies left on a bobbing path, drops a drop (17) every `$1e` passes, climbs away. |
| W2 | 38 (1) | `w2::flyer` | Loops on one of four loops in turn; one hit or stomp kills it. |
| W2 | 37 (21) | `w2::swooper` | Flies left and right on a wave, pausing now and then (RNG); unkillable. |
| W4 | 135 (1) | `w4::swooper` | Loops, stepping the hover's phase counter; one hit kills it. |
| W6 | 103 (14) | `w6::flier` | Loops, faces the frog, drops a shot (107) every `$40` passes; one hit kills it. |

**Wait-and-strike hazards**: a timer cycle at a fixed spot, harmless part of the time.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W2, W3, W4, W6 | 53, 54 (18, 19); 55, 56 (18, 19); 59–62 (18–21); 57, 58 (18, 19) | `w2::dart`, `w3::dart`, `w4::dart`, `w6::dart` | Waits hidden, flies from its spawn point to a solid cell, waits again. Only the inits differ; the wait and flight are h0+$25fa8 and h0+$25f42. |
| W3 | 99 (23) | `w3::drip` | Falls to a cell marked `$3d` or above, waits 8–12 passes (RNG). |
| W5 | 31 (11) | `w5::drip` | Falls after four animation steps, splashes 8 passes; harmless. |
| W4 | 141 (15), 142 (16) | `w4::crusher` | Waits `$15` passes, strikes `$30` pixels down or up, creeps back. |
| W5 | 140 (16) | `w5::crusher` | W4's crusher with a `$40` strike. |
| W5 | 28 (10), 30 (12) | `w5::faller` | Hangs until the frog is within `$40` across (30: or in view), shakes, falls and lies harmless. |
| W3 | 88 (16) | `w3::bouncer` | Falls and bounces off the ground three cells under it, forever. |
| W2 | 71 (16), 72 (20) | `w2::spikes` | Rise and fall in a cycle, in step with a live neighbour, so a row moves as a wave. |
| W4 | 145 (25) | `w4::spikes` | Shoot out and back; hurt only while the bob's counter is below 1. |

**Swinging maces**: a ball and five links in their own slots.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W2 | 63, 64 (17) | `w2::mace` | Inits only; updates are `w3::swing`'s. Also W5's marker 17. |
| W3 | 65, 66 (27) | `w3::swing` | The ball on `PATH`; links in sixths; ball freed with fewer than five links. |
| W4 | 69, 70 (17) | `w4::mace` | Inits, plus its own copy of the updates (see §5's note). |
| W6 | 67, 68 (17) | `w6::mace` | Inits only; updates are `w3::swing`'s. |

**Shooters and their shots**: a parent that spawns children at pixel positions.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W1 | 33 (10); 14, 15, 16 | `w1::spout` | Background scenery throwing a shot every `$32` passes, up-left, up, up-right in turn (`w1_spout_turn`). |
| W1 | 17 | `w1::dropper::DROP` | The dropper's drop: falls to a solid cell and splats, harmless. |
| W4 | 151 (26); 152–159 | `w4::fountain` | Throws four drops at an animation step, straight and slanting ways in turn. |
| W5 | 85 | `w5::shooter::SHOT` | Flies left and falls to a solid cell. |
| W6 | 101 (12); 109 | `w6::cannon` | Turns to the frog in five steps, fires every `$30`–`$50` passes (RNG); cannot be touched. |
| W6 | 107, 108 | `w6::shot` | The flier's and thrower's shots: straight, or falling in an arc. |

**Watchers**: scenery that faces the frog by `common::direction`.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W1 | 76 (18) | `w1::watcher` | One of eight animations by the direction to the frog. |
| W5 | 29 (22) | `w5::watcher` | As W1's, and blinks every 50–70 passes (RNG). |

**Map writers**: platforms and gates that put solid markers into the map.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W2 | 149 (36) | `w2::crumbling_ledge` | Two solid cells below it; crumbles when landed on, open at its broken frame. |
| W3 | 150 (36) | `w3::crumble` | Holds the frog while its animation runs, gives way at its last step; sample 4. |
| W4 | 148 (27) | `w4::ledge` | Solid except while its last frame shows; landing resets it. |
| W4 | 146 (32), 147 (33) | `w4::gate` | Each frame's three markers from a table into the row below; 147 mirrored. |

**Animated scenery**: passive, no hitbox.

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| W2 | 52 (10); 24, 25, 26 (13–15) | `w2::backdrop`, `w2::scenery` | Drawn into the background, or animates. Marker 34's stub is an `rts`. |
| W3 | 87 (5); 34 (17); 89 (14), 90 (15); 98 (24) | `w3::scenery` | The bounce pad's picture, three sceneries, and a flow in step with a neighbour. 98 is also W5's marker 24. |
| W4 | 32 (10), 27 (11) | `w4::scenery` | Animate where placed. 32 is also W5's marker 15. |

**The boss.**

| World | Template (marker) | Code | What it does |
|---|---|---|---|
| Final level (row 5) | 134 (1) | `bosses::witch` | Rises, bobs and loops in turn relative to the camera, hurts on touch, 128 hit points; at 0 a white flash and `level_state` `$64`. |

## 2. Kind mix

`amiga` 30, `game` 55, `data` 15, `shell` 0. Judged by reading every world's `mod.rs`, `common.rs`,
the witch and about half the type files, and splitting their lines by the parts below. It is an
estimate, not a count.

- **`game` (55).** What each type does: when it moves, how far, when it turns, when it hurts,
  what kills it, what it becomes, its timers and the RNG draws that pick its pauses and effects.
  This would exist in any reimplementation of Superfrog. Here it is written in the original's
  shape: offsets into a slot record and the 68000's flags.
- **`amiga` (30).** The byte-exact memory model these rules are written in. That covers routine
  identity by 68000 address (every `ObjectKind`'s table, every `set_l(slot + 4, ADDR)` state
  change, the registration order that decides which copy of a shared routine runs), `n_clear` and
  wrapping arithmetic standing in for condition codes, and `row_base`'s `ext.l` sign. It also
  covers counters kept inside the code, the crawler's read of a word inside the animation records,
  the null-pointer write at `$64`, and the map read through `map_rows` pointers.
- **`data` (15).** Addresses of the original's tables read in place from the loaded hunk image:
  animation records, path tables, the witch's flash palette, the gates' marker tables, the
  fountain's trigger step, and `template(t)`, the templates in `obj_templates`. These lines are
  mostly `const` addresses and the `bob_anim` calls that use them.
- **`shell` (0).** None. The checks are in `sf-conform` ([verification](verification.md)).

## 3. Mechanisms it uses

Terms are the [glossary](glossary.md)'s.

- **Byte-exact state at original addresses** ([memory-model](memory-model.md)). Every slot field,
  the map, the frog (`FROG + f::X`), the camera (`CAMERA_X`, `CAMERA_Y`), `SCORE`, `LEVEL_STATE`,
  the fade records and the counters inside the code are read and written by address through
  `Bus`. Routine identity is an address too.
- **The layout table** ([memory-model](memory-model.md)). The counters inside the code are
  `LAYOUT` variables (`W1HoverPhase`, `W1SpoutTurn`, `ChainLinkCount`, `ThrowerVx`,
  `FlierFacingRight`, `CannonAim`), so conformance compares them. `obj_slots`, `obj_count`,
  `hitboxes` and `level_map` are gameplay variables, never presentation
  (`no_gameplay_variable_is_presentation` in `engine/crates/sf-core/src/layout.rs`).
- **The loaded hunk image** ([copied-in-data](copied-in-data.md)). Animation records, paths,
  templates and tables are read where the executable put them.
- **Cycle charging** ([cpu-time-and-beam](cpu-time-and-beam.md)). Charged per routine call by
  `level::call` from `beam::routine_cost`, not in the owned files.
- **The 25 Hz main-loop pass** ([frame-interrupts-input](frame-interrupts-input.md)). Every update
  runs once a pass from `obj_run_all`, in slot order. The frog they chase moves at 50 Hz in the
  handler.
- **The blitter** ([display-hardware](display-hardware.md)), indirectly. The types only point
  their bob at an animation record and move it. The level's bob drawing (`level/bobs.rs`) blits
  them.
- **Paula writes** ([sound-driver](sound-driver.md)). `c.sfx` and `c.sample` in the touch
  handlers and a few updates go to the sound driver.

## 4. What depends on it

- **[level](level.md).** `standard_dispatch` calls `enemies::register_all` and
  `bosses::register_all`; without them every marker of worlds 1–6 spawns nothing and every
  transplanted enemy slot is reported unported. The map writers change the cells the frog's
  collision reads.
- **[slot-and-bonus](slot-and-bonus.md).** `touch_stomped` calls `bonus::death::become_reward`.
  The bonus level's own enemies use their own handlers.
- **[frog-and-items](frog-and-items.md).** `touch_hurt` calls `level::death::frog_hurt`, and the
  types read the frog's position. The stomp bounce is the frog's.
- **[game-flow](game-flow.md) and [story-and-ending](story-and-ending.md).** The witch's
  `level_state` `$64` ends the final level and leads to the end sequence.
- **[verification](verification.md).** The routine tests and the level cases in §5 call these
  routines through `standard_dispatch`. The milestones `CoinTaken` and `LevelExit` depend on the
  enemies behaving, because a route scripted against the original fails if an enemy hits the
  frog where it did not.
- **[app](app.md).** Every level the app plays runs them.

## 5. What a native rewrite must preserve

Faithful stays the bar. Each row is behaviour the original shows, and the check that holds the
engine to it today. Unless a path is given, tests are in `engine/crates/sf-conform/tests/`, and the
routine tests are in each world's `enemies_w{n}_routines.rs` (`enemies_w1_routines.rs` …
`enemies_w6_routines.rs`). A routine test runs the original's routine on the Moira 68000 emulator
from the original's memory and the port from the same memory transplanted, over generated states.
It requires every byte written, the sound driver's state and the Paula writes to match
(`engine/crates/sf-conform/src/routine.rs`).

| Behaviour to keep | Check that proves it |
|---|---|
| Each world's markers spawn the original's template at its offset, into the original's slot, and the same cell never twice. A paired type spawns one end only while the other has no live slot. | `spawners_match_the_original` in every world's routine test; `bosses_routines.rs` `the_spawner_matches_the_original`; `a_patroller_out_in_any_slot_stops_the_spawn` (`enemies_w1_routines.rs`). |
| Which templates and markers exist per world. | `the_kinds_and_markers_are_registered` (each routine test); `the_witch_and_her_marker_are_registered`. |
| Every init and update leaves the slot, its bob, the map, the counters and the RNG exactly as the original's: positions, timers, state changes, turns, the frames that step, the RNG draws in the original's order. | `inits_match_the_original` and `updates_match_the_original` in every world's routine test; `init_matches_the_original`, `updates_match_the_original` (`bosses_routines.rs`). |
| Edge behaviour at thresholds: the hover's drift bounds, the watcher's 16 directions, a link's sixths, the charger's reach, the cannon's aim and facing at every frog x, the walkers turning on the same pixel, the ball freed with its fifth link, the witch's patterns turning at their ends. | `the_hover_drifts_as_the_original_at_its_bounds`, `the_watcher_looks_as_the_original_at_every_threshold` (W1); `a_link_sits_as_the_original_at_the_divides_edges` (W3); `the_watchers_looks_are_sixteen` (W5); `the_chargers_reach_matches_at_its_edges`, `the_aim_and_facing_match_at_every_frog_x`, `the_walkers_turn_at_the_same_pixel`, `the_ball_goes_with_its_fifth_link` (W6); `the_patterns_turn_at_their_ends` (`bosses_routines.rs`). |
| The paths and tables end where the listing says. | `the_paths_end_where_the_listing_says` (W1, W3, W4); `the_tables_end_where_the_listing_says` (W2). |
| Touch: hit points taken (`$20`, or 1 for the witch, whatever touched her), death into a fall or a flattening, the marker cleared, the effect and channel (RNG-picked for a kill), the reward and the 200 points, the witch's flash and `level_state`. | `touches_match_the_original` in every world's routine test, including the crawler's h0+$2d128 word varied over 0–3 and `$ffff` (W1); `the_hit_counter_matches_the_original` (`bosses_routines.rs`). |
| The counters inside the code stay at their original addresses, compared as state. | `the_counters_are_layout_variables` (W4); `the_aims_and_counters_are_layout_variables` (W6). W1's `w1_hover_phase` and `w1_spout_turn` are `LAYOUT` variables, but no test asserts it. |
| The write of the ball through a null slot pointer to `$64` when a link finds no slot. | No check names it (as `memory-model` notes). W3's spawner test fills some link cells but does not set up full slots. To find out, add a spawner case with all 20 slots live. |
| Frame for frame in a level with every object live: slots, hitboxes, the map's markers, score, lives and the picture of every enemy, equal to the original's. The frog meets, is hurt by and kills the enemies each case names. | `enemies_w1.rs` (`w1_1` … `w1_4`, `w1_activation`, `w1_hover_kill`); `enemies_w2_cases.rs`; `enemies_w3.rs`; `cases.rs` `enemies_w4_1`, `enemies_w4_2` and `frog_cases.rs` `enemies_w4_4_activation`; `enemies_w5_cases.rs`; `enemies_w6_cases.rs`; the cases `engine/conform/cases/enemies-*.toml`. |
| Whole levels completed through their enemies, compared and run alone (check 4, rust-engine DESIGN §2.1). | The `*_complete` tests in `enemies_w2_cases.rs`, `enemies_w3.rs`, `enemies_w5_cases.rs` and `enemies_w6_cases.rs`. Several pin re-syncs, or are red under checks 1–3 and listed in rust-engine FINDINGS (2026-10-18 T29, T212). |
| Which slot each spawn takes and which slot is evicted when all 20 are full, so the slots' order matches the original's. | `enemies_w3_3_slot_order` and `enemies_w3_3_slots_alone_in_the_originals_order` (`enemies_w3.rs`). The case still re-syncs at 3634, 4028 and 12237 (rust-engine FINDINGS 2026-10-16, T195). |
| The pass's CPU time, so a stomp or a coin lands in the original's frame when a pass straddles a vertical blank. Today this is `Cost::Routine` and the two measured costs, an approximation. | `enemies_w6_4_stomps_after_the_blank_its_frame_ended_before`, `enemies_w6_4_alone_waits_out_a_pass_due_at_the_frames_end`, `enemies_w6_4_takes_its_coins_with_the_box_from_before_the_blank` (`enemies_w6_cases.rs`). No test checks the cost values themselves against the original per type. |
| The final level: the witch's rise, patterns and 128 hits, the flash stepping back, and the level ending 100 passes later into the end sequence. | `final_win_to_the_levels_end` (`bosses.rs`, in the test block); `final_win`, `final_lose`, `final_idle` (ignored as slow, run by `sf-conform -- all`). |

**A note on shared routines.** The darts' wait and flight (h0+$25fa8, h0+$25f42) are ported
twice, as `w3::dart::wait`/`fly` and `w4::dart::dart_wait`/`dart_fly`. The swing's updates
(h0+$2619c, h0+$262be) are also ported twice, as `w3::swing::update`/`link_update` and
`w4::mace`'s private copies. The dispatch keeps the last registration, and W6 registers after W4
with W3's functions, so the W4 copies never run in `standard_dispatch`. A rewrite keeps one of each.

## 6. Native sketch (non-binding)

This sketch is non-binding: designing the rewrite is the later plan's job.

The patterns of §1 suggest about ten behaviours with parameters, not ninety files. A walker is
(speed, step frames, turn rule, stompable, death); a paired pacer is (speed, pair markers, start
end); a dart is (look, speed, wait). Each type would become an enum variant or a small struct with
named fields (`Walker { x, y, vx, facing, hp }`), stepped once a pass, and its template would
become a table entry giving its parameters. Routine identity by address would become the variant,
and a state change would become a field. The touch handlers would become one function over a
`Death` choice (fall, flatten, unkillable, boss).

Four things the rewrite has to settle first:

- **Comparing slot state.** Conformance compares `obj_slots` byte for byte and transplants the
  original's slots mid-level. A native object list needs a mapping to and from the slot record,
  including the routine addresses in `+$04` and `+$10`, or a change to the checks
  ([memory-model](memory-model.md) and [verification](verification.md), DESIGN §2.5).
- **Slot order and eviction.** Which slot a spawn takes, and which off-screen slot is reclaimed
  when all 20 are full, change the update order. Update order changes the RNG draw order and so
  everything after it. A native list has to keep the 20 places and their order.
- **The quirks.** The crawler reads a word in the animation data, the null-pointer write lands on
  an interrupt vector, and the hover and swooper share one phase counter. Each is behaviour the
  original shows, so each becomes explicit native state with a comment saying why.
- **Animation as data.** The bob records and animation steps are what the picture check sees, and
  several rules read an animation's step. The animation tables could be decoded once into native
  frame lists, as long as the step a rule reads still lands on the same pass.

## 7. Reuse

Rating: **`superfrog-only`**. 5 849 lines (`python tools/architecture.py totals`).

Every type, address, table, template index and marker is this game's. For another Amiga game
only the method carries: walk the original's spawn table, port each routine against the original
on the routine oracle, register it by its original address so transplanted state runs on, and
check whole levels against the original with every object live. The dispatch skeleton that would
carry (`Dispatch`, `ObjectKind`, `register`, `register_routines`, `call`, with unported routines
reported rather than guessed) lives in `engine/crates/sf-core/src/level/mod.rs` and is rated in
[level](level.md). In these files only `common::direction`'s shape (a 16-way direction from two
points) is generic, and its thresholds are Superfrog's. Another Team 17 game may share the
`obj_flag_jumptab` and slot-record design and the touch-handler set (unchecked).

## 8. Sources

- [docs/09](../09-object-system.md): placement, the slot record, touch handlers, types per world,
  the witch.
- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1 (faithful, check 4).
- [rust-engine FINDINGS](../../plans/rust-engine/FINDINGS.md): the witch (2026-10-15, T18),
  slot order (2026-10-16, T115, T195), the red and pinned enemy cases (2026-10-15 T184,
  2026-10-18 T29, T212).
- Code: the owned files; `engine/crates/sf-core/src/level/mod.rs` (`Dispatch`, `call`,
  `standard_dispatch`), `engine/crates/sf-core/src/level/objects.rs`,
  `engine/crates/sf-core/src/level/beam.rs` (`routine_cost`), `engine/crates/sf-core/src/layout.rs`.
