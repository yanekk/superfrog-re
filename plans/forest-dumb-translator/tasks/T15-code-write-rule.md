# T15 — code-write-rule

**Phase:** 2 · **Depends on:** T02, T14 · **Weight:** medium

## Goal

Settle writes into code (DESIGN §2.5): the judge reports states where the original also writes into one of its executed instruction bytes as `self-modifying`, outside the count, and the code-writes check covers every path this plan runs, not only level 1.

## Design sections this implements

DESIGN §2.5.

## Files

- `fd-conform/src/judge/translated.rs` (classify), `fd-conform/src/code_writes.rs` (power-on and worlds 2 to 6 starts, the runs T02 defines), `fd-conform/src/main.rs` (`code-writes --all`; today `--cases` covers level cases only), tests.
- `tools/translator_trial.py` table column `self-modifying`.

## Tests

- [ ] Made-up routine writing into its own code on some states: those states are `self-modifying`, the rest judged; a translated store into code where the original does not write is still a failure.
- [ ] Full: `h0_6852` passes on the states the game can give, with its `self-modifying` count shown.
- [ ] Full: `fd-conform code-writes --all` reports 0 changed instructions over level 1's cases, the power-on run and the world starts.

## Done when

- [ ] `h0_6852` passes; the trial table shows 85 of 85 judgeable with `self-modifying` counted separately.
- [ ] The code-writes total over every path is in FINDINGS and `docs/14-translator.md`.
