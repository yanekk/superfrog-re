# 10 — The object system, W1-1 measured

What each value of W1-1's object map (`_MapaBobow1`, [05](05-file-formats.md)) does, measured on the
oracle (plan `forest-dumb-level-1`, T21, 2026-11-05). The kept evidence is
`engine/crates/fd-conform/tests/object_claims.rs`; the list with each value's porting task is
[`re/objects-W1-1.toml`](../re/objects-W1-1.toml), which `fd-conform/tests/ported.rs` checks against the map.
Addresses are `hN+$x` ([07](07-executable-map.md)); routine names are `re/translate.toml`'s.

## How it was measured

One scripted run per claim, from the level sync point ([09](09-play-measured.md)): hold right, and the
character slides down the first slope from `W1-1:7,7` to `W1-1:42,35`, where a wall stops it at pass ~135
(x 669, y 553). The edge redraws bring new cells into view as it goes. A value either lies on that path
as shipped (**walk**: values 1 and 6), or is poked into a cell on the path at the sync point, before the
cell is in view, so the original's own code meets it (**placed**). T31's counter shows each entry point
ran; the state after the run shows the effect. The cells: `W1-1:24,20` (the body, pass ~71), `W1-1:28,23`
(a creature ahead, pass ~83), `W1-1:36,30` (under the feet, pass ~107, while `h0+$4ed0` is set). One run
takes about 0.3 s from the cached sync point.

## Every value in W1-1

Every non-zero value of the 240 used columns (tested). "Entry" is where the value first acts: the scan
in the edge redraws (`redraw_edge_a..d`, hunk 0 `$9b2`, `$c92` and their twins) dispatches on the value by
compares, never through a table.

| Value | Count | Example | Group | Entry | What it does | Seen how |
|---|---|---|---|---|---|---|
| `$01` | 77 | `W1-1:2,89` | creature | `h0_681c` | creature kind 0; reading: flies at the character on a 32-step path | walk, `W1-1:26,19` |
| `$02` | 16 | `W1-1:9,92` | creature | `h0_681c` | creature kind 1; reading: walks a pixel a pass, turning on `collision_map` probes | placed |
| `$03` | 15 | `W1-1:5,92` | creature | `h0_681c` | creature kind 2; reading: walks on a speed table | placed |
| `$04` | 21 | `W1-1:6,40` | creature | `h0_681c` | creature kind 3; reading: hops on x and y tables | placed |
| `$05` | 4 | `W1-1:45,92` | creature | `h0_681c` | creature kind 4: walks on a speed table, a pixel every other pass, turning at walls (T77) | placed; walk, `W1-1:45,92` (`enemy-kind4`) |
| `$06` | 30 | `W1-1:30,11` | creature | `h0_681c` | creature kind 5; reading: walks on a speed table | walk, `W1-1:30,11` |
| `$10` | 2 | `W1-1:222,56` | spring | `h0_24ce` | spring, left half: throws the character up | placed |
| `$11` | 1 | `W1-1:224,56` | spring | `h0_24ce` | spring, right half | placed |
| `$20` | 128 | `W1-1:2,73` | item | `h0_6882`, `h0_26ec` | coin: +100, `items_left` -1 | placed |
| `$21` | 108 | `W1-1:2,74` | item | `h0_6882`, `h0_26ec` | coin, spin one frame on | placed |
| `$22` | 51 | `W1-1:23,72` | item | `h0_6882`, `h0_26ec` | coin, two frames on | placed |
| `$23` | 45 | `W1-1:2,77` | item | `h0_6882`, `h0_26ec` | coin, three frames on | placed |
| `$28` | 11 | `W1-1:136,66` | water | `h0_6882` | animated water surface; no state | placed |
| `$30` | 8 | `W1-1:22,86` | vanishing block | `h0_6882` | fades out and back; solid while shown | placed |
| `$31` | 8 | `W1-1:25,86` | vanishing block | `h0_6882` | a quarter cycle on | placed |
| `$32` | 7 | `W1-1:22,80` | vanishing block | `h0_6882` | half a cycle on | placed |
| `$33` | 3 | `W1-1:25,80` | vanishing block | `h0_6882` | three quarters on | placed |
| `$40` | 200 | `W1-1:2,37` | bonus | `h0_a06` etc., `h0_26ec` | orange: +20 | placed |
| `$41` | 129 | `W1-1:2,38` | bonus | as `$40` | apple: +30 | placed |
| `$42` | 105 | `W1-1:2,39` | bonus | as `$40` | cherries: +40 | placed |
| `$43` | 1 | `W1-1:172,58` | bonus | as `$40` | pear: +50 | placed |
| `$44` | 13 | `W1-1:11,91` | bonus | as `$40` | banana: +60 | placed |
| `$48` | 34 | `W1-1:6,24` | bonus | as `$40` | diamond: +100 | placed |
| `$ff` | 6 | `W1-1:207,91` | exit | `h0_26ec` | sets `level_exit_touched` | placed |

