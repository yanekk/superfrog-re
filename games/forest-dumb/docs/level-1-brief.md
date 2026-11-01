# Brief for `pir plan`: level 1 of Forest Dumb Forever

Written 2026-11-01 for the planner. Slug: `forest-dumb-level-1`. The sequence around this plan is in
[ROADMAP.md](ROADMAP.md). Read that, `games/forest-dumb/CLAUDE.md`, `toolkit/PLAYBOOK.md` steps 6 to 9,
`plans/forest-dumb-start/REPORT.md` and its `FINDINGS.md` first.

## The goal

One plan ends with level 1 (`W1-1`) working: the owner opens the app, lands straight in the level, and can
play it from start to end, with its collectibles, enemies and special mechanics. A test plays it from
start to end. The owner decided against splitting this into several plans (2026-10-31), because nothing
less than a working level is tangible. Do not propose a split into plans; split into tasks.

## Stages, in order of work

| Stage | Delivers | Proven by |
|---|---|---|
| Character in the level | Level 1 headless: running, jumping, collision, scrolling, drawing | Same inputs on the original and the engine; movement and picture match on stretches with no enemies or collectibles |
| Playable app | Window and keyboard; walk around level 1 | A test drives the real app through a scripted run |
| Recorder | Save a recording of play or a bug, replay it | A saved recording replays to the same result |
| Map page | HTML page: level 1's map with the character's path | A test generates the page from a scripted run and checks its contents |
| Route planner | Finds a route through level 1, shown on the map page; based on Superfrog's | The route replays on the engine and reaches the expected point |
| Collectibles | Level 1 with collectibles, playable end to end | A test |
| Enemies | Level 1 with enemies, playable end to end | A test |
| The rest of the level | Any special mechanics, such as doors or secret passages | A test plays the level from start to end |

The planner orders the tasks and may run them side by side. The character work goes first: it is the
largest unknown, and everything else rests on it.

## Settled by the owner

- The app starts straight in level 1. The title screen and menus are a later plan.
- Silent. Sound is a later plan, after the level plans.
- Faithful to the original, judged by machine on the oracle, never by the owner playing or looking
  (`knowledge/self-testing-faithful-bar.md`). The cracked `[cr WT]` set is the measuring stick.
- Levels 2 to 6 are one plan each, later. Do not generalise for them beyond what level 1 needs.

## Reuse

Root CLAUDE.md, "Before building something", binds this plan. Every task doc names what it reuses or
states that nothing exists.

| Need | Start from |
|---|---|
| Porting routines against the original | `toolkit/crates/amiga-oracle/src/routine.rs`, `toolkit/crates/amiga-conform/`, `games/forest-dumb/engine/crates/fd-conform/`; pattern in `games/superfrog/engine/crates/sf-conform/tests/collision_routines.rs` |
| One level with only the character first | `plans/rust-engine/tasks/T14-frog-and-level.md`, `plans/rust-engine/PLAN.md` ("The small thing before the big thing") |
| Display, blitter, copper, timing | `toolkit/crates/amiga-chips/`, `amiga-clock` |
| App | `toolkit/crates/amiga-app/`, `amiga-input`; `games/superfrog/engine/crates/sf-app/` for how a game wires them |
| End-to-end driving | `toolkit/crates/amiga-e2e/`, `games/superfrog/engine/crates/sf-e2e/` |
| Recorder and its zips | `toolkit/crates/amiga-record/`, `plans/bug-recorder/DESIGN.md`, `games/superfrog/docs/BUG-FIX.md` |
| Route planner, map page | `games/superfrog/engine/crates/sf-plan/`, `sf-routeview/`, `plans/smart-planner/DESIGN.md`, `games/superfrog/docs/planner-stall-ask-owner.md` |
| Level files | `games/forest-dumb/tools/render_levels.py`, `docs/05-file-formats.md`, `docs/06-graphics-and-rendering.md`; the Rust readers land in this plan |

Superfrog's game logic is a pattern for how to port, never a source for how this game behaves.

## Known gaps to plan for

From `forest-dumb-start`; each needs a task or a named home.

- During a level the oracle's `Rgb12Frame` holds only the 320x56 status bar, because the crop uses the DIW
  at frame end and the game moves it mid-frame. Fix this before any picture comparison in play.
- The first case has no input after its start. Input timing mid-run is unproven; the first case that uses
  it needs its own test.
- The level renders have not been compared with the game's own screen. They were once drawn transposed
  and a review caught it. Compare them against the oracle early.
- The object maps (`_MapaBobow*`) are grouped, not understood. Collectibles and enemies depend on them.
- Play runs inside the copper interrupt (`level3_handler`, `game_frame`); "once a frame" is a reading from
  the code, not measured in play. Measure it before porting on top of it.
- Most of `docs/07-executable-map.md` is read from the code and not run. Treat names as readings.
- T00: fire at the title loads `SCENA 1`, fire again enters the level. That is the path to a sync point.
- Smoke has a 15-minute limit (root CLAUDE.md, Conventions). The whole-level play test may belong in the
  full set with a shorter smoke test beside it; settle it with the owner if the crucial tests do not fit.
- The zips stay in `games/forest-dumb/` in the main checkout.

## Open, for the planner to settle

Proposals only. Each is the owner's to decide in the planning session.

1. **What "correct" means before enemies and collectibles exist.** The original always has them. Proposed:
   compare movement, collision and scrolling on stretches where nothing else interferes.
2. **Copy or move.** Superfrog's route planner and map page are outside the toolkit. Proposed: move the shared
   parts into the toolkit, since every later level uses them, with Superfrog's checks staying green.
3. **Route planner before the mechanics.** If the exit needs a mechanic from the last stage, the route stops
   short of it until then. Proposed: the test is "reaches the furthest point possible", then the full run at
   the last stage.

What the owner will see, not yet specified anywhere. Ask rather than assume:

4. What happens when level 1 is finished: stop on a message, restart the level, or quit. The original
   moves on to world 2, which this plan does not have.
5. What happens on death and game over, with no title screen to return to. Proposed: lives behave as in
   the original, and game over restarts the level.
6. The status bar. Proposed: drawn as in the original from the first stage that compares pictures, since
   the comparison needs it.
7. Controls and quitting. Proposed: the same keys and rebinding as Superfrog's app, which read the owner's
   settings (`knowledge/owner-controls.md`).

## Out of scope

Levels 2 to 6, sound, the title screen and menus, the ending, the whole-game test, map editing.
