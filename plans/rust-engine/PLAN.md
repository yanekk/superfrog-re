# Implementation plan

31 tasks in 4 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
interfaces it defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- **The reference comes first.** Every mechanic is judged by comparison with the original running in
  vAmiga (DESIGN §2.1), so the spike proving that comparison is possible (T00) and the permanent
  harness (T05, T06) come before any game code. Without them "frame-exact" is a claim nobody can check.
- **Everything is proven headless before anything opens a window.** Phases 1 and 2 build and verify
  the whole game as pure functions against the oracle; Phase 3 wires a window, sound and menus onto a
  core already known to match the original.
- **Understand before porting.** T07 maps the executables once, so the mechanic tasks start from a
  shared map instead of each rediscovering the main loop. T07 may re-split the mechanic tasks; that is
  an amendment to this plan, made with the owner.
- **The small thing before the big thing.** One level with only the frog (T14) before items, enemies
  and bosses; scripted runs from transplanted sync points before a whole game from power-on (T21).

```
Phase 0  ▸  T00                       prove the oracle            throwaway
Phase 1  ▸  T01 … T11                 foundations, headless       no window
Phase 2  ▸  T12 … T21                 the game, frame-exact       no window
Phase 3  ▸  T22 … T30                 the app                     window, menus, pad
```

---

## Phase 0 — Prove the ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-oracle-spike.md) | oracle-spike | — |

**T00 gates the whole verification design** (DESIGN §2.1, §3.2 `sf-oracle`, §7). If vAmiga boots the
disks headless, deterministically, with per-frame input and readable memory, frame and Paula state,
T05 builds on it. If it boots but one capability is missing, T05 adds it to the shim (the core is
ours to patch, MPL-2.0). If it cannot boot or is not deterministic, the plan stops and the owner
chooses between another emulator core and a weaker definition of exactness.

## Phase 1 — Foundations, headless

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-rust-workspace.md) | rust-workspace | — |
| [T02](tasks/T02-re-workbench.md) | re-workbench | — |
| [T03](tasks/T03-disk-import.md) | disk-import | T01 |
| [T04](tasks/T04-atn-decompressor.md) | atn-decompressor | T03 |
| [T05](tasks/T05-oracle-harness.md) | oracle-harness | T00, T01 |
| [T06](tasks/T06-routine-oracle.md) | routine-oracle | T04, T05 |
| [T07](tasks/T07-exe-map.md) | exe-map | T02, T04, T05 |
| [T08](tasks/T08-asset-decode.md) | asset-decode | T04, T07 |
| [T09](tasks/T09-asset-cache.md) | asset-cache | T03, T08 |
| [T10](tasks/T10-paula-mixer.md) | paula-mixer | T01, T05 |
| [T11](tasks/T11-audio-driver.md) | audio-driver | T06, T07 |

At the end: the disks import into a verified cache in Rust, every asset decodes, the original runs
under scripted control in the oracle, and the code map tells every later task where to look.

## Phase 2 — The game, frame-exact

| # | Task | Depends on |
|---|---|---|
| [T12](tasks/T12-core-state.md) | core-state | T05, T06, T07 |
| [T13](tasks/T13-renderer.md) | renderer | T08, T12 |
| [T14](tasks/T14-frog-and-level.md) | frog-and-level | T06, T11, T12, T13 |
| [T15](tasks/T15-items-and-scoring.md) | items-and-scoring | T14 |
| [T16](tasks/T16-enemies-worlds-1-3.md) | enemies-worlds-1-3 | T15 |
| [T17](tasks/T17-enemies-worlds-4-6.md) | enemies-worlds-4-6 | T15 |
| [T18](tasks/T18-bosses.md) | bosses | T16, T17 |
| [T19](tasks/T19-slot-and-bonus.md) | slot-and-bonus | T15 |
| [T20](tasks/T20-front-end.md) | front-end | T08, T11, T12, T13 |
| [T21](tasks/T21-game-flow.md) | game-flow | T18, T19, T20 |

At the end: a scripted game from power-on to the end sequence matches the original frame for frame,
headless.

## Phase 3 — The app

