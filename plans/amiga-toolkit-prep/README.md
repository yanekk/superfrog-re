# amiga-toolkit-prep — drafts, not a plan

Preparation for extracting a reusable Amiga toolkit (proposed 2026-10-15) before a second game.
Owner question 2026-10-23: reuse for game two is better served by the current, Amiga-shaped engine
than by the full native rewrite of `docs/architecture/README.md` ("Recommended order"), provided the
Amiga imitation becomes one layer instead of being woven through every subsystem. These four drafts
take the reuse-friendly parts of that order and scope each one to what the toolkit needs, nothing
more. Steps 4–5 of the native order (subsystems onto native state, native renderer) are out: they
cost reuse.

The owner took every recommended decision on 2026-10-24 (recorded in each draft); nothing is
reviewed by pir yet. Each draft is input for `@repo/single` (a one-run change) or
`pir plan` (multi-task); pir writes its own `BRIEF.md`/`PLAN.md` from it and reads it back.

| Draft | Size | Run as | Depends on |
|---|---|---|---|
| [1 — tables decoded at import](DRAFT-1-tables-at-import.md) | 400–700 lines (minimal scope) | `pir plan`, 3 tasks | none |
| [2 — one clock](DRAFT-2-one-clock.md) | 300–600 lines | `pir plan`, 3 tasks | 4 first (one blit to time) |
| [3 — one frame scheduler, one keyboard](DRAFT-3-one-scheduler.md) | 250–400 lines (A+B) | `@repo/single`; part C a later plan | A+B none; C after 2 |
| [4 — one copy of each chip model](DRAFT-4-one-chip-model.md) | 300–450 lines | `pir plan`, 2 tasks | none |
| [5 — extract the toolkit](DRAFT-5-extract-toolkit.md) | moves, not new code; phase A, 7 tasks + recorder | `pir plan` | 3 (A+B) and 2 merged |
| [6 — monorepo and knowledge tiers](DRAFT-6-monorepo-and-knowledge.md) | 4 tasks | `pir plan` | 5 phase A |

**Recommended order:** 4, then 3 (A+B), then 2, then 3C only if the north star still asks for it.
1 can run beside any of them; it touches different files.
Reason: 4 and 3A+B are small, independent and each removes a duplicate outright; 2 is easier once 4
leaves one blitter to time; 1 has the lowest toolkit payoff per line (most of it is Superfrog
content).

**After each step**, a session answers [the north star](NORTH-STAR.md) and appends a column.

## Readiness plan (rough, 2026-10-24)

| Stage | Work | Run as | Then |
|---|---|---|---|
| 0 | Merge this branch. One full `sf-conform all` (~2 h) as M1's baseline, so a later red is attributable to its step. Owner answers J1. | this session / owner | baseline column complete |
| 1 | Draft 4 (chips) and draft 3 A+B (keyboard, pass counter) side by side; only `front/mod.rs` is shared, a small merge | `pir plan` (2 tasks) + `@repo/single` | north-star column each; owner J2 |
| 2 | Draft 2 (one clock), after draft 4 | `pir plan` (3 tasks, last one runs `sf-conform all`) | north-star column; owner J2 |
| 3 | Readiness call: the five north-star conditions plus the dry fit below | one session | verdict paragraph in NORTH-STAR; owner says "extract now" |
| 4 | Extraction, draft 5: a `toolkit/` workspace in this repo, then split to its own repository (owner, 2026-10-27) | `pir plan` | toolkit repo pinned by Superfrog; game two is the next step |
| — | Draft 1 and draft 3 C only if a north-star row they move is the one still short | as their drafts say | |

Expected: after stage 2, M2's toolkit rows reach target and M4 lists three crates (`sf-paula`,
`amiga-chips`, the clock crate), about eight pir tasks in all. Draft 1's hunk-0 row is wanted, not
required.

**Dry fit (proposed, owner to confirm).** The counts show the pieces are separate; they do not show
they are enough. Before the call, a throwaway test program built from the toolkit crates alone (no
`sf-*` crate in its dependency tree) blits a pattern through the copper and display path, steps the
clock across a frame and plays a Paula note; and, once J1 is yes, game two's boot runs on the oracle
with the generic harness. Either failing names what the toolkit still lacks.

**If not ready after stage 2**, the north star names the short row; the owner chooses between the
draft that moves it (1 or 3 C) and extracting anyway with that piece left per game.

## Cross-cutting, for every draft

- **pir's test line runs `cd engine && cargo test -q --workspace`** beside the Python suite
  (`.pir/settings.json`, owner 2026-10-24); before, no engine check ran in a pir run. The slower
  `sf-conform` suites (`--ignored`, `all`) are still the builder's to run; each draft names them.
- **Faithful stays the bar** (rust-engine DESIGN §2.1). Every draft is a refactor: zero behaviour
  change, all `sf-conform` cases green, no tolerance touched (`Tolerance::OWNER` fixed). A red check
  is a bug in the refactor, never a reason to loosen.
- **The address coupling (native order step 0) is not decided here and need not be.** All four
  drafts keep `LAYOUT`, transplant and probe as they are.
- **New toolkit-bound crates join `PURE`** in `engine/crates/sf-conform/tests/purity.rs`, or the
  purity test fails.
- Project CLAUDE.md still calls T213 open; `plans/rust-engine/PROGRESS.md` has it ✅ (the open part
  is the story's `sfx_play` timing, rust-engine FINDINGS). Stale line, not touched here.

Sources: the four research reports behind the drafts (2026-10-23) checked each claim in the code;
baselines in `NORTH-STAR.md` were re-run by the writing session.
