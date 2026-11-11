# T13 — insn-gate

**Phase:** 1 · **Depends on:** T07, T08, T09, T10, T12 · **Weight:** light

## Goal

Confirm stage 1 holds: one full `insn_check.py` run over the whole census, every column, on fresh states, and record it where later sessions look. This is the gate before the whole tree is judged and before the machine relies on cycles.

## Design sections this implements

DESIGN §2.6, success criteria.

## Files

- `games/forest-dumb/docs/14-translator.md` (new; T16, T27, T33 add to it): the census size and the gate reading.
- `games/forest-dumb/CLAUDE.md` Notes line for it.
- `fd-conform` full-set test `insn_gate` wrapping the run (labelled `full` in `smoke.toml`).

## Tests

- [ ] `insn_gate` fails if any census kind fails any column on 2 000 states with a new seed.

## Done when

- [ ] The full census passes all four columns on 2 000 states a kind; the reading is in `docs/14-translator.md` and FINDINGS.
- [ ] Any fix needed here is a general one in the right `pcode_fix` module, with its test.