| # | Task | Depends on |
|---|---|---|
| [T22](tasks/T22-app-rig.md) | app-rig | T09, T10, T12, T13 |
| [T23](tasks/T23-app-shell.md) | app-shell | T21, T22 |
| [T24](tasks/T24-macos-menus.md) | macos-menus | T23 |
| [T25](tasks/T25-controls-and-gamepad.md) | controls-and-gamepad | T24 |
| [T26](tasks/T26-save-states.md) | save-states | T24 |
| [T27](tasks/T27-crt-and-widescreen.md) | crt-and-widescreen | T24 |
| [T28](tasks/T28-level-override.md) | level-override | T14, T24 |
| [T29](tasks/T29-app-drill.md) | app-drill | T21, T25, T26, T27, T28 |
| [T30](tasks/T30-acceptance.md) | acceptance | T29 |
| [T31](tasks/T31-render-remaining-levels.md) | render-remaining-levels | T13 |
| [T32](tasks/T32-story-intro.md) | story-intro | T20; blocks T21 |
| [T33](tasks/T33-attract-demo.md) | attract-demo | T16, T20 |
| [T34](tasks/T34-game-over.md) | game-over | T15, T20; blocks T21 |
| [T35](tasks/T35-end-sequence.md) | end-sequence | T20; blocks T21 |
| [T36](tasks/T36-items-on-the-beam.md) | items-on-the-beam | T15 |
| [T37](tasks/T37-level-restart.md) | level-restart | T36; blocks T34 |
| [T38](tasks/T38-level-tally.md) | level-tally | T37; blocks T21 |
| [T39](tasks/T39-enemies-world-5.md) | enemies-world-5 | T16, T38; blocks T18 |
| [T40](tasks/T40-enemies-world-6.md) | enemies-world-6 | T38; blocks T18 |
| [T41](tasks/T41-enemies-world-4.md) | enemies-world-4 | T38; blocks T18 |
| [T42](tasks/T42-enemies-world-1.md) | enemies-world-1 | T38; blocks T18, T33 |
| [T43](tasks/T43-enemies-world-2.md) | enemies-world-2 | T38; blocks T18, T39 |
| [T44](tasks/T44-enemies-world-3.md) | enemies-world-3 | T38; blocks T18 |
| [T46](tasks/T46-story-intro-scenes.md) | story-intro-scenes | T32; blocks T21 |
| [T47](tasks/T47-story-intro-handover.md) | story-intro-handover | T46; blocks T21 |
| [T48](tasks/T48-story-intro-skips.md) | story-intro-skips | T46, T47; blocks T21 |
| [T49](tasks/T49-story-fade-setup-job.md) | story-fade-setup-job | T46; blocks T56 |
| [T50](tasks/T50-story-objects-init-display.md) | story-objects-init-display | T49; blocks T56 |
| [T51](tasks/T51-story-oracle-blit-tooling.md) | story-oracle-blit-tooling | T46; blocks T52 |
| [T52](tasks/T52-story-blitter-bus-sim.md) | story-blitter-bus-sim | T51; blocks T53 |
| [T53](tasks/T53-story-draw-cpu-program.md) | story-draw-cpu-program | T52; blocks T54 |
| [T54](tasks/T54-story-handler-end-feed.md) | story-handler-end-feed | T53; blocks T56 |
| [T55](tasks/T55-story-intro-sound-timing.md) | story-intro-sound-timing | T50; blocks T56 |
| [T56](tasks/T56-story-intro-to-insert-disk.md) | story-intro-to-insert-disk | T49, T50, T54, T55; blocks T21, T47, T48 |
| [T70](tasks/T70-bonus-level.md) | bonus-level | T15; blocks T21 |
| [T71](tasks/T71-slot-sync.md) | slot-sync | T15 |
| [T72](tasks/T72-slot-screen.md) | slot-screen | T71 |
| [T73](tasks/T73-slot-rounds.md) | slot-rounds | T72 |
| [T74](tasks/T74-slot-prizes.md) | slot-prizes | T73; blocks T21 |
| [T75](tasks/T75-bonus-shots.md) | bonus-shots | T70; blocks T21 |
| [T76](tasks/T76-bonus-enemies-a.md) | bonus-enemies-a | T70; blocks T21 |
| [T77](tasks/T77-bonus-enemies-b.md) | bonus-enemies-b | T70, T76; blocks T21 |
| [T78](tasks/T78-bonus-enemy-death.md) | bonus-enemy-death | T70, T75; blocks T21 |
| [T79](tasks/T79-bonus-level-frames.md) | bonus-level-frames | T70, T37, T75, T76, T77, T78; blocks T21 |
| [T81](tasks/T81-beam-cleanup.md) | beam-cleanup | T36; blocks T37 |
| [T82](tasks/T82-beam-tests.md) | beam-tests | T81 |
| [T83](tasks/T83-items-w1.md) | items-w1 | T81 |
| [T84](tasks/T84-items-w3.md) | items-w3 | T81 |
| [T85](tasks/T85-items-w2.md) | items-w2 | T81 |
| [T86](tasks/T86-items-launchers.md) | items-launchers | T81 |
| [T87](tasks/T87-slot-symbols-routines.md) | slot-symbols-routines | T71; blocks T72 |
| [T88](tasks/T88-slot-symbols-tables.md) | slot-symbols-tables | T71; blocks T72 |
| [T89](tasks/T89-story-copper-bus-cycles.md) | story-copper-bus-cycles | T52; blocks T53 |
| [T90](tasks/T90-story-draw-blits-oracle.md) | story-draw-blits-oracle | T89; blocks T53 |
| [T91](tasks/T91-slot-beam-races.md) | slot-beam-races | T72 |
| [T92](tasks/T92-slot-load.md) | slot-load | T72 |
| [T93](tasks/T93-slot-attract-draw.md) | slot-attract-draw | T72, T91, T92 |
| [T94](tasks/T94-slot-interrupt-timing.md) | slot-interrupt-timing | T72, T91, T93; blocks T73 |
| [T95](tasks/T95-slot-load-finish.md) | slot-load-finish | T92; blocks T93 |
| [T96](tasks/T96-story-audio-dma-slots.md) | story-audio-dma-slots | T53; blocks T56 |
| [T97](tasks/T97-story-split-handler.md) | story-split-handler | T54; blocks T56 |
| [T100](tasks/T100-front-vbl-timing.md) | front-vbl-timing | T37; blocks T21 |
| [T101](tasks/T101-lag-long-pass.md) | lag-long-pass | T81, T82; blocks T21 |
| [T102](tasks/T102-items-w3-faults.md) | items-w3-faults | T101; blocks T21 |
| [T103](tasks/T103-w4-completions.md) | w4-completions | T41, T101, T102; blocks T18 |
| [T105](tasks/T105-w2-1-hang-fruit-life.md) | w2-1-hang-fruit-life | T85; blocks T21 |
| [T106](tasks/T106-w1-completions.md) | w1-completions | T42, T86, T101; blocks T18 |
| [T107](tasks/T107-enemies-w1-2-case.md) | enemies-w1-2-case | T42; blocks T18, T33, T106 |
| [T108](tasks/T108-enemies-w1-3-case.md) | enemies-w1-3-case | T42; blocks T18, T33, T106 |
| [T109](tasks/T109-enemies-w1-4-case.md) | enemies-w1-4-case | T42; blocks T18, T33, T106 |
| [T110](tasks/T110-enemies-w1-activation.md) | enemies-w1-activation | T42; blocks T18, T33, T106 |
| [T111](tasks/T111-enemies-w3-2-case.md) | enemies-w3-2-case | T44; blocks T18 |
| [T112](tasks/T112-enemies-w3-3-case.md) | enemies-w3-3-case | T44; blocks T18 |
| [T113](tasks/T113-enemies-w3-4-case.md) | enemies-w3-4-case | T44, T102; blocks T18 |
| [T114](tasks/T114-enemies-w3-activation.md) | enemies-w3-activation | T44; blocks T18 |
| [T115](tasks/T115-w3-completions.md) | w3-completions | T44, T101, T102, T111, T112, T113; blocks T18 |
| [T121](tasks/T121-bob-draw-cut.md) | bob-draw-cut | T101; blocks T21 |
| [T122](tasks/T122-w2-completions.md) | w2-completions | T43, T101, T121; blocks T18 |
| [T123](tasks/T123-enemies-w5-2-case.md) | enemies-w5-2-case | T39; blocks T18 |
| [T124](tasks/T124-enemies-w5-3-case.md) | enemies-w5-3-case | T39; blocks T18 |
| [T125](tasks/T125-enemies-w5-4-case.md) | enemies-w5-4-case | T39; blocks T18 |
| [T126](tasks/T126-w5-completions.md) | w5-completions | T39, T101, T121, T123, T124, T125; blocks T18 |
| [T127](tasks/T127-restore-sprite-wait.md) | restore-sprite-wait | T101; blocks T21 |
| [T128](tasks/T128-w3-4-sprite-race.md) | w3-4-sprite-race | T127; blocks T113 |
| [T129](tasks/T129-slot-lamp-cut.md) | slot-lamp-cut | T74; blocks T21 |
| [T130](tasks/T130-front-vbl-318.md) | front-vbl-318 | T100; blocks T21 |
| [T133](tasks/T133-w2-1-fruit-room.md) | w2-1-fruit-room | T105; blocks T21 |
| [T120](tasks/T120-conform-no-silent-pass.md) | conform-no-silent-pass | T05, T36; blocks T21 |
| [T104](tasks/T104-w6-completions.md) | w6-completions | T40, T101, T102; blocks T18 |

