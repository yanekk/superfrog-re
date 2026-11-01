# Implementation plan

31 tasks in 7 phases. Each has a file in [tasks/](tasks/). Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

## Shape of the build

- Measure before porting. T00 replaces the readings the rest depends on, so a wrong reading costs one task.
- Everything is proven headless before a window opens: the character stage ends at T11, and T13 wires a window to it.
- Protect the first game before moving its code: T02 pins routes before T17 and T18 move anything.
- The planner comes before collectibles and enemies, so each later stage extends how far a route reaches, and T27 proves the finish.

## Main path, builder and wirer

| Step | Built by | Wired by |
|---|---|---|
| Files read from the disks | T30, T05 | T07's `GameData`, T13 calls it |
| App opens, disks found | T13 | T13 `fd-app/src/main.rs` |
| Enters level 1 | T07 | T13 constructs `Game` |
| Character, camera | T08, T09 | T08, T09 edit `Game::step` |
| Drawing, status bar | T10 | T13 presents it |
| Object scan | T22 | T22 calls it from `Game::step` |
| Collectibles, enemies, mechanics | T22, T24, T25 | Each registers in the object dispatch T07 defines; T22's scan calls it |
| Game over, level end | T23, T26 | `Scene`, and T23, T26 edit `fd-app/src/main.rs` for restart and message |
| Recorder | T14 | T14 `fd-app` menu |
| Controls | T12 | T13 menu |
| Route to the end | T20, T27 | T28 plays it in the app |

## Phase 0: prove the ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-measure-play.md) | measure-play | — |
| [T01](tasks/T01-smoke-room.md) | smoke-room | — |
| [T02](tasks/T02-route-pin.md) | route-pin | — |

T00 gates the sync point, pass matching in T06, how T10 draws objects, and the start seed. Its answers are written in `games/forest-dumb/docs/`. If play is not one pass a frame, T06 matches passes instead of frames. If the game uses hardware sprites, T10 takes Superfrog's composition pattern into the toolkit.

## Phase 1: comparing during play

| # | Task | Depends on |
|---|---|---|
| [T03](tasks/T03-level-sync.md) | level-sync | T00 |
| [T04](tasks/T04-picture-in-play.md) | picture-in-play | T00 |
| [T30](tasks/T30-unpack-in-rust.md) | unpack-in-rust | — |
| [T05](tasks/T05-level-readers.md) | level-readers | T03, T04, T30 |
| [T06](tasks/T06-level-runner.md) | level-runner | T03, T04 |

## Phase 2: the character, headless

| # | Task | Depends on |
|---|---|---|
| [T07](tasks/T07-level-entry.md) | level-entry | T05, T06 |
| [T08](tasks/T08-character-movement.md) | character-movement | T07 |
| [T09](tasks/T09-camera-scroll.md) | camera-scroll | T07 |
| [T10](tasks/T10-level-drawing.md) | level-drawing | T08, T09 |
| [T11](tasks/T11-character-cases.md) | character-cases | T01, T08, T09, T10 |

## Phase 3: app and recorder

| # | Task | Depends on |
|---|---|---|
| [T12](tasks/T12-app-toolkit-move.md) | app-toolkit-move | — |
| [T13](tasks/T13-forest-dumb-app.md) | forest-dumb-app | T01, T10, T12 |
| [T14](tasks/T14-recorder.md) | recorder | T12, T13 |
| [T15](tasks/T15-recording-compare.md) | recording-compare | T03, T14 |
| [T16](tasks/T16-app-drill.md) | app-drill | T13, T14 |

## Phase 4: map page and planner

| # | Task | Depends on |
|---|---|---|
| [T17](tasks/T17-planner-toolkit-move.md) | planner-toolkit-move | T02 |
| [T18](tasks/T18-map-page-toolkit-move.md) | map-page-toolkit-move | T17 |
| [T19](tasks/T19-map-page.md) | map-page | T11, T18 |
| [T20](tasks/T20-planner-level-1.md) | planner-level-1 | T11, T17, T19 |

## Phase 5: collectibles, lives, enemies

| # | Task | Depends on |
|---|---|---|
| [T21](tasks/T21-object-meanings.md) | object-meanings | T05, T06 |
| [T22](tasks/T22-collectibles.md) | collectibles | T11, T20, T21 |
| [T23](tasks/T23-lives-and-game-over.md) | lives-and-game-over | T11, T13, T14, T20 |
| [T24](tasks/T24-enemies.md) | enemies | T20, T21, T22, T23 |

## Phase 6: finishing the level

| # | Task | Depends on |
|---|---|---|
| [T25](tasks/T25-mechanics.md) | mechanics | T11, T20, T21, T22 |
| [T26](tasks/T26-level-end.md) | level-end | T11, T13, T14, T23 |
| [T27](tasks/T27-full-route.md) | full-route | T20, T22, T24, T25, T26 |
| [T28](tasks/T28-finish-end-to-end.md) | finish-end-to-end | T16, T27 |
| [T29](tasks/T29-level-drill.md) | level-drill | T15, T28 |

## Critical path

```
T00 → T03 → T05 → T07 → T08 → T10 → T11 → T19 → T20 → T22 → T24 → T27 → T28 → T29
```

T01, T02, T12, T30, T17 and T18 are off it and start early. Leaves: only T29, the final deliverable.

## Parallel width

31 tasks · longest dependency chain 14 · up to 5 could run at once. Computed from the table by a small script; `analyzeParallelism` is not in this repo.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T06, T08, T10, T12, T17, T24, T25, T27 |
| **Medium** | T30, T00, T03, T05, T07, T09, T11, T13, T14, T15, T18, T20, T21, T22, T23, T26, T28 |
| **Light** | T01, T02, T04, T16, T19, T29 |

T24, T25 and T27 are the likeliest overruns: what level 1 holds is unknown until T21, and the planner finished 9 of 24 levels unguided on the first game.

## Decisions still open

- How much T24 and T25 hold: settled by T21. More work than one session means asking the owner to add a task; this blocks nothing now.
- If T00 finds random numbers, the app's seed equals the sync point's, so each session starts alike. If that visibly repeats play, T00's report raises it with the owner.
- Whether the owner must play a stretch: settled by T27.
