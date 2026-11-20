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
| Translating routines with Muse | `games/forest-dumb/spikes/muse-spark/` (judge, `run.py`, README with the measurements); see "Translation by Muse" below |
| One level with only the character first | `plans/rust-engine/tasks/T14-frog-and-level.md`, `plans/rust-engine/PLAN.md` ("The small thing before the big thing") |
| Display, blitter, copper, timing | `toolkit/crates/amiga-chips/`, `amiga-clock` |
| App | `toolkit/crates/amiga-app/`, `amiga-input`; `games/superfrog/engine/crates/sf-app/` for how a game wires them |
| End-to-end driving | `toolkit/crates/amiga-e2e/`, `games/superfrog/engine/crates/sf-e2e/` |
| Recorder and its zips | `toolkit/crates/amiga-record/`, `plans/bug-recorder/DESIGN.md`, `games/superfrog/docs/BUG-FIX.md` |
| Route planner, map page | `games/superfrog/engine/crates/sf-plan/`, `sf-routeview/`, `plans/smart-planner/DESIGN.md`, `games/superfrog/docs/planner-stall-ask-owner.md` |
| Level files | `games/forest-dumb/tools/render_levels.py`, `docs/05-file-formats.md`, `docs/06-graphics-and-rendering.md`; the Rust readers land in this plan |

Superfrog's game logic is a pattern for how to port, never a source for how this game behaves.

## Translation by Muse, and session size (decided 2026-11-05)

The owner decided this after the Muse spike (`spikes/muse-spark/README.md`, rounds 1-3), to keep Opus
tokens for what only Opus does. In `plans/rust-engine` 98.6-99.3% of each heavy task's tokens were cache
reads, builders took 86% of the run, and T14's first builder reached the 1M context limit after ~9 h and
$57 with auto-compact off. The cost is long sessions re-reading their context, so the plan does three
things.

1. **A translation service**, built early, before the character work needs it. A command takes a routine
   (`hN+$off`) and a generator, sends the listing (control-flow extraction, named labels) to
   `meta/muse-spark-1.3-contributor` on OpenRouter, judges each answer on the routine oracle (hardware
   mode, custom-chip writes in order), sends back the first failing case, and returns a passing port and
   a log line (tokens, cost, tries). The builder sees only that result, never the listing or the retries.
   - Start from the spike's judge and `run.py`. It lives in the game first (root CLAUDE.md, "Before
     building something"); the judge's `Mem` and hardware model are generic and move to the toolkit when
     a second game uses them.
   - Measured limits: Muse passed 24 of 24 runs on routines up to 183 instructions and 2 of 6 at 298-461,
     so a routine over ~180 instructions is split at its callees first. One that still fails after three
     tries is translated by the builder.
   - `max_tokens` 96 000 (32 000 cut most answers off mid-reasoning). The key is `OPENROUTER_API_KEY`
     from the environment or `~/.zshrc`. A call costs under a cent; it needs a `DESIGN.md` §5.3 row.
     Sending the game's code to Meta through the contributor model was accepted on 2026-11-04.
   - The generator is the builder's work and the reviewer's check: a pass is only as strong as the states
     it ran on. The judge reports how many paths the original took; a reviewer looks for branches no state
     reaches.
   - Raw ports and listings derive from the game's code and stay in `build/`. What lands in `fd-core` is
     the engine-shaped port, as with Superfrog.
2. **A batch translation before the stages build on it.** Run the service over `game_frame`'s call tree
   (about 8 300 instructions with callees by the spike's scan) in one unattended run, a night run if that
   suits. It writes a table: routine, pass, fail or too big, port path, generator, cost. Failures become
   the routines a builder translates. Generators for the batch are written in batches by sessions sized
   for it and reviewed. The model calls should cost a few dollars at most.
3. **Small builder sessions.** Size tasks so a builder stays well below the context limit: one
   subsystem's routines, not a whole stage. Give each frame-matching effort its own task, split by case,
   since T14's builder died in that loop. Auto-compact stays on. Keep the run's conversation logs
   (`plans/forest-dumb-level-1/.parallel/control/conversations/`) somewhere cleanup does not reach, so the
   tokens spent per activity can be measured after the run. The rust-engine logs were deleted, and that
   breakdown is missing.

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
