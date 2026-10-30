# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)** — read the rows
touching the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose, no bold-per-clause, no aphorism. The
cell is an index for the next session; the account is the commit message you are about to
write, and writing it twice is what turns a tracker into a history nobody reads. **Whoever
writes a cell also fixes the over-budget cell they walk past.**

**Plan reviewed:** 2026-10-30 — 10 fixed, 3 decided with the user
**Status:** Plan reviewed, awaiting the build start; no task started, nothing blocked.
**Last updated:** 2026-10-30
**Next `pir-work` will:** T04-games-forest-dumb, T05-architecture-chapters-carve.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

**The Task cell is the task's kebab slug** (DESIGN §2.9) — the same name as its
`tasks/T{nn}-{slug}.md` file and, in parallel mode, its worker's agent name. One name, not two
to keep in sync.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | knowledge-tiers | — | ✅ | |
| T02 | superfrog-move | T01 | ✅ | |
| T03 | playbook-and-template | T02 | ✅ | Clean, no fix commit. Eight mutations probed: seven red across test_playbook and the name check, the eighth was gutting the test itself. Template gitignore probed in a scratch repo. Root README gap stays a finding. cargo half: toolkit-checks only, no Rust changed. |
| T04 | games-forest-dumb | T03 | ⬜ | |
| T05 | architecture-chapters-carve | T02 | ⬜ | |

A Notes cell holds what was built or what the review found, the test count, and one line per
deviation from the task doc. A deviation needing a paragraph needs the commit message: name
it here and point there.

**A ✅ task's cell may be cut to one line** once the next task has been reviewed — its detail
is in the commit and the file is read in full by every session from here on.

**Review queue:** *(empty)*

## Blocked on the user

*(Anything waiting on a person: what it is, the exact command, what to look for. Empty is a
good state and should say so.)*
