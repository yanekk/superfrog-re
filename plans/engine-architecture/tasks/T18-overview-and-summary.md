# T18 — overview-and-summary

**Phase:** 3 · **Depends on:** T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17 · **Weight:** medium

## Goal

Turn seventeen chapters into one answer the owner can read: the overview page with the
connection diagram, the table of every chapter, the reuse totals that replace the 2026-10-15
estimate, and a recommended order for the native rewrite. Settle any ownership moves the chapter
tasks logged, make the drift check strict, and link the documentation from the repository README.

## Design sections this implements

DESIGN §2.4 (`ALLOW_TBD`), §2.6, §2.7.

## Files

- `docs/architecture/README.md`
- `tools/test_architecture_docs.py` (`ALLOW_TBD = False`)
- `tools/architecture.py` (only if `totals` needs a column the overview uses)
- any chapter's front matter `owns`, only for a move logged in `FINDINGS.md`, with a sentence in
  both chapters
- `README.md` (a line in the docs list and the Status table pointing at `docs/architecture/`)
- `plans/engine-architecture/FINDINGS.md`

## What the overview must contain

- What this is and how to read it: the four kinds, the reuse scale, the three tiers, the glossary.
- A Mermaid diagram of the crates and the mechanisms, showing which subsystems lean on which
  mechanism and where the verification tooling attaches (probe, transplant, `FrameOut`).
- One table: every chapter with tier, kind, mix, reuse and lines, as `tools/architecture.py
  totals` prints it, and the date it was computed.
- Totals: lines of code (DESIGN §2.2) per kind across the codebase, and per reuse rating,
  compared in one sentence with the 2026-10-15 "about 20%" estimate (DESIGN §1); the uncounted
  non-code lines stated once beside them.
- The recommended native rewrite order: which mechanism to replace first and why, taken from the
  mechanism chapters' "what depends on it" and "must preserve" sections. The memory-address
  coupling (DESIGN §2.5) is named as the first decision, with its options linked, not chosen.
- Open questions the chapters left, collected with links.

## Tests

- [ ] with `ALLOW_TBD = False` the test command passes (no chapter left `tbd`)
- [ ] the overview's table matches `tools/architecture.py totals` output on the merged tree
- [ ] every logged ownership finding is either moved or answered in `FINDINGS.md`

## Done when

- [ ] The overview contains every item above and the test command passes with `ALLOW_TBD = False`.
- [ ] `README.md` links to `docs/architecture/README.md`.
- [ ] No chapter body changed except for ownership moves logged in `FINDINGS.md`.
