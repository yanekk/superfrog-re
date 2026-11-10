# The route through the whole of W1-1

Planned 2026-11-10 (plan `forest-dumb-level-1`, T27, DESIGN §2.11). The planner needed no guide and
no stretch played by the owner.

## What it is

`engine/crates/fd-plan/tests/routes/W1-1.txt`: an input script, a joystick state a step, from the
level's start (`Start::Level1`, the level sync point) to the step `main_loop` finds the level's end
in. Inputs only, no disk data, so it is committed.

- 22 528 steps, about 7.5 minutes of play. The last step raises `Event::LevelEnd`.
- All 282 needed items taken (`items_left` 0), ending in the exit at `W1-1:208,92`.
- 32 lives throughout, never dying. Score `$59310`, the countdown at 642.
- Contact is met on 106 passes in 99 runs, 7 of them two passes long, never the three in a row
  that cost a life (`char_hit_check`, docs/10 "Contact damage"; measured as `hits_taken` after each
  step). The planner keeps no margin from contact.

## Planning it again

From `engine/`:

```
cargo run --release -p fd-plan -- level W1-1 --rounds 2100 --no-confirm --out ../build/routes/W1-1.txt
```

Seed 1, the walk compass. The exit comes in round 2 095 after 20 216 839 engine frames: 232 s on 10
threads of a loaded machine, 146 s in the test. Rounds end the search, so the route is the same on
any machine and thread count; `--budget-mins 30` finds the same route when the machine gets there in
time. Shorter searches stall: 100 rounds at `W1-1:66,90` with 188 items left, 400 rounds at
`W1-1:184,92` with 128. An engine step costs about 50 microseconds in release.

## How it is guarded

`engine/crates/fd-plan/tests/full_route.rs`:

- smoke: the route replayed on the engine alone reaches the level's end with no life lost on any
  step and no other event; it passes 16 milestones (a cell and the items left, every 1 500 steps)
  within a cell and 50 steps; the file reads back as the planner writes it.
- full: the planner finds the same route again in 2 100 rounds.

Any engine change that moves the character on the route breaks the smoke tests. Then plan again
with the command above, replace the route keeping its header, and set `FRAMES`, `ROUNDS` and
`MILESTONES` from the new run.

## On the original

`engine/conform/scripts/route-W1-1.txt`, played by `engine/conform/cases/route-W1-1.toml` (T48,
2026-11-10): 22 528 steps from the level sync point, the original running `next_world` in the last.
The engine agrees on all 21 764 passes compared: the character, the camera, the status variables,
`clxdat_copy` and every creature slot and flag. No life is lost, 32 lives on every pass; 763 pass
ends fall two to a step and are not seen. `fd-conform/tests/full_route.rs` holds it: the opening
600 steps in smoke with the whole picture, the whole route in full.

It is not the route above. The engine alone does not know which step each pass of the original
read or which pass cut a redraw, so T27's route ends elsewhere on the original, and this one loses
a life on the engine alone at step 1 743. The app plays the route above; the original this one.

It was planned from the original's states by
`plans/forest-dumb-level-1/prototype/t48-route-on-original/onoriginal.rs` (its README says how to
run it): T27's route fed a pass at a time, each step given the input of the pass that reads next,
the rest forecast on the engine alone from the original's state every 200 steps and wherever the
original left the forecast, and repaired by doubling or dropping one to three inputs. 28 passes
left the forecast, all by a creature started a pass apart; 6 repairs; 77 s. After an engine change
that moves the route, run it again from this route's inputs or from the route above.

Three differences stopped the first runs, each fixed where it lay:

- **A row cut before its shapes** (pass 4 976). A pass that interrupts a row or column redraw
  before `h0_7cc` has copied all its shapes probes `collision_map` as it was. The runner measures
  how many words were copied (`EdgesCut::shape_words`, from the PC and the loop counters) and the
  engine copies the rest after the pass. Twice in a row on one redraw is modelled and unmeasured.
- **The joystick read** (pass 12 294). A frame can end between `read_joystick`'s entry and its
  `move.w $dff00c.l, d0` (`h0+$8e1c`); the pass then reads the next step's joystick. `PassInputs`
  counts at the read. `input_log_index` stops at 12 000, so nothing checks the count after that.
- **The map's top two rows** (pass 13 941). `redraw_edge_b`'s shape row is the camera's row less 2
  as a word: `$ffff` or `$fffe` there, 64 KB past the shape map, in hunk 0's tables from
  `h0+$38eb7`. The shapes those bytes name land in the band of the row below the view.
  `LevelImage` reads the program outside the level's buffers.

## The picture on the route

Compared on all 22 152 frames with nothing set aside (T82, 2026-11-10). T48 had set 40 frames
aside in ten boxes, of three kinds as it read them. What each was, measured:

- **How frames in a row are counted.** The runner told the picture judge only of frames that
  differed, so glitch frames thousands of frames apart read as a run. It now tells it of every
  frame, as the toolkit counts (the coordinator agent for the owner, 2026-11-10). With nothing set aside 53 frames were past a
  few pixels; two stretches were three or more in a row.
- **The countdown's and the score's last digits** (frames 18 365 to 18 396). The original never
  ran its status bar redraw after pass 12 064: `main_loop` was still in its edge redraws when
  pass 12 065 came. The runner counts the redraw's calls and compares such a period with the bar
  as it stood (`engine/conform/README.md`, "A status bar redraw never reached"). Seven periods of
  the route are like it, from passes 576, 2 528, 4 448, 7 008, 7 456, 12 064 and 17 600; six
  differed by one countdown digit inside the few-pixel tolerance before and are exact now.
- **Two coins a phase apart** (frames 18 894 to 18 896, read by T48 as the view's first rows).
  Pass 12 569 cut `h0_6882` at `h0+$693e`, after its four counts: the original put the cell at x
  `$720` in the list chosen before the pass. The engine chose again after it, so the animated
  cell lists differed through passes 12 570 to 12 625, the only such stretch of the route. Fixed
  in the engine (`AddCut`).
- **The view's first rows** (frames 17 196, 17 907, 19 440, 19 442, 19 454, 19 456, 19 459,
  19 463). A row redraw's bobs land in a row whose first lines are in view. A pass that cuts the
  scan leaves the later bobs for after it: of ten, eight at beam lines 197 to 231 and two at lines
  103 to 108 of the next frame, so those two show a frame later. The engine drew all ten before
  the pass. Fixed in the engine: the bobs from the cut cell on start after the pass.
- **A taken item erased behind the beam**: left, inside the tolerance. `h0_29ca`'s blit comes
  mid-frame (at 15 813, beam lines 142 and 152 for a cell on lines 135 to 150), so the frame
  still shows the item, whole or its top rows; the engine erases the cell at once. Nine frames,
  never three in a row: 10 358 (105 pixels), 11 699 (166), 11 704 (147), 13 438 (197), 13 439
  (110), 15 761 (82), 15 809 (244), 15 813 (78), 20 693 (100). `full_route.rs` pins the count. A
  rule for it would follow T80's (the blit's rows the beam had passed); none is built.
