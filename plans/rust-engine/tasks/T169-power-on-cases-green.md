# T169 — power-on-cases-green

**Phase:** 2 · **Depends on:** T160; blocks T21 · **Weight:** small

Added by the T120 review (owner, 2026-10-15): `sf-conform -- all` has ten power-on cases red at
frame 0, the same without T120, and only `front-boot` and `front-attract` are named by any task
(T160). This task owns all ten.

## Goal

Every power-on case green in `sf-conform -- all` with its variables on: `front-boot`,
`front-attract`, `front-noskip`, `front-options`, `front-options-all`, `front-password`,
`front-skip-blink`, `front-skip-bounce`, `front-skip-left`, `front-skip-wait`.

## What is known

- Each diverges at frame 0 (the original's 3879) on `level1_vector` ($64): original `$fc0c8e`
  (Kickstart 1.3's level 1 autovector), engine 0. T44 added the variable to `LAYOUT`; the
  power-on `State` never sets it (FINDINGS 🐞 2026-10-14, "Not T95's").
- T33 found the fix (`State::default` sets `$64` to `$00fc0c8e` beside `EXEC_BASE`); it is in
  `plans/rust-engine/T33-wip.patch` (`KICKSTART_LEVEL1`), which T160 applies. Check whether T160
  landed it first; fix only what is left after it, case by case.

## Interface

None new.

## Files

- `engine/crates/sf-core/src/state.rs` (if T160 did not set the vector)
- `engine/conform/cases/front-*.toml` (only if a case needs a comment updated)
- wherever a case's next fault lives in `engine/crates/sf-core/src/`

## Tests

- [ ] each of the ten cases above green in `sf-conform -- all`.
- [ ] `tests/front.rs`'s `power_on_to_the_title_matches_the_original`,
      `the_level_code_entry_matches_the_original` and
      `fire_skipping_the_title_intro_matches_the_original` green.

## Done when

- [ ] All ten power-on cases green with vars on; any still red past frame 0 is fixed or has a
      named owning task in `FINDINGS.md`.
