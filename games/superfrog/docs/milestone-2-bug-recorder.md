# Milestone 2: the bug recorder

Moved from Claude's memory 2026-10-30; checked against plans/bug-recorder (15 tasks ✅, REPORT.md) and
docs/BUG-FIX.md. Corrected: this was a plan to come and is now built; kept as the record of why and of
the owner's decisions. The procedure is docs/BUG-FIX.md.

Decided 2026-10-15: once the `rust-engine` pir run finished, milestone 2 was a bug-report
flow, planned separately with `pir plan`.

Shape first discussed: the app keeps seed + per-frame inputs + level starts; a key saves a bug
(recording from the current level's start, screenshot, `W1-1:94,51`-style location, the owner's note);
a replay option. Fix flow: `@repo/single` → agent replays headlessly, compares with the original from
that level's start (quirk → keep, per DESIGN "keep original bugs"; divergence → fix), turns the
recording into a permanent case, fresh review, `Escape:` trailer.

**Why:** the owner wants to report bugs without testing fixes themselves
(knowledge/self-testing-faithful-bar.md).

Update 2026-10-20: planned as `plans/bug-recorder/` (branch pir/bug-recorder), since built. Owner decisions: Start Level dialog (Lives 3/5/7, Energy 1–4 dropdowns), STOP ⌘., one zip in git-ignored `bugs/`, game over never ends a recording, quirk → report only, test block scoped to touched crates.