Scores are BCD (`score_add`), so `+20` is `$20`. The fruit names are read off `_ELEMENTY`'s second row.

## Each group

**Creatures (1 to 6).** When an edge redraw brings the cell into view, `h0_681c` finds a free slot of
five and `h0_6852` fills it: `creature_slots` (`h0+$67ec`, 8 bytes each: x.w, y.w, kind.w, direction.b,
phase.b), kind `value - 1`, direction +1 or -1 by which half of the view the cell is in; `creature_active`
(`h0+$67da`) flags it. The cell is cleared; with no free slot it stays and is met again next time.
Measured: spawn, the slot's kind, and `h0_2fea`'s branch for that kind running every pass. Every pass
`h0_2fae` runs `h0_2fea` on each active slot, which branches on kind (compares, no table): kinds 0 to 5
move, kind 7 counts its phase to `$20` and frees the slot. When a creature leaves the view, `h0_2dd6`
(`h0_2e0e`) writes `kind + 1` back into the map near it and frees the slot (measured: counter, map
cell). `h0_3698` sorts the slots by y for drawing; `h0_5820` cuts them onto the screen and `h0_5e44`
restores behind them. Kind 6's frame table points at blank frames ([06](06-graphics-and-rendering.md)), and
no value spawns it. The movements per kind are readings of the listing; T24 measures them by porting.

