# Implementation plan

50 tasks in 7 phases and a translation track. Each has a file in [tasks/](tasks/). Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

## Shape of the build

- Measure before porting. T00 replaces the readings the rest depends on, so a wrong reading costs one task.
- Translate before porting (DESIGN §4.3). Muse writes literal ports through a judged service (T33); one batch (T40) covers `game_frame`'s call tree before any porting task starts. Porting tasks turn passing ports into engine code and spend Opus tokens only on what Muse failed.
- Small sessions (DESIGN §4.3). A porting task holds one subsystem's routines and proves them on the rig. Whole-run agreement with the original is a separate task per group of cases (T41 to T48), because the first game's character builder died in that loop.
- Everything is proven headless before a window opens: the character stage ends at T11, and T13 wires a window to it.
- Prove a port ran, not only that it matched: T32's rig and port register and T06's coverage report, on T31's counter, are in place before the first routine is ported (DESIGN §4.1).
- Protect the first game before moving its code: T02 pins routes before T17 and T18 move anything.
- The planner comes before collectibles and enemies, so each later stage extends how far a route reaches; T27 finds the full route and T48 proves it on the original.

## Main path, builder and wirer

| Step | Built by | Wired by |
|---|---|---|
| Files read from the disks | T30, T05 | T07's `GameData`, T13 calls it |
| Literal ports of `game_frame`'s calls | T33, T35 to T39, T40; object routines T49 | Porting tasks read T40's table |
| App opens, disks found | T13 | T13 `fd-app/src/main.rs` |
| Enters level 1 | T07 | T13 constructs `Game` |
| Character, camera | T08, T09 | T08, T09 edit `Game::step` |
| Drawing, status bar | T10 | T13 presents it |
| Object scan | T22 | T22 calls it from `Game::step` |
| Collectibles, enemies, mechanics | T22, T24, T25 | Each registers in the object dispatch T07 defines; T22's scan calls it |
| Game over, level end | T23, T26 | `Scene`, and T23, T26 edit `fd-app/src/main.rs` for restart and message |
| Recorder | T14 | T14 `fd-app` menu |
| Controls | T12 | T13 menu |
| Route to the end | T20, T27, T48 | T28 plays it in the app |
| Rig, port register, coverage | T32, T06 on T31's counter | T07's dispatch raises `Unported`; T41 to T48 and T11 check coverage |

## Phase 0: prove the ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-measure-play.md) | measure-play | — |
| [T01](tasks/T01-smoke-room.md) | smoke-room | — |
| [T02](tasks/T02-route-pin.md) | route-pin | — |
| [T31](tasks/T31-code-ran-counter.md) | code-ran-counter | — |
| [T32](tasks/T32-routine-rig.md) | routine-rig | — |

T00 gates the sync point, pass matching in T06, how T10 draws objects, and the start seed. Its answers are written in `games/forest-dumb/docs/`. If play is not one pass a frame, T06 matches passes instead of frames. If the game uses hardware sprites, T10 takes Superfrog's composition pattern into the toolkit.

## Phase 1: comparing during play

| # | Task | Depends on |
|---|---|---|
| [T03](tasks/T03-level-sync.md) | level-sync | T00 |
| [T04](tasks/T04-picture-in-play.md) | picture-in-play | T00 |
| [T30](tasks/T30-unpack-in-rust.md) | unpack-in-rust | — |
| [T05](tasks/T05-level-readers.md) | level-readers | T03, T04, T30 |
| [T06](tasks/T06-level-runner.md) | level-runner | T03, T04, T31, T32 |

## Phase 1, translation track

Runs beside the rest of phase 1. Every porting task waits for T40.

| # | Task | Depends on |
|---|---|---|
| [T33](tasks/T33-translate-service.md) | translate-service | T32 |
| [T34](tasks/T34-translate-inventory.md) | translate-inventory | T00 |
| [T35](tasks/T35-gen-character.md) | gen-character | T33, T34 |
| [T36](tasks/T36-gen-camera.md) | gen-camera | T33, T34 |
| [T37](tasks/T37-gen-drawing.md) | gen-drawing | T33, T34 |
| [T38](tasks/T38-gen-objects.md) | gen-objects | T33, T34 |
| [T39](tasks/T39-gen-rest.md) | gen-rest | T33, T34 |
| [T53](tasks/T53-gen-character-2.md) | gen-character-2 | T33, T34; blocks T40 |
| [T54](tasks/T54-gen-objects-2.md) | gen-objects-2 | T33, T34; blocks T40 |
| [T56](tasks/T56-gen-edge-redraw.md) | gen-edge-redraw | T33, T34; blocks T40 |
| [T58](tasks/T58-translate-btst-fix.md) | translate-btst-fix | T33; blocks T40 |
| [T40](tasks/T40-batch-translate.md) | batch-translate | T35, T36, T37, T38, T39 |

T34 assigns every routine a generator batch and a porting task, and asks the owner to add sibling tasks where a share passes DESIGN §4.3's size. T40's run is a `worker` row of DESIGN §5.3, capped at $5.

## Phase 2: the character, headless

