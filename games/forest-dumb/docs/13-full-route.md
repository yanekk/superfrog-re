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

Not established here: that is T48's. Played once through `fd_plan::confirm` (2026-11-10) the
original agreed on all 13 459 passes compared and ended elsewhere than the engine alone
(`Outcome::GoalMissed`), as `fd-plan/README.md`'s gotcha describes: the engine alone does not know
which step each pass read or which pass cut a redraw, so a creature is met a pass apart. A route
that holds on the original has to be planned from the original's states
(`plans/forest-dumb-level-1/prototype/t79-route-repair/goto.rs`).
