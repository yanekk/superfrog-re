# T44 — haiku-decision

**Phase:** 3 · **Depends on:** T19 · **Weight:** light

Added by T19 (coordinator's instruction, 2026-11-12): Haiku's trial fell below the owner's pass mark, the
case DESIGN §2.13 and §5.1 leave to the owner, so the decision is set aside here and T20 is not held.

## Goal

Get the owner's decision on Haiku and apply it. **Open question (the owner's):** accept Haiku as the
default for `tools/generate.py` (test-state generators) and `tools/reshape.py` (drafts of chosen
rewrites), or no model, sessions writing both by hand.

The evidence, T19 (`docs/11-translation.md`, "Haiku's trial"; `build/haiku-trial/table.md` from
`haiku_trial.py`): generators 5 of 5, each reaching every branch way Muse's did; rewrites 19 of 21, the
mark being 20. `left_pressed` (`h0_4718`) had no answer within `model.py`'s 300 s on any of 3 calls;
`run_right_step` (`h0_497c`) failed 3 tries (one compile error, then 6 153 and 10 000 of 10 000 states
differing). The rig caught both; no wrong answer passed. T19's recommendation: accept, since generators
are the job a model was kept for and a rejected rewrite falls back to the session.

Until it is answered the plan's default stands (DESIGN §2.13): sessions write generators and rewrites by
hand, and T20 words the procedure that way.

## Design sections this implements

DESIGN §2.13, §5.1 (row "Haiku's trial result below the pass mark").

## Files

- If accepted: `games/forest-dumb/docs/11-translation.md` (the generator and rewrite steps call
  `generate.py` and `reshape.py` first, by hand after a `fail` or `timeout`), `games/forest-dumb/CLAUDE.md`,
  `games/forest-dumb/tools/README.md`, `tools/task_pack.py` and `tools/test_task_pack.py` where T20 wrote
  "by hand".
- If no model: the same files say by hand only; whether `generate.py`, `reshape.py`, `model.py` and their
  tests are deleted or kept unused is asked in the same question.
- `plans/forest-dumb-translator/FINDINGS.md`: the answer, dated.

## Tests

- [ ] `test_task_pack.py`: a porting task's pack carries the procedure the owner chose.
- [ ] If accepted: `test_generate.py`, `test_reshape.py` still pass. If deleted: no doc outside "History"
  sections names the deleted tools (the doc test T20 adds).

## Done when

- [ ] The owner's answer is in FINDINGS, dated.
- [ ] Every file above says what the owner chose.

## Needs a person

The decision itself.

Expect: the T19 table and the two failures above.
Tell me: accept Haiku as the default for generators and rewrites, or no model (and if no model, delete the
tools or keep them unused).