**Contact (measured, T59, `object_claims.rs`; modelled, T70, `contact.rs`).** `char_hit_check` reads CLXDAT
(`$dff00e`, its bits 1 and 2: odd bitplanes against sprites 0 and 2), not a box test. CLXCON is `$37dd` from
the play copper list (`h1+$118`). In single playfield the oracle compares the odd planes only where the even
ones match too (vAmiga's `checkS2PCollisions`), so contact is a pixel of colour 29 alone under the
character's sprites, drawn in front; 21, 23 and 31 match the odd planes but not the even. The status bar
holds colour 29, so bit 0 (playfield against playfield) is always set in play, and pass 1 reads `$f1ff`.
The level-3 interrupt comes at line 233, so a pass's read covers the play area of the frame shown since the
last pass: the engine models it from that frame (`fd_core::render::contact`). Contact does
nothing while `h0+$4b62` is set (after a life is lost). With `h0+$4ed0` set (falling), `h0_3dd8` tests
each creature's 48 x 32 box against the character + (`$10`, `$24`); the first one hit becomes kind 7 and
scores 250, and no hit is counted that pass. Otherwise, once `h0+$9b04` is 0, `hits_taken` counts
contact passes, cleared on any pass without, and the third in a row costs a life. `h0+$9b04` counts up
from the level start and wraps to 0 about 100 passes after the sync point: until then contact never
hurts. Measured: a creature (kinds 0, 1, 5) over the standing character, hits 1, 2, 3, a life lost on
the third; one 200 pixels away, no contact. A creature under the falling character, kind 7 and +250.
`$28` water under the feet hurts (contact passes 107 to 109 of the slide, a life lost), and so do the
tiles where the first slope ends against a wall (`W1-1:42,35`, no creature in the map: contact from pass
133, a life lost at 135). Which tiles carry the colours there is not measured. In W1-1's own water (T72, 2026-11-06,
`death-water`, `contact.rs`): with the water drawn the character drops into the
pool at `W1-1:148,66` to `149,66` (frame 7042), and three contact passes cost a life at frame 7062 and
the next at 7204, engine and original equal on every pass and frame (T72's inputs, creatures frozen;
since T79 the case reaches the pool with creatures live, at `149,66`, and the lives go at pass 2211 and frame 8652). The pool's tiles carry colour 29
themselves: the same run with the water undrawn loses the same two lives, and the water's frames change
CLXDAT only on a few passes (on T72's inputs pass 773 sets a bit the tiles do not; 758 and 786 to 793 cover theirs). Those colours are in
134 of W1-1's 328 tiles, in the coin and water frames and in `_ZNIKAJKA1` (counted for 21, 23, 29, 31). The oracle reports
collision only since T59 (`DENISE_CLX_SPR_PLF`, off in vAmiga by default; before it CLXDAT read `$8000`).

**Springs (`$10` to `$13`).** `h0_24ce` reads the cell under the feet (`char_x + $e`, `char_y + $23`) each
pass, only while `h0+$4ed0` is set and `h0+$4b62` is clear (measured: the same spring with the flag clear
does nothing). It sets `h0+$4ecf` and `h0+$41b6`, which `char_step` takes as a throw, and marks the spring's
position; `$10` and `$12` take the redraw origin 14 pixels right of `char_x`, `$11` and `$13` 2 left, and
`$12`, `$13` set `h0+$21be`. Six passes later `h0_21c0` redraws the 2 x 2 tiles there and starts a sound.
Measured: the character thrown up to y 264 instead of sliding on. The cell stays, so a spring fires
again. The picture is `_WYBIJACZKA`.

**Items (`$20` to `$27`).** In view, `h0_6882` adds the cell to `h0_62b6`'s animated cells (four lists of
12 at `h2+$1680`, positions and values at `h2+$16b0`; the shortest list takes it). Each pass `h0_62b6`
drops cells whose map value is gone or that left the view, and draws `_ELEMENTY` frame
`(value - $20 + tick) & 7`, the tick at `h0+$62b2` stepping every fourth pass. `char_object_cells` reads
the cells at `char_y + $19` and `char_y + 9`, column `char_x + $e`: `h0_26ec` takes an item, clears the
cell, redraws its tile (`h0_29ca`), adds 100 and takes one from `items_left`. W1-1 holds 332 items;
`items_left` starts at 282 (set by `load_list_world_1`), so 50 are spare. `h0+$7a9e` counts `$20`-`$27`
over the whole map but is not what sets `items_left`.

**Water (`$28`) and the other animated cells.** `$28` draws `_ELEMENTY` frames 8 and 9, or 8 to 11 when
`h0+$6342` is set; `$2a` frames 10 and 11. `h0_26ec` ignores `$28` to `$3f`, so water changes no state
except, perhaps, through its pixels' colours (above). `$29`, `$2b` to `$2f` and `$35` to `$3f` end
`h0_62b6`'s loop for the pass (an `rts` mid-loop); none is in W1-1.

**Vanishing blocks (`$30` to `$33`).** `h0_62b6` draws `_ZNIKAJKA` (32 x 16) at the frame the table at
`h0+$63ec` gives for `(value - $30) * 16 + tick`, a 64-step cycle, so the four values run a quarter apart.
On every frame but the empty one (offset `$14`) it writes `collision_map` solid (`$fffffff0`, 28 pixels on 8
lines); on the empty one it clears it. So `collision_map` under a block changes with time: play state.
`$34` draws `_Plomien` (a flame) and writes nothing; not in W1-1.

**Bonuses (`$40` to `$5f`).** The edge redraw that brings the cell in blits `_ELEMENTY`'s second row,
frame `value - $40`, into the screen once (`h0_a06`, `h0_ce4`, `h0_104c`, `h0_1390`, one per edge). Taken by
`h0_26ec`: the cell cleared, the tile redrawn, the score raised by `$20` to `$5000` by value; `$4c` instead
adds 100 to `countdown` with its own sound. Only `$40`-`$44` and `$48` are in W1-1.