T22 is the rig task: it comes before every surface task and they depend on it. T29 is the phase's
drill. At the end: the owner plays it.

---

## The main path, builder and wirer

| Step | Built by | Wired by |
|---|---|---|
| Launch `superfrog` | T22 (`sf-app/src/main.rs`) | T22 |
| Find or ask for disks, import, verify cache | T03, T04, T08, T09 | T23 (import flow in `sf-app/src/import.rs`, called from `main.rs`) |
| Power-on → intro → title → options/passwords | T20 | T21 (`sf-core/src/flow.rs`), run by the game loop T22/T23 |
| Play a level: frog, items, enemies, bosses | T14–T18 | T14 (`sf-core/src/level/mod.rs` object dispatch; T15–T18 register into it) |
| Slot machine and bonus levels between levels | T19 | T21 |
| Picture to the window | T13 | T22 (`sf-app/src/video.rs`) |
| Sound to the speakers | T11, T10 | T22 (`sf-app/src/audio.rs`) |
| Keyboard / pad to the game | T23, T25 | T23 (`sf-app/src/input.rs`), T25 adds the pad |
| Menus, settings, save states, CRT, widescreen, level files | T24–T28 | T24 (`sf-app/src/menu.rs`); each feature task adds its items there |
| Progress remembered, passwords offered | T21 (`Progress` in core), T23 (file) | T23 |

