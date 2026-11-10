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

Compared on all 22 152 frames. On 40 it is past the owner's tolerance as the runner counts it, and
the case sets those aside in ten boxes naming T82, each the smallest that holds one frame's
difference and each on its own frames (`"rows A..B cols C..D frames F..G"`). Three kinds:

- **A taken item erased behind the beam** (frames 15 813, 20 693). `h0_29ca`'s blit comes mid-frame
  (at 15 813, beam lines 142 and 152 for a cell on lines 135 to 150), so the frame still shows the
  item's top rows; the engine erases the cell whole.
- **The countdown's and the score's last digits** through one status period (frames 18 364 to
  18 395, passes `32k + 1` to `32k + 31` from 12 065). The original shows 809 and the old score
  digit, the engine 808 and the new, and no cut redraw the engine draws (`Game::cut_redraw`) matches.
- **The first rows of the view at the map's top** (frames 17 196, 18 896, 19 454, 19 456, 19 459,
  19 463). The engine shows an object or two oranges in the top row that the original does not show
  yet; once a sliver of a fruit differs.

The count is the runner's: it tells `PictureJudge` of a frame only when it is not `Same`, so "three
frames in a row" is three glitch frames since the last few-pixel frame, however far apart. Eight of
the 40 are single frames. Whether an identical frame should end a run is T82's first question.
