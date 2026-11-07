# T08 — trial-report

**Phase:** 2 · **Depends on:** T05, T07 · **Weight:** light

## Goal

The trial ends with the owner's go or no-go (DESIGN §2.7). This task writes the report the owner reads, with the final table, the code-writes result and the gate reading, puts it in front of the owner, and records the answer. It is the plan's final deliverable.

## Design sections this implements

DESIGN §1 (success criteria), §2.6, §2.7.

## Files

- `games/forest-dumb/docs/12-translator-trial.md` (new): what was run and how to rerun it, the final table (routine names, addresses, verdicts and counts only, no listing text), the failure categories from T07, the code-writes total from T05 and what it cannot see, Muse's column and its totals, the gate reading, and what the trial says for stages 1 to 6.
- `games/forest-dumb/CLAUDE.md` (one line in Notes pointing at the report), `games/forest-dumb/docs/ROADMAP.md` (the translator section: the trial's outcome).

## Tests

- [ ] The table in the report equals `build/translator/table.md` of a fresh `translator_trial.py --skip-export` run (checked by the worker, stated in the commit).

## Done when

- [ ] The report is committed and the owner's answer is in `FINDINGS.md` as a ✅ row, dated, in their words.
- [ ] On go: `ROADMAP.md` says the second plan (`forest-dumb-translator`) is next once level 1 merges. On no-go: it says why, from the report.

## Needs a person

The worker prepares the report and the gate reading, then asks in the session and waits.

```
Read games/forest-dumb/docs/12-translator-trial.md: the table, the gate line, and what failed and why.
```

Expect: the gate line states judgeable routines, passes with no hand edit, the percentage against 90%, and whether level 1's path writes into code.
Tell me: go or no-go for the rest of the translator, and anything the second plan should take from it.
