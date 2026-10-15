# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md).** Read the rows
touching the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The cell is an index for the next session;
the account is the commit message. Whoever writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-10 — 7 fixed, 7 decided with the user

**Status:** Plan written; nothing built.
**Last updated:** 2026-10-10
**Next `pir-work` will:** T00 oracle-spike, because every later test rests on the reference emulator; T01 and T02 can run beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | oracle-spike | — | ✅ | |
| T01 | rust-workspace | — | ✅ | |
| T02 | re-workbench | — | ✅ | |
| T03 | disk-import | T01 | ✅ | |
| T04 | atn-decompressor | T03 | ✅ | |
| T05 | oracle-harness | T00, T01 | ✅ | |
| T06 | routine-oracle | T04, T05 | ✅ | |
| T07 | exe-map | T02, T04, T05 | ✅ | |
| T08 | asset-decode | T04, T07 | ✅ | |
| T09 | asset-cache | T03, T08 | ✅ | |
| T10 | paula-mixer | T01, T05 | ✅ | |
| T11 | audio-driver | T06, T07 | ✅ | |
| T12 | core-state | T05, T06, T07 | ✅ | |
| T13 | renderer | T08, T12 | ✅ | |
| T14 | frog-and-level | T06, T11, T12, T13 | ✅ | |
| T15 | items-and-scoring | T14 | ✅ | |
| T16 | enemies-worlds-1-3 | T15 | ✅ | |
| T17 | enemies-worlds-4-6 | T15 | ✅ | |
| T18 | bosses | T16, T17 | ⬜ | |
| T19 | slot-and-bonus | T15 | ✅ | |
| T20 | front-end | T08, T11, T12, T13 | ✅ | |
| T21 | game-flow | T18, T19, T20 | ⬜ | |
| T22 | app-rig | T09, T10, T12, T13 | ✅ | |
| T23 | app-shell | T21, T22 | ⬜ | |
| T24 | macos-menus | T23 | ⬜ | |
| T25 | controls-and-gamepad | T24 | ⬜ | |
| T26 | save-states | T24 | ⬜ | |
| T27 | crt-and-widescreen | T24 | ⬜ | |
| T28 | level-override | T14, T24 | ⬜ | |
| T29 | app-drill | T21, T25, T26, T27, T28 | ⬜ | |
| T30 | acceptance | T29 | ⬜ | |
| T31 | render-remaining-levels | T13 | ✅ | |
| T32 | story-intro | T20; blocks T21 | ✅ | |
| T33 | attract-demo | T16, T20 | ✅ | |
| T34 | game-over | T15, T20; blocks T21 | ✅ | |
| T35 | end-sequence | T20; blocks T21 | ✅ | |
| T36 | items-on-the-beam | T15 | ✅ | |
| T37 | level-restart | T36; blocks T34 | ✅ | |
| T38 | level-tally | T37; blocks T21 | ✅ | |
| T39 | enemies-world-5 | T16, T38; blocks T18 | ✅ | |
| T40 | enemies-world-6 | T38; blocks T18 | ✅ | |
| T41 | enemies-world-4 | T38; blocks T18 | ✅ | |
| T42 | enemies-world-1 | T38; blocks T18, T33 | ✅ | |
| T43 | enemies-world-2 | T38; blocks T18, T39 | ✅ | |
| T44 | enemies-world-3 | T38; blocks T18 | ✅ | |
| T46 | story-intro-scenes | T32; blocks T21 | ✅ | |
| T47 | story-intro-handover | T46; blocks T21 | ⬜ | |
| T48 | story-intro-skips | T46, T47; blocks T21 | ⬜ | |
| T70 | bonus-level | T15; blocks T21 | ✅ | |
| T71 | slot-sync | T15 | ✅ | |
| T72 | slot-screen | T71 | ✅ | |
| T73 | slot-rounds | T72 | ✅ | |
| T74 | slot-prizes | T73; blocks T21 | ✅ | |
| T75 | bonus-shots | T70; blocks T21 | ✅ | |
| T76 | bonus-enemies-a | T70; blocks T21 | ✅ | |
| T77 | bonus-enemies-b | T70, T76; blocks T21 | ✅ | |
| T78 | bonus-enemy-death | T70, T75; blocks T21 | ✅ | |
| T79 | bonus-level-frames | T70, T37, T75, T76, T77, T78; blocks T21 | ✅ | |
| T49 | story-fade-setup-job | T46; blocks T56 | ✅ | |
| T50 | story-objects-init-display | T49; blocks T56 | ✅ | |
| T51 | story-oracle-blit-tooling | T46; blocks T52 | ✅ | |
| T52 | story-blitter-bus-sim | T51; blocks T53 | ✅ | |
| T53 | story-draw-cpu-program | T52; blocks T54 | ✅ | |
| T54 | story-handler-end-feed | T53; blocks T56 | ✅ | |
| T55 | story-intro-sound-timing | T50; blocks T56 | ✅ | |
| T56 | story-intro-to-insert-disk | T49, T50, T54, T55; blocks T21, T47, T48 | ✅ | |
| T81 | beam-cleanup | T36; blocks T37 | ✅ | |
| T82 | beam-tests | T81 | ✅ | |
| T83 | items-w1 | T81 | ✅ | |
| T84 | items-w3 | T81 | ✅ | |
| T85 | items-w2 | T81 | ✅ | |
| T86 | items-launchers | T81 | ✅ | |
| T87 | slot-symbols-routines | T71; blocks T72 | ✅ | |
| T88 | slot-symbols-tables | T71; blocks T72 | ✅ | |
| T89 | story-copper-bus-cycles | T52; blocks T53 | ✅ | |
| T90 | story-draw-blits-oracle | T89; blocks T53 | ✅ | |
| T91 | slot-beam-races | T72 | ✅ | |
| T92 | slot-load | T72 | ✅ | |
| T93 | slot-attract-draw | T72, T91, T92 | ✅ | |
| T94 | slot-interrupt-timing | T72, T91, T93; blocks T73 | ✅ | |
| T95 | slot-load-finish | T92; blocks T93 | ✅ | |
| T96 | story-audio-dma-slots | T53; blocks T56 | ✅ | |
| T100 | front-vbl-timing | T37; blocks T21 | ✅ | |
| T97 | story-split-handler | T54; blocks T56 | ✅ | |
| T101 | lag-long-pass | T81, T82; blocks T21 | ✅ | |
| T102 | items-w3-faults | T101; blocks T21 | ✅ | |
| T105 | w2-1-hang-fruit-life | T85; blocks T21 | ✅ | |
| T106 | w1-completions | T42, T86, T101; blocks T18 | ✅ | |
| T107 | enemies-w1-2-case | T42; blocks T18, T33, T106 | ✅ | |
| T108 | enemies-w1-3-case | T42; blocks T18, T33, T106 | ✅ | |
| T109 | enemies-w1-4-case | T42; blocks T18, T33, T106 | ✅ | |
| T110 | enemies-w1-activation | T42; blocks T18, T33, T106 | ✅ | |
| T103 | w4-completions | T41, T101, T102; blocks T18 | ✅ | |
| T111 | enemies-w3-2-case | T44; blocks T18 | ✅ | |
| T112 | enemies-w3-3-case | T44; blocks T18 | ✅ | |
| T113 | enemies-w3-4-case | T44, T102; blocks T18 | ⬜ | |
| T114 | enemies-w3-activation | T44; blocks T18 | ✅ | |
| T115 | w3-completions | T44, T101, T102, T111, T112, T113; blocks T18 | ⬜ | |
| T121 | bob-draw-cut | T101; blocks T21 | ✅ | |
| T122 | w2-completions | T43, T101, T121; blocks T18 | ✅ | W2-1 done (`w2-1-complete`, red at 1094, 1508: T155); W2-2/3/4 split to T166–T168 (owner). Review: scan reproduced both reds, original exits at 2523, `script_seen` frames rerun, `plan_exit` flags probed on W2-1/W2-3; fix: T155 now carries the ledge breaking and the 2365 heart. No mutation: no tested code changed. |
| T120 | conform-no-silent-pass | T05, T36; blocks T21 | ⬜ | |
| T104 | w6-completions | T40, T101, T102; blocks T18 | ✅ | |
| T123 | enemies-w5-2-case | T39; blocks T18 | ✅ | |
| T124 | enemies-w5-3-case | T39; blocks T18 | ⬜ | |
| T125 | enemies-w5-4-case | T39; blocks T18 | ⬜ | |
| T126 | w5-completions | T39, T101, T121, T123, T124, T125; blocks T18 | ⬜ | |
| T127 | restore-sprite-wait | T101; blocks T21 | ⬜ | |
| T128 | w3-4-sprite-race | T127; blocks T113 | ⬜ | |
| T130 | front-vbl-318 | T100; blocks T21 | ⬜ | |
| T129 | slot-lamp-cut | T74; blocks T21 | ⬜ | |
| T140 | story-setup-under-music | T56 | ⬜ | |
| T141 | story-fade-colour-race | T56 | ⬜ | |
| T142 | story-draw-end-order | T56 | ⬜ | |
| T143 | story-last-fade-out | T56 | ⬜ | |
| T144 | story-intro-full-green | T56, T140, T141, T142, T143; blocks T21, T47, T48 | ⬜ | |
| T133 | w2-1-fruit-room | T105; blocks T21 | ⬜ | |
| T131 | w4-completion-faults | T103, T121, T127; blocks T18, T21 | ⬜ | |
| T132 | w4-4-completion | T103; blocks T18, T131 | ⬜ | |
| T145 | w4-4-dart-21 | T132; blocks T18, T131 | ⬜ | |
| T146 | w6-completion-faults | T104; blocks T18, T21 | ⬜ | |
| T147 | w6-2-completion | T104; blocks T18, T146 | ⬜ | |
| T148 | w6-2-alt-completion | T104, T147; blocks T18, T146 | ⬜ | |
| T149 | w6-3-completion | T104; blocks T18, T146 | ⬜ | |
| T150 | w6-4-completion | T104; blocks T18, T146 | ⬜ | |
| T151 | w6-4-alt-completion | T104, T150; blocks T18, T146 | ⬜ | |
| T153 | bob-draw-step-cuts | T121; blocks T21 | ⬜ | |
| T155 | w2-completions-green | T122, T127, T131, T132; blocks T18 | ⬜ | |
| T166 | w2-2-completion | T122; blocks T18, T155 | ⬜ | |
| T167 | w2-3-completion | T122; blocks T18, T155 | ⬜ | |
| T168 | w2-4-completion | T122; blocks T18, T155 | ⬜ | |
| T160 | attract-demo-entry | T33 | ⬜ | |
| T161 | attract-demo-init-timing | T160 | ⬜ | |
| T162 | attract-demo-level-start | T161 | ⬜ | |
| T163 | attract-demo-return | T162 | ⬜ | |
| T164 | attract-demo-cycles | T163 | ⬜ | |
| T154 | w1-4-completion | T106, T127; blocks T18 | ⬜ | |
| T165 | w1-1-completion | T106, T131; blocks T18 | ⬜ | |

A Notes cell holds what was built or what the review found, the test count, and one line per
deviation from the task doc. A ✅ task's cell may be cut to one line once the next task has been reviewed.

**Review queue:** *(empty)*

## Blocked on the user

Nothing yet. Expected later: granting Accessibility and Screen Recording to the terminal (T22), the
8BitDo check (T25), the play session (T30), and the open decisions in PLAN.md once T07 reports.