## Critical path

```
T01 → T03 → T04 → T07 → T12 → T13 → T14 → T15 → T16 → T18 → T21 → T23 → T24 → T25 → T29 → T30
```

T00 and T05 run alongside T01–T04. T02 can start at once. T08–T11 run beside T12. T19 and T20 run
beside the enemy tasks, and T22 beside Phase 2's mechanics. T23 onward waits for T21, because
T23–T28 test the real game (the frog in W1-1, the title, progress, saving mid-level, widescreen on
the slot screen); owner's decision 2026-10-10 to keep those checks in the tasks rather than move them
to the drill.

Leaves (tasks nothing depends on): only T30.

## Parallel width

31 tasks · longest dependency chain 16 · up to 5 could run at once (for example T16, T17, T19, T20,
T22). Every edge is a real use: a task depends on what its code calls or its tests
compare against.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T05, T07, T13, T14, T16, T17, T18, T20 |
| **Medium** | T00, T04, T06, T08, T11, T12, T15, T19, T21, T22, T23, T29, T30 |
| **Light** | T01, T02, T03, T09, T10, T24, T25, T26, T27, T28 |

Where it will overrun: T07 (330 KB of 68000 code plus four other executables; it is a map, not a full
understanding, and must stop at that), and the enemy and boss tasks, whose size is unknown until T07
counts the object types. T13 risks overrunning on copper effects (per-line palette changes) and on how
the original composes bobs and hardware sprites.

## Decisions still open

- **Lag and timing-dependent randomness** (DESIGN §2.1). T07 measures whether the original ever takes
  more than one frame per main-loop pass and whether its random seed depends on CPU timing. If either
  is true, the owner decides how frame-exact is defined there; this blocks T12's final layout of
  `Game::step`, not T00–T11.
- **The real task split for enemies and bosses.** T07 reports the object types per world; if
  T16–T18 are each more than one session, this plan is amended with the owner before they start.
- **Which Kickstart boots the crack** (T00). Does not block design.
- **Ghidra or a capstone workbench** (T02). Does not block design.
