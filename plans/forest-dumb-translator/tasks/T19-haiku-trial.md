# T19 — haiku-trial

**Phase:** 3 · **Depends on:** T17, T18 · **Weight:** medium

## Goal

Decide by measurement whether Haiku keeps the generator and rewrite jobs (DESIGN §2.13). Rerun level 1's recorded sets: T61's 5 generators and T62's 21 rewrites (T08's and T09's routines), the rewrites now starting from the translated routine (`translate.py`), the input sessions will really give it.

## Design sections this implements

DESIGN §2.13.

## Files

- `tools/haiku_trial.py`, `tools/test_haiku_trial.py` (new), `build/haiku-trial/` outputs, a section in `docs/11-translation.md`.

## Interface

```
haiku_trial.py [--only gen|reshape] [--tries 3]   # passes --max-calls 100 to model.py, the §5.3 row's bound
# build/haiku-trial/table.md: per item: Haiku result, branches covered (gen) or rig result (reshape), Muse's recorded result
# summary: {"gen_at_least_muse": n/5, "reshape_pass": n/21, "mark_met": bool, "calls": n}
```

Muse's numbers come from level 1's records (`plans/forest-dumb-level-1/FINDINGS.md`, `docs/11-translation.md`, T61 and T62 rows), cited per item.

## Outside actions

- Haiku trial batch — `worker`.

## Tests

- [ ] On fakes: the mark is computed right at 4/5, 5/5, 19/21, 20/21.

## Done when

- [ ] The table and the mark reading are in `docs/11-translation.md` and FINDINGS.
- [ ] If the mark is met, `generate.py` and `reshape.py` are the documented default; if not, the owner has decided (below) and the answer is in FINDINGS, dated.

## Needs a person

Only if the mark is missed.

Expect: the table, with what Haiku got wrong.
Tell me: drop the model (sessions write generators and rewrites by hand), or accept Haiku anyway.
