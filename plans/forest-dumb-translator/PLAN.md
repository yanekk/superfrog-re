# Implementation plan

43 tasks in 8 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the interfaces it defines, and what done means. Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- Instructions are proven one kind at a time against Moira before anything whole runs, because a wrong flag in a rare instruction is cheap to find alone and costly to find inside a frame.
- The translator becomes the sessions' default (phase 3) as soon as instructions and code-finding hold, before the machine (owner, 2026-11-11).
- The machine is proven routine by routine (T27) before the whole program runs (phase 5), and the whole program from the level sync point before power-on.
- The hand engine is mounted (phase 6) only once the translated program passes level 1 on its own, so a failure after mounting is the seam's.
- The riskiest unknowns go first, in phase 0: how the hand engine plugs in (T00), which OS services the program needs (T01), and where the original actually runs (T02).

```
Phase 0  ▸  T00 T01 T02 T05 T17     settle the unknowns
Phase 1  ▸  T03 … T13               every instruction right
Phase 2  ▸  T14 T15 T16             finding the code, the stops
Phase 3  ▸  T18 T19 T20 T21         translator becomes the default
Phase 4  ▸  T22 … T27               the machine
Phase 5  ▸  T28 … T34               the whole program on the bar
Phase 6  ▸  T35 T36                 the seam, level 1 mounted
Phase 7  ▸  T37 … T42               the app, the drill, the close
```

---

## Phase 0 — Settle the unknowns

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-seam-spike.md) | seam-spike | — |
| [T01](tasks/T01-os-surface.md) | os-surface | — |
| [T02](tasks/T02-code-seen.md) | code-seen | — |
| [T05](tasks/T05-oracle-step.md) | oracle-step | — |
| [T17](tasks/T17-haiku-backend.md) | haiku-backend | — |

T00 is throwaway and gates T35's seam: per routine (adapter around each ported routine) or per level frame (the hand `Game::step` replaces `game_frame` while in level 1). T01 gates T26's OS answers. At the end: the OS surface and the code addresses are measured and committed, Moira steps one instruction, and the model tools call Haiku.

## Phase 1 — Every instruction right

| # | Task | Depends on |
|---|---|---|
| [T03](tasks/T03-whole-export.md) | whole-export | T02 |
| [T04](tasks/T04-census-and-split.md) | census-and-split | T03 |
| [T06](tasks/T06-insn-check.md) | insn-check | T04, T05 |
| [T07](tasks/T07-known-defects.md) | known-defects | T06 |
| [T08](tasks/T08-flags-arith.md) | flags-arith | T06 |
| [T09](tasks/T09-flags-shift-bit.md) | flags-shift-bit | T06 |
| [T10](tasks/T10-flags-move-control.md) | flags-move-control | T06 |
| [T11](tasks/T11-cycles-static.md) | cycles-static | T06 |
| [T12](tasks/T12-cycles-dynamic.md) | cycles-dynamic | T11 |
| [T13](tasks/T13-insn-gate.md) | insn-gate | T07, T08, T09, T10, T12 |

At the end every census kind matches Moira in every column (DESIGN §2.6).

## Phase 2 — Finding the code, the stops

| # | Task | Depends on |
|---|---|---|
| [T14](tasks/T14-run-stops.md) | run-stops | T04 |
| [T15](tasks/T15-code-write-rule.md) | code-write-rule | T02, T14 |
| [T16](tasks/T16-whole-tree-judge.md) | whole-tree-judge | T13, T15 |
| [T43](tasks/T43-call-fallthrough.md) | call-fallthrough | T03; blocks T28 |

At the end the whole program translates with no unsupported instruction, and every judgeable routine passes on the full tree.

## Phase 3 — The translator becomes the default

| # | Task | Depends on |
|---|---|---|
| [T18](tasks/T18-translate-command.md) | translate-command | T16, T17 |
| [T19](tasks/T19-haiku-trial.md) | haiku-trial | T17, T18 |
| [T20](tasks/T20-docs-and-packs.md) | docs-and-packs | T18, T19 |
| [T21](tasks/T21-default-check.md) | default-check | T20 |
| [T44](tasks/T44-haiku-decision.md) | haiku-decision | T19; blocks T42 |

T18 depends on T17 because T17 moves the shared judge and model plumbing out of Muse's `translate.py`, which T18 replaces. At the end a fresh session reaches for `tools/translate.py` unprompted.

## Phase 4 — The machine

| # | Task | Depends on |
|---|---|---|
| [T22](tasks/T22-machine-core.md) | machine-core | T11, T14 |
| [T23](tasks/T23-interrupts.md) | interrupts | T22 |
| [T24](tasks/T24-machine-chips.md) | machine-chips | T22 |
| [T25](tasks/T25-machine-sound-input.md) | machine-sound-input | T22 |
| [T26](tasks/T26-os-answers.md) | os-answers | T01, T22 |
| [T27](tasks/T27-routine-oracle-hw.md) | routine-oracle-hw | T16, T23, T24, T25 |

