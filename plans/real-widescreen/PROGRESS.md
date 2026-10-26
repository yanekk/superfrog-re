# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale tracker
costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The cell is an index; the account is the commit
message. Whoever writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-22 — 10 fixed, 4 decided with the user

**Status:** Planned; nothing built.
**Last updated:** 2026-10-22
**Next `pir-work` will:** T00 wide-spawn-spike, T02 wide-spawn and T04 wide-camera; T01, T05, T07 are done.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | wide-spawn-spike | — | ⬜ | |
| T01 | wide-mode | — | ✅ | |
| T02 | wide-spawn | T01 | ⬜ | |
| T03 | wide-capacity | T00, T01, T02 | ⬜ | |
| T04 | wide-camera | T01 | ⬜ | |
| T05 | wide-special-cases | T01 | ✅ | |
| T06 | wide-switch | T02, T03 | ⬜ | |
| T07 | wide-recording | T01 | ✅ | Review clean, no fix commit. Probed a hand-written v2 zip through the CLI (`W` held 100 frames toggles once, `- W` back; replay frame widths 334/464 at the toggles; compare exit 1, nothing written). Mutations caught: writer skipping toggle lines, toggle applied one step late. Three recorded deviations held by unit tests. App header/toggles left to T10. |
| T08 | side-objects | T02, T03 | ⬜ | |
| T09 | side-frog | T04, T08 | ⬜ | |
| T10 | app-wiring | T05, T06, T07, T08, T09 | ⬜ | |
| T11 | widescreen-drill | T10 | ⬜ | |
| T12 | wide-every-level | T11 | ⬜ | |

**Review queue:** —

## Blocked on the user

Nothing yet. T12 needs smart-planner merged into main, then the owner's hand-merge of main into
`pir/real-widescreen` with the run stopped (DESIGN §5, D5).
