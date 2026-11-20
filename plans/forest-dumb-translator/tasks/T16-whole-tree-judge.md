# T16 — whole-tree-judge

**Phase:** 2 · **Depends on:** T13, T15 · **Weight:** medium

## Goal

Prove stages 1 and 2 together: the whole program translates with no unsupported instruction and no unresolved computed jump, and every routine that has a generator passes on the whole tree, with no hand edit. This is the point the translator becomes good enough to be the default (DESIGN §2.12).

## Design sections this implements

DESIGN §2.2, §2.3, §2.12.

## Files

- `tools/translator_trial.py` (`--whole`: judge every routine with a registered generator on the whole tree), `docs/14-translator.md`.

## Tests

- [ ] Full: the whole-tree run reports `unsupported 0`, `unresolved_computed 0` (or each remaining one listed as needing a hint, which fails the gate).
- [ ] Full: every judgeable routine passes on 2 000 states with no hint.

## Done when

- [ ] The reading (instructions, routines judged, passes, self-modifying, unjudgeable) is in `docs/14-translator.md` and FINDINGS.
- [ ] `re/translator-hints.toml` still holds no hint, or each hint is shown to the owner as a hand edit before this task closes.