| # | Task | Depends on |
|---|---|---|
| [T07](tasks/T07-level-entry.md) | level-entry | T05, T06, T32 |
| [T08](tasks/T08-character-movement.md) | character-movement | T07, T40 |
| [T51](tasks/T51-character-anim-input.md) | character-anim-input | T07, T40; blocks T08 |
| [T09](tasks/T09-camera-scroll.md) | camera-scroll | T07, T40 |
| [T55](tasks/T55-edge-redraw.md) | edge-redraw | T09; blocks T10, T41, T42, T43 |
| [T10](tasks/T10-level-drawing.md) | level-drawing | T08, T09, T40 |
| [T41](tasks/T41-case-run-jump.md) | case-run-jump | T08, T09 |
| [T42](tasks/T42-case-walls-pits.md) | case-walls-pits | T08, T09, T41 |
| [T43](tasks/T43-case-scroll.md) | case-scroll | T08, T09 |
| [T44](tasks/T44-case-picture.md) | case-picture | T10 |
| [T11](tasks/T11-character-cases.md) | character-cases | T01, T41, T42, T43, T44 |

## Phase 3: app and recorder

| # | Task | Depends on |
|---|---|---|
| [T12](tasks/T12-app-toolkit-move.md) | app-toolkit-move | — |
| [T13](tasks/T13-forest-dumb-app.md) | forest-dumb-app | T01, T10, T12 |
| [T14](tasks/T14-recorder.md) | recorder | T12, T13 |
| [T15](tasks/T15-recording-compare.md) | recording-compare | T03, T14 |
| [T16](tasks/T16-app-drill.md) | app-drill | T13, T14 |
| [T50](tasks/T50-sf-menus-os-test-stale.md) | sf-menus-os-test-stale | — |
| [T57](tasks/T57-sf-scale-fullscreen-os-test.md) | sf-scale-fullscreen-os-test | — |

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
| [T21](tasks/T21-object-meanings.md) | object-meanings | T05, T06, T32 |
| [T49](tasks/T49-translate-objects.md) | translate-objects | T21, T38, T40 |
| [T22](tasks/T22-collectibles.md) | collectibles | T11, T20, T21, T40, T49 |
| [T23](tasks/T23-lives-and-game-over.md) | lives-and-game-over | T11, T13, T14, T20 |
| [T24](tasks/T24-enemies.md) | enemies | T20, T21, T22, T23 |
| [T45](tasks/T45-case-collect.md) | case-collect | T22 |
| [T46](tasks/T46-case-enemies.md) | case-enemies | T24, T45 |
| [T52](tasks/T52-creature-drawing.md) | creature-drawing | T10, T22; blocks T46 |

## Phase 6: finishing the level

| # | Task | Depends on |
|---|---|---|
| [T25](tasks/T25-mechanics.md) | mechanics | T11, T20, T21, T22 |
| [T26](tasks/T26-level-end.md) | level-end | T11, T13, T14, T23 |
| [T47](tasks/T47-case-mechanics.md) | case-mechanics | T25, T45 |
| [T27](tasks/T27-full-route.md) | full-route | T20, T26, T45, T46, T47 |
| [T48](tasks/T48-route-on-original.md) | route-on-original | T27 |
| [T28](tasks/T28-finish-end-to-end.md) | finish-end-to-end | T16, T48 |
| [T29](tasks/T29-level-drill.md) | level-drill | T15, T28 |

## Critical path

```
T00 → T03 → T05 → T07 → T08 → T10 → T44 → T11 → T19 → T20 → T22 → T24 → T46 → T27 → T48 → T28 → T29
```

The translation track (T32, T33, T34 to T40) runs beside T03 to T07 and joins at T08; if the batch is slower than T07, it is on the critical path. T01, T02, T31, T32, T12, T30 and T17 are off it and start early. T31 changes the oracle's build hash, so every sync cache rebuilds once after it merges. Leaves: only T29, the final deliverable.

## Parallel width

50 tasks · longest dependency chain 17 · up to 8 could run at once. Computed from the table by a small script; `analyzeParallelism` is not in this repo.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T12, T17 |
| **Medium** | T00, T03, T05, T06, T07, T08, T09, T10, T13, T14, T15, T18, T20, T21, T22, T23, T24, T25, T26, T27, T28, T30, T31, T32, T33, T34, T35 to T39, T41 to T49 |
| **Light** | T01, T02, T04, T11, T16, T19, T29, T40 |

The two heavy tasks are moves of the first game's code, checked by its existing tests, with no frame-matching loop. Porting tasks are sized by T34 and T21 against DESIGN §4.3; one over it gets sibling tasks before it starts. A frame-matching task whose cases need more than one session asks for a task for the rest, split by case.

## Decisions still open

- How much T24 and T25 hold: settled by T21 and T34. A share over DESIGN §4.3's size means asking the owner to add a task; this blocks nothing now.
- If T00 finds random numbers, the app's seed equals the sync point's, so each session starts alike. If that visibly repeats play, T00's report raises it with the owner.
- Whether the owner must play a stretch: settled by T27.
