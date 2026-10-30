# Landing a plan amendment into a live pir build

Moved from Claude's memory 2026-10-30; checked against `~/.claude/pir-engine/src/foundation/core/progress.mjs`
the same day. Corrected: the deps cell now has a real `blocks` clause (a worker may add one to a new
row), so the word is syntax there, not only a trap.

Learned landing the rust-engine "faithful" amendment, 2026-10-15:
- pir's PROGRESS parser knows only ⬜🟡🔍✅⛔; a dropped task must leave the table (⛔ blocks the run's end).
- The deps cell is split on the word "blocks": prose like "every row that blocks T21" breaks it.
- At merge, `adoptNewTaskRows(feature, branch)` flags any row a task branch carries differently as a
  forbidden edit, so after changing rows, fast-forward/merge the feature branch into every live task branch.
- Validate with pir's own `parseProgress` / `adoptNewTaskRows` from `~/.claude/pir-engine/src/foundation/core/progress.mjs`.
- Order: owner stops run → ff `pir/{slug}` → update task branches → owner restarts.
