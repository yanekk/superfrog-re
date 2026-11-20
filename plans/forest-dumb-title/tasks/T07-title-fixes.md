# T07 — title-fixes

**Phase:** 2 · **Depends on:** T03, T04, T05, T06 · **Weight:** heavy

## Goal

Make every title case of T03 to T06 pass on `--engine translated`, each fix general, by pattern in the
translator or the machine, never a hint, an address special case or an edit to generated Rust, and
with the 50 existing cases reading as before. This is where the plan's unknown work is.

## Design sections this implements

DESIGN §2.4; translator DESIGN §2.2, §2.7, §2.8.

## Files

Wherever each defect is: `games/forest-dumb/tools/pcode_*.py`, `engine/crates/pcode-machine/`,
`engine/crates/fd-translated/`, `engine/crates/fd-conform/src/frames.rs`. `title_idle_cases.rs`, `title_code_cases.rs`,
`title_quit_cases.rs`, `game_over_cases.rs`: every case moved to `PASSING`, `DIFFER` empty. `docs/14-translator.md` or `docs/16-title.md`: each
fix's reading, dated.

## Tests

- [ ] Each fix has a test that fails without it: a made-up program (`insn_*.rs`, `os_answers.rs`
      style) or a machine unit test, labelled in `smoke.toml`.
- [ ] The four case test files pass with every title case listed as passing.
- [ ] `fd-conform run-all --engine translated` before and after: 49 of 50 as at T58, the same failure.

## Done when

- [ ] Every title case passes translated; every `DIFFER` list is empty.
- [ ] `run-all` on the existing 50 reads as before, recorded in the commit.
- [ ] No hint added to `re/translator-hints.toml`, and no new frame bar allowance.

If a fix needs a new allowance, an OS model the translator plan chose not to build, or grows past one
session, ask the owner (a task added mid-run), with the case, the difference and the cost.
