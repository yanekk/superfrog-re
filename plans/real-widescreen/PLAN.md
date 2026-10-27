# Implementation plan

13 tasks in 4 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
interfaces it defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- **The capacity unknown is measured first.** T00 widens spawning crudely on a throwaway copy and
  replays every level's finished run, so T03 sizes the extra slots from numbers, not a guess.
- **The game rules before the picture.** Phase 1 makes widescreen a game mode and proves every edge
  rule headless; phase 2 then draws what the game already keeps alive. A strip drawn before the rules
  exist would have nothing in it to test.
- **4:3 is guarded on every task.** Every rule is a branch on the mode, and every task touching
  `sf-core` level code runs the whole workspace once, because the conformance cases that pin the
  original live outside the test block.
- **The every-level proof is last**, after the drill has removed what a person would notice, and after
  smart-planner is merged (DESIGN §5, D5).

```
Phase 0  ▸  T00                       measure peak live objects     throwaway
Phase 1  ▸  T01 … T07                 widescreen as a game mode     headless
Phase 2  ▸  T08 … T11                 the strips drawn, app, drill
Phase 3  ▸  T12                       every level finished wide     after smart-planner
```

---

## Phase 0 — Prove the ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-wide-spawn-spike.md) | wide-spawn-spike | — |

**T00 gates T03's `WIDE_SLOTS`** and lists the levels whose objects need the most room or crash when
woken early. If the peak stays at 20 or below everywhere, T03 shrinks to a guard; if it exceeds
about 60, the owner hears it before T03 starts. Throwaway: its code is deleted, its numbers stay in
FINDINGS.

## Phase 1 — Widescreen as a game mode

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-wide-mode.md) | wide-mode | — |
| [T02](tasks/T02-wide-spawn.md) | wide-spawn | T01 |
| [T03](tasks/T03-wide-capacity.md) | wide-capacity | T00, T01, T02 |
| [T04](tasks/T04-wide-camera.md) | wide-camera | T01 |
| [T05](tasks/T05-wide-special-cases.md) | wide-special-cases | T01 |
| [T06](tasks/T06-wide-switch.md) | wide-switch | T02, T03 |
| [T07](tasks/T07-wide-recording.md) | wide-recording | T01 |

At the end: headless, a widescreen game spawns, frees, holds and clamps by DESIGN §2.2–§2.7, switches
mid-level, and records and replays; 4:3 is unchanged.

## Phase 2 — The strips drawn, the app, the drill

| # | Task | Depends on |
|---|---|---|
| [T08](tasks/T08-side-objects.md) | side-objects | T02, T03 |
| [T09](tasks/T09-side-frog.md) | side-frog | T04, T08 |
| [T10](tasks/T10-app-wiring.md) | app-wiring | T05, T06, T07, T08, T09 |
| [T11](tasks/T11-widescreen-drill.md) | widescreen-drill | T10 |

At the end: the app plays the widescreen game, the strips show everything alive, the README and
docs/12 say so, and a worker has driven it.

## Phase 3 — Every level finished

| # | Task | Depends on |
|---|---|---|
| [T12](tasks/T12-wide-every-level.md) | wide-every-level | T11 |

T12 also needs `sf-plan` on its branch. pir brings main in only after every task is done, so once
smart-planner is merged into main the owner stops the run, main is merged into `pir/real-widescreen` by
hand, and the run restarts (DESIGN §5, D5); until then T12's worker asks and waits.

---

## Critical path

```
T01 → T02 → T03 → T08 → T09 → T10 → T11 → T12
T00 → T03
```

T03 waits for T02 because its replay and eviction tests need the widened spawn and despawn. T09 waits
for T08 because the frog in the strip needs T08's composition of sprites over the strips. T04, T05, T06
and T07 are off the path.

Leaves: T12 only.

## Wiring

Each step of the main path and the task that plugs it in: the mode reaches the core through
`Game::set_widescreen` (T01) and the app's menu toggle (T10, `sf-app/src/main.rs` `toggle_widescreen`
and every `set_view` call site in `menu.rs`/`levelfile.rs`); the `Screen` type is consulted at each site
by T02–T05; the switch-on scan runs inside the level step (T06); the draw list is filled by `bob_draw`
(T08) and read by the renderer (T08), which `Game::render` already calls; recordings are written by the
app's recorder (T10) in T07's format and read by `sf-conform bug` (T07).

## Parallel width

13 tasks · longest dependency chain 8 · up to 5 could run at once (T00, T02, T04, T05, T07 after T01).

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T03, T08, T12 |
| **Medium** | T00, T02, T04, T05, T07, T10, T11 |
| **Light** | T01, T06, T09 |

T03 will overrun if a slot loop hides in a place the survey missed (the six literal `(0..20)` loops and
`PASS_WRITES` are known). T12 will overrun on levels that stalled the planner before (W1-4, W2-4, W6-2,
W6-3), now with more enemies alive.

## Decisions still open

- `WIDE_SLOTS`: settled by T00's numbers. Does not block anything before T03.
- Whether the final level's widescreen clamp moves its camera off the start point (DESIGN §2.5): T04
  measures and records it; no decision needed unless it breaks the witch fight, which goes to the owner.
- Whether smart-planner merges before T12 comes up: if not, T12 waits for the owner's hand-merge (D5).
