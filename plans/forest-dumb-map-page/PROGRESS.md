# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Building; T03 reviewed ✅ 2026-11-20.
**Last updated:** 2026-11-20
**Next `pir-work` will:** T04 (page-map-only), now unblocked; T00, T01, T06 and T07 can run beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | ✅ | |
| T01 | objects-all-worlds | — | ✅ | |
| T02 | map-levels | T00, T01 | ⬜ | |
| T03 | chrome-rig | — | ✅ | Built `amiga-browser`, Chrome over the DevTools pipe; click test in smoke. Review clean, no fix commit. Ran drive.rs five times; probed reopen, missing file, promises, rejections, a network image (blocked), a bad `CHROME`. Mutations: no Fetch block (font red), no group kill (drop hangs). Pipe-inheritance race not reproduced with a widened window. |
| T04 | page-map-only | T03 | ⬜ | |
| T05 | page-live | T04 | ⬜ | |
| T06 | live-reader | — | ⬜ | |
| T07 | secrets-research | — | ⬜ | |
| T08 | map-command | T02, T05 | ⬜ | |
| T09 | secrets-layer | T07, T08 | ⬜ | |
| T10 | app-live | T02, T05, T06 | ⬜ | |
| T11 | app-show-map | T08, T10 | ⬜ | |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** (empty)

## Blocked on the user

Nothing.
