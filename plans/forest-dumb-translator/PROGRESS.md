# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-11-11 — 5 fixed, 6 decided with the user

**Status:** Planned 2026-11-11; nothing built.
**Last updated:** 2026-11-11
**Next `pir-work` will:** T24, T26 and T13 can run; T27 needs T16 and T24.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | seam-spike | — | ✅ | |
| T01 | os-surface | — | ✅ | |
| T02 | code-seen | — | ✅ | |
| T03 | whole-export | T02 | ✅ | |
| T04 | census-and-split | T03 | ✅ | |
| T05 | oracle-step | — | ✅ | |
| T06 | insn-check | T04, T05 | ✅ | |
| T07 | known-defects | T06 | ✅ | |
| T08 | flags-arith | T06 | ✅ | |
| T09 | flags-shift-bit | T06 | ✅ | |
| T10 | flags-move-control | T06 | ✅ | |
| T11 | cycles-static | T06 | ✅ | |
| T12 | cycles-dynamic | T11 | ✅ | |
| T13 | insn-gate | T07, T08, T09, T10, T12 | ✅ | |
| T14 | run-stops | T04 | ✅ | |
| T15 | code-write-rule | T02, T14 | ✅ | |
| T16 | whole-tree-judge | T13, T15 | ⬜ | |
| T17 | haiku-backend | — | ✅ | |
| T18 | translate-command | T16, T17 | ⬜ | |
| T19 | haiku-trial | T17, T18 | ⬜ | |
| T20 | docs-and-packs | T18, T19 | ⬜ | |
| T21 | default-check | T20 | ⬜ | |
| T22 | machine-core | T11, T14 | ✅ | |
| T23 | interrupts | T22 | ✅ | Review: 2 fixes, INTREQ bit 14 now level 6 as vAmiga; poll's retake loop had no test (added). rustfmt skip on the fixture mod. Probed: never-cleared request stops `hang (steps)`, stack outside memory stops `outside-memory`; game handlers never read the stacked PC (listing). Mutated mask, settle, loop: caught. |
| T24 | machine-chips | T22 | ⬜ | |
| T25 | machine-sound-input | T22 | ✅ | |
| T26 | os-answers | T01, T22 | ⬜ | |
| T27 | routine-oracle-hw | T16, T23, T24, T25 | ⬜ | |
| T28 | translated-engine | T26, T27 | ⬜ | |
| T29 | conform-translated | T28 | ⬜ | |
| T30 | level1-cases-a | T29 | ⬜ | |
| T31 | level1-cases-b | T29 | ⬜ | |
| T32 | power-on-cases | T29 | ⬜ | |
| T33 | later-world-cases | T32 | ⬜ | |
| T34 | generated-purity | T28 | ⬜ | |
| T35 | seam | T00, T28 | ⬜ | |
| T36 | mount-level-1 | T30, T31, T35 | ⬜ | |
| T37 | app-translated | T18, T32, T36 | ⬜ | |
| T38 | app-sound | T37 | ⬜ | |
| T39 | app-stop | T37 | ⬜ | |
| T40 | app-e2e | T33, T38, T39 | ⬜ | |
| T41 | app-drill | T40 | ⬜ | |
| T42 | close | T21, T33, T34, T41 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