**The exit (`$ff`).** See [09](09-play-measured.md), "Level end". The scan in the edge redraws also stops
at a `$ff` cell. Measured here: `level_exit_touched` set on each pass on it, nothing else with items left.

## Taking and writing back, ported (T22)

`fd-core/src/objects/collectible.rs` ports `char_object_cells`, `h0_26ec`, `h0_29ca`, `score_add_routine`,
`countdown_plus_100`, `items_left_minus_one`, `h0_2dd6` and `h0_2e0e` (rig: `fd-conform/tests/collect_routines.rs`).

**Where the original can be stopped partway** (`toolkit/docs/porting-routines.md` §1), listed before
porting. All eight run inside `game_frame`, so inside the level-3 handler: the vertical blank cannot
cut them, and `main_loop` runs only once the pass ends. What waits: `item_sound` (`h0+$2648`, and its
twin inline for `$4c`) waits for four VHPOSR line changes between switching the channel's DMA off and
on, and `h0_29ca` waits for BLTBUSY before its blit. Neither changes game state, and both finish
inside the pass. Values latched: `object_row` (`h0+$2692`) and `object_taken` (`h0+$29c6`) are
written before each read on every path; `creature_slot_index` (`h0+$2faa`) is set by `h0_2dd6`'s
loop. The ABCD and SBCD adds take the X flag as 0: `score_add_routine` and `items_left_minus_one`
clear it, and `countdown_plus_100`'s only caller reaches it after `h0_29ca`'s `lsl.l #2` of a word,
which shifts out a 0. Nothing is a loop left and re-entered on a later pass.

**The screen is state?** The taken cell's tile is blitted over the play screen (`h0_29ca`, tile offset
from the table at `h0+$3b988`, which holds `n * $a0`). The engine adds it to the screen's blits
(`Blit::Cell`); nothing reads the screen back as game state.

## Creatures, ported (T24)

`fd-core/src/objects/creature.rs` ports `h0_2fae`, `h0_2fea`, `h0_320a`, `h0_3222`, `h0_32f8`, `h0_3322`,
the four probes `h0_3530`, `h0_3566`, `h0_35a0`, `h0_35dc`, the sort `h0_3698` and the stomp's box test
`h0_3dd8`; `char_hit_check`'s stomp loop is `character::Play::stomp` (rig: `fd-conform/tests/enemy_routines.rs`,
`death_routines.rs`). A creature lives in its slot, never as a `crate::Object`: no `ObjectKind` is registered.

