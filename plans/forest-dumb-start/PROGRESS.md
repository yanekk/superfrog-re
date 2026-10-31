# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)** — read the rows
touching the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose, no bold-per-clause, no aphorism. The
cell is an index for the next session; the account is the commit message you are about to
write, and writing it twice is what turns a tracker into a history nobody reads. **Whoever
writes a cell also fixes the over-budget cell they walk past.**

**Plan reviewed:** 2026-10-31 — 7 fixed, 2 decided with the user

**Status:** All tasks built; T09 awaits review.
**Last updated:** 2026-10-31
**Next `pir-work` will:** review T09.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

**The Task cell is the task's kebab slug** (DESIGN §2.9) — the same name as its
`tasks/T{nn}-{slug}.md` file and, in parallel mode, its worker's agent name. One name, not two
to keep in sync.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | oracle-spike | — | ✅ | |
| T01 | ofs-reader | — | ✅ | |
| T02 | fd-import | T01 | ✅ | |
| T03 | fd-boot-chain | T00, T02 | ✅ | |
| T04 | fd-pack-check | T02 | ✅ | |
| T05 | fd-file-formats | T02, T04 | ✅ | |
| T06 | fd-exe-map | T02, T03 | ✅ | |
| T07 | frame-parameterise | T00 | ✅ | |
| T08 | fd-core | T00, T06, T07 | ✅ | |
| T09 | fd-first-case | T00, T02, T03, T08 | 🔍 | Runner, case, purity: 16 tests, same on 15079 passes. Deviations: sync 4922, T06's correction; added `swap` key; hand parser, `toml` not admitted; `read_adf` public, no direct amiga-disk; `test.sh` already complete, unchanged, not run: worker rule leaves it to pir's end run. |

A Notes cell holds what was built or what the review found, the test count, and one line per
deviation from the task doc. A deviation needing a paragraph needs the commit message: name
it here and point there.

**A ✅ task's cell may be cut to one line** once the next task has been reviewed — its detail
is in the commit and the file is read in full by every session from here on.

**Review queue:** T09

## Blocked on the user

*(Anything waiting on a person: what it is, the exact command, what to look for. Empty is a
good state and should say so.)*