At the end translated routines are judged on the machine with real hardware effects.

## Phase 5 — The whole program on the bar

| # | Task | Depends on |
|---|---|---|
| [T28](tasks/T28-translated-engine.md) | translated-engine | T26, T27 |
| [T29](tasks/T29-conform-translated.md) | conform-translated | T28 |
| [T30](tasks/T30-level1-cases-a.md) | level1-cases-a | T29 |
| [T31](tasks/T31-level1-cases-b.md) | level1-cases-b | T29 |
| [T32](tasks/T32-power-on-cases.md) | power-on-cases | T29 |
| [T33](tasks/T33-later-world-cases.md) | later-world-cases | T32 |
| [T34](tasks/T34-generated-purity.md) | generated-purity | T28 |
| [T45](tasks/T45-custom-writes-compared.md) | custom-writes-compared | T29; blocks T30, T31, T32 |
| [T46](tasks/T46-level1-cases-a-rest.md) | level1-cases-a-rest | T30; blocks T36 |
| [T47](tasks/T47-machine-power-on.md) | machine-power-on | T32; blocks T33 |
| [T48](tasks/T48-level1-cases-b-pass.md) | level1-cases-b-pass | T30, T31; blocks T36 |
| [T49](tasks/T49-machine-dma-contention.md) | machine-dma-contention | T46; blocks T36 |

T30 to T33 may outgrow a session, as level 1's frame-matching tasks did; a worker that finds more failure kinds than fit asks to add a task (CLAUDE.md, adding a task mid-run).

## Phase 6 — The seam

| # | Task | Depends on |
|---|---|---|
| [T35](tasks/T35-seam.md) | seam | T00, T28 |
| [T36](tasks/T36-mount-level-1.md) | mount-level-1 | T30, T31, T35 |

## Phase 7 — The app, the drill, the close

| # | Task | Depends on |
|---|---|---|
| [T37](tasks/T37-app-translated.md) | app-translated | T18, T32, T36 |
| [T38](tasks/T38-app-sound.md) | app-sound | T37 |
| [T39](tasks/T39-app-stop.md) | app-stop | T37 |
| [T40](tasks/T40-app-e2e.md) | app-e2e | T33, T38, T39 |
| [T41](tasks/T41-app-drill.md) | app-drill | T40 |
| [T42](tasks/T42-close.md) | close | T21, T33, T34, T41 |

`fd-e2e` already drives the app, so no rig task is planned; T40 extends it and T41 is the app's drill.

---

## Main path: builder and wirer

| Step | Built by | Plugged in by |
|---|---|---|
| Whole-program export and translation | T03, T04 | T18 (`translate.py`), T28 (`fd-translated` build script) |
| Instruction corrections | T07 to T12 | `pcode_rust.py` loads `tools/pcode_fix/` (T06) |
| Stops and caps | T14 | `pcode-rt`, emitted by `pcode_rust.py` |
| Machine (memory, chips, sound, input, interrupts, OS) | T22 to T26 | T27 (judge), T28 (`TranslatedGame`) |
| Cases on the translated engine | T29, T32, T33 | `fd-conform --engine translated` (T29) |
| Seam and level 1 mount | T35, T36 | `fd-translated`'s dispatch (T35), `PORTED` (T36) |
| App from power-on, sound, stop | T37, T38, T39 | `fd-app/src/main.rs` (T37, T38, T39) |
| Default for sessions | T18, T20 | `CLAUDE.md`, `task_pack.py` (T20) |
| Haiku | T17 | `generate.py`, `reshape.py` (T17) |

## Critical path

```
T02 → T03 → T04 → T06 → T11 → T12 → T13 → T16 → T27 → T28 → T29 → T30 → T36 → T37 → T38 → T40 → T41 → T42
```

Off it: phase 3 (T18 to T21; T18 must finish before T37), T01 and T26 (must finish before T28), T00, T34, T32 and T33 (T33 before T40).

Leaves: only T42.

## Parallel width

43 tasks · longest dependency chain 18 · up to about 12 could run at once (phase 1's fix tasks beside phase 4's chip tasks).

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T03, T06, T12, T22, T23, T24, T26, T28, T30, T31, T32 (with the toolkit move), T35, T36, T37 |
| **Medium** | T00, T01, T02, T04, T07, T08, T09, T10, T11, T14, T15, T16, T18, T19, T20, T25, T27, T29, T33, T38, T39, T40, T41 |
| **Light** | T05, T13, T17, T21, T34, T42 |

Where it will overrun: T30 to T33 (whole-program agreement, as level 1's T41 to T48 did), T26 (the OS surface may be wider than the program's own notes say), T08 to T12 (the census may hold more kinds than expected).

## Decisions still open

- T00 settles the seam's granularity; T35 writes it into DESIGN §2.9. It blocks T35 only.
- If T19's Haiku trial falls below the pass mark, the owner decides (DESIGN §2.13). It blocks T20's wording of the generator and rewrite procedure only.