**Where the original can be stopped partway** (`toolkit/docs/porting-routines.md` §1), listed before
porting. All run inside `game_frame`, so inside the level-3 handler: nothing cuts them, and none is a
loop left and re-entered on a later pass. The one wait is the stomp sound's (`h0_3adc`): four VHPOSR
line changes between channel 2's DMA off and on, which change no game state. Values latched:
`creature_slot_index` (`h0+$2faa`) is set by `h0_2fae`'s loop before each `h0_2fea`, which reads it only
for kind 7 (it clears that slot's flag, not `d0`'s). Write-only: `h0+$2fe6` (`h0_2fea`'s `d0`), `h0+$306e`
(set to 1 by kind 7) and `h0_3698`'s sorted copy at `h0+$366a`, which equals the slots after the sort; its
`move.w` back writes `creature_active+5` as 0. `h0_3698` compares the middle pairs' y as bytes and the
outer as words, each by the result's sign.

**The screen is state?** Not here: these routines write slots only. The creatures are cut onto the play
screen by `h0_2d4c` (T52), and contact with them reaches `char_hit_check` through CLXDAT from that
picture, so the engine's creatures hurt the character only once T52 draws them. The stomp needs no
picture: it tests the slots' boxes against the feet.

## Mechanics, ported (T25)

`fd-core/src/objects/spring.rs` ports `h0_24ce` (a spring under the feet) and `h0_21c0` (its picture);
`fd-core/src/objects/animated.rs` ports `h0_62b6` (the animated cells: items, water, vanishing blocks,
flames). Rig: `fd-conform/tests/mechanic_routines.rs`. Like collectibles they stay map cells: no
`ObjectKind` is registered for `$10`, `$11`, `$28`, `$30` to `$33`.

Since T47 (2026-11-09) no case freezes `h0_6882` or `h0_24ce`, on either side; `mechanic-vanish` and
`mechanic-spring` hold them to the original (`fd-conform/tests/mechanic_cases.rs`). Measured: the
original's pass grows with the animated cells' blits and more pass ends go unseen (`camera-right-left`
24, `collect-return` 51). A pass long enough to end after the next frame starts, while `main_loop` is in
an edge redraw, has its `frame_flag` cleared by `main_loop` when that redraw ends, so its latch waits a
pass: `edge_latch_down` differs for one pass (`camera-bottom-right` pass 495, `mechanic-vanish` 293),
and on `mechanic-spring` the row is scanned a pass late, so its coins join other animated-cell lists
and spin out of step. Since T74 the case runner measures where each pass cut `main_loop`'s redraws and
hands it to the engine (docs/09, "The edge redraws and the next pass"); no exclusion is left.

**Where the original can be stopped partway** (`toolkit/docs/porting-routines.md` §1), listed before
porting. All three run inside `game_frame`, so inside the level-3 handler: the vertical blank cannot cut
them. What waits: `h0_21c0` waits for BLTBUSY before each of its four tile blits and two picture blits,
and at `spring_timer` 1 for three VHPOSR line changes between switching audio channel 1 off and on
(`h0+$23aa`); `h0_62b6` waits for BLTBUSY before each blit. None changes game state, and all finish in
the pass. Values latched: `spring_x`, `spring_y` and `spring_kind` are set by `h0_24ce` in the pass a
spring fires and read by `h0_21c0` one and six passes later; a spring fired again in between moves them,
as in the engine, which keeps them in `State`. `anim_tick` carries the phase (which of the four lists runs)
and the tick from pass to pass. Loops left and re-entered: `h0_62b6` returns mid-list on a value it does not
draw (`$29`, `$2b` to `$2f`, `$35` to `$7f`), leaving the list's later cells for its next turn, four passes
on; the engine does the same. `h0_24ce` has none.

**The screen is state?** `h0_62b6`'s item and water blit draws the frame over the cell's tile (C is the
tile, not the screen); the vanishing block, flame and spring blits copy a picture. None reads the play
screen, so it stays picture state (`Blit::Element`, `Blit::Picture`, `Blit::Cell`). `collision_map`
under a vanishing block is game state: the engine writes it into the collision plane.

**`water_cycle`** (`h0+$6342`) is set by `load_list_world_1` (and `_2`), so W1-1's water runs `_ELEMENTY`
frames 8 to 11.

## Values W1-1 does not hold

Read, not measured: `7` to `$f` and `$14` to `$1f` are skipped by the scan and by `h0_24ce`; `$12`, `$13`
are springs, `$24` to `$27` coins, `$2a` water, `$34` a flame, `$45` to `$4b` and `$4d` to `$50` bonuses,
`$4c` time. The placed runs measured `$12` and `$13` too.

## Mechanics in W1-1

| Mechanic | Spot | Values |
|---|---|---|
| Springs | `W1-1:222,56` to `224,56` | `$10`, `$10`, `$11` |
| Vanishing blocks | `W1-1:22,86`, `25,86`, `22,80`, `25,80`, and on to `W1-1:195,27` | `$30` to `$33`, 26 cells |
| Water | `W1-1:136,66` to `141,66` | `$28`, 11 cells |
| Exit | `W1-1:207,91` to `208,93` | `$ff`, 6 cells |
| Countdown | the whole level: 999 at start, -1 every 63 passes, game over at 0 ([09](09-play-measured.md)); measured falling while sliding | none |
| Contact damage | creatures, `$28` water, the tiles at `W1-1:42,35`: three contact passes in a row cost a life; a falling character stomps a creature (kind 7, +250) | `$01`-`$06`, `$28` |

## Freezing objects (DESIGN §2.2)

**The freeze is gone (T79, 2026-11-10).** No case or test returns early from a routine of the
original, and the case format has no `freeze` or `thaw` key. History: T21 froze `h0_681c` (no creature
ever started), `h0_6882` (no animated cell), the four bonus blits, `h0_26ec` (nothing taken, no exit)
and `h0_24ce` (no spring) in `fd-conform/src/freeze.rs`; T22 removed taking and the bonus blits, T47
`h0_6882` and `h0_24ce`, T46 and T75 ran the enemy cases live, and T79 the last users of `h0_681c`.

What the freeze hid, measured when it went: a frozen `h0_681c` left each creature cell in the object
map, where the edge redraw's scan cut it as a bob, so the frozen runs' rows of 8 and 9 bobs were
creature cells (docs/09, "The edge redraws and the next pass"). A claim that needs no creature
about takes the creature values (1 to 6, `$87` to `$fe`) out of the map instead
(`tests/object_claims.rs`, `no_creatures`): so isolated, the slide still ends against the tiles at
`W1-1:42,35` in contact from pass 133 and loses a life at pass 135.

Creatures run live in every case, held by `fd-conform/tests/enemy_cases.rs`. Measured (T46): a pass
made longer by drawn creatures can cut `main_loop`'s edge redraw (interrupted in `h0_3ac2`) before
its scan reaches a creature cell, so the original starts that creature after the pass, a pass later
than an engine that runs the redraw whole, and it stays a pass behind.

### A creature start the next pass cuts (T75, measured 2026-11-09)

The engine models no sub-frame timing (DESIGN §2.1), so the runner hands it the cut, as for the
latches and the animated cells (T74; docs/09, "Every cut, measured"). `fd_conform::edges_cut` reads
where the pass took the foreground; inside a redraw's scan the cell it stood at is `a2`, and a start
still in the scan's creature branch (`h0+$abc`, `$d98`, `$10fc`, `$1444` to each `bsr.w h0_681c`) or
in `h0_681c` and `h0_6852` short of the slot's `st.b` (`h0+$687a`) counts as not made. The engine
(`level::main_loop_redraws`, `Cut::At`) draws the cut redraw whole but holds the scan's creature
cells from that cell on in `Edges::deferred` with its animated cells, and starts them (`h0_681c`,
then the cell cleared) after the pass, slots as that pass left them. Alone (the app, the planner's
search) the engine starts every creature in its redraw: which pass is cut is the original's cycles.

Measured on the original, every case comparing the five slots on every pass: without the hold the
engine has the creature a pass early at pass 58 (`collect-ledge`, `collect-return`), 93
(`camera-right-left`, `camera-bottom-right`, `enemy-hop`), 641 (`enemy-contact`, `contact-route`),
571 (`death-water`, live) and 1250 (`enemy-route`, `edge-latch-route`); with it all agree to their
ends (`enemy-route` 1 308 steps, `enemy-contact` 1 300, `camera-bottom-right` 2 165). In every one of
those cuts the scan had not passed a creature cell, so holding the whole scan agrees too: the place
inside a scan is proven for animated cells only (`mechanic-spring`, pass 495). A cut said where none
was measured starts W1-1:27,2's hopper a pass late and differs at pass 84
(`enemy_cases.rs`, `a_creature_start_waits_only_where_the_measured_cut_says`).

Seven of T46's eight frozen cases run live since T75. Live, creatures hurt and are stomped, so
`camera-bottom-right` and `collect-return` have waits and jumps added to reach their goals, and
`collect-ledge`, `camera-right-left`, `character-edge-pit`, `contact-route` and `edge-latch-route`
keep their inputs with the goal both sides now reach. T79 (2026-11-10) gave the last three their live
inputs, each found with the original in the loop (`plans/forest-dumb-level-1/prototype/t79-route-repair/`):
`death-water` and `mechanic-vanish` by fd-plan's explorer planned from the original's states, no life
lost; `mechanic-spring` from `camera-bottom-right`'s live opening. The planner's confirmation
(`fd_plan::confirm`) runs live too: the 100-round route agrees on every pass but ends elsewhere than
the engine alone from 120 steps on, the engine alone not knowing the original's cuts.

### A creature the engine starts a pass early (T79 review, measured 2026-11-10; T81 fixes it)

`mechanic-vanish`, pass 330 (oracle frame 6632): the runner hands the cut `next: 1`, `defer_from`
59008, the engine's row redraw starts the kind 0 creature at `$429,$482` before the pass, and the
original has not started it when the pass ends (slot 0 still the creature freed at `$3bb,$3a2`, its
flag 0). A pass later flags and places agree; the kind, facing and phase longs stay apart
(`creature_slots+$4` 28 passes, `+$c` 544). The character never meets it, so every other variable
agrees. The case excludes those five slot variables naming T81; `death-water` and
`mechanic-spring` compare all fifteen and agree.

### A creature cut behind the beam (T79, measured 2026-11-10; T80 models it)

With several creatures drawn a pass runs long, and `h0_5820` cuts a creature after the beam has
passed its lines: that frame shows it restored and not yet cut, so it is missing, where the engine,
drawing a pass whole, shows it. Measured on `mechanic-spring` from pass 475, five creatures in view:
the play screen's bytes equal the engine's where each pass ends, the frame differs (445 pixels at
frame 6779, a creature at the top of the view), 3 frames and more in a row, past the tolerance's 2.
Frames differing so: 41 of `mechanic-spring`'s 3 609, 59 of `mechanic-vanish`'s 1 469, 30 of
`death-water`'s 2 364, at any height of the play area. Those three cases exclude the play area's
picture (`rows 0..194`), naming T80 (owner, 2026-11-10); their variables and status bar stay compared.

### Kind 4 in play (T77, measured 2026-11-10)

W1-1 holds four `$05` cells: `W1-1:45,92`, `145,52`, `156,50` and `187,17`. None of T46's routes
brought one into view (the nearest, `enemy-contact`, came within 180 pixels of `45,92`). The bottom
corridor (rows 88 to 93, columns 2 to 110) is open from where `camera-bottom-right` drops into it at
`W1-1:93,91` (step 700), so `enemy-kind4` walks left from there, over the mound at columns 52 to 59,
and stands at `W1-1:45,92`. On the original and the engine, every slot equal on all 1 390 passes
compared: the creature starts facing right as the left column's redraw brings `45,92` in, walks from
x 723 to the mound's wall at x 784 a pixel every other pass, turns there in the pass that makes its
last step right, walks left past the standing character to x 586, leaves the view and is written
back (`h0_2e0e`), the map holding `$05` there again. Kind 4 has no routine of its own: its walk is
inline in `h0_2fea` on the tables at `h0+$33f6` and `$3416`, so no coverage count shows it and
`enemy_cases.rs` reads the engine's slots instead; the case fails on a slot within 16 passes of the
start when the right table is changed. The other three cells are not visited. Kind 2 turning right
(`h0_3222`) is held on `enemy-contact`: one walking left turns at x 1720 (pass 665) and steps on.

The case compares state only. Its picture differs at pass 477 (oracle frame 6779, 445 pixels from
(172, 11)), on `camera-bottom-right`'s stretch before the corridor, which never compared the
picture; so no case compares kind 4 drawn in play (the rig does, `creature_drawing_routines.rs`).

## No dispatch tables

The plan expected object routines reached only through dispatch tables, outside T34's call-tree scan
(DESIGN §4.3). There are none: the scan and `h0_2fea` dispatch by compares, `char_object_cells` and
`h0_24ce` by compares, `h0_62b6` by compares and a frame table of data. Every routine the values run
through is already in `re/translate.toml`, so T21 appended nothing to it and T49 has no routine to
translate. The four `jmp (an)` in the program (`h0+$435bc`, `h4+$12ddc`, `h4+$19a66`, `h4+$19bc2`) are
outside play's call tree.
