# T208 — w6-4-start-and-final-load

**Split from:** T203 (its goal: the divergences `game-whole` meets fixed, here W6-4's start and the final level's load)

**Phase:** 2 · **Depends on:** T203; blocks T23, T29 · **Weight:** medium

Added by T203 with the owner's yes (2026-10-17). T203 left `game-whole` green on checks 1–3 up
to W6-4; these are the two places checks 1–3 still fail (FINDINGS, T203 rows).

## Goal

1. **W6-4's start, a frame early** (engine 212074–212098, the original's 230931–230955). The
   engine's `level_start` for W6-4 runs a frame ahead of the original: its fade-in set-up begins at
   the end of 212072, the original's at line 299 of 212073. In 212072 the original's copper
   interrupt blits (the screen drawn, h0+$1697a from h0+$165e0, `level_start`'s scroller) run to
   line 258, past line $ff, so `wait_frame_raster` (h0+$73d0, from h0+$69c4) misses $ff and waits a
   frame; the engine's end before 255. Checks 1 (fade records, then `frog_anim` at 212098) and 2
   (the fade-in, 1456–2268 pixels past one step, 13–20 frames in a row) fail. Make the engine's
   level start take the frame the original takes, measured, not fitted to this one case (the other
   level starts must stay green: `code-*`, `chain-*`).
2. **`final_level_flag` 15 frames early** (engine 222665–222679, the original's 241522–241536):
   check 1 only; the final level's load re-aligns both. The engine reaches `world_load_lw`
   (h0+$a49a) sooner than the original after W6-4's tally (`top_loop` h0+$766..$772). Find what
   the original spends those frames on and model it, or show it is a load the runner should skip.

Judge by the faithful checks (DESIGN §2.1), not `--strict`; fix what fails a check, accept what is
within tolerance (owner, 2026-10-17). A difference that fails only on a variable that only shapes
the picture is raised with the coordinator, not tagged here.

## Files

As the causes need: `engine/crates/sf-core/src/front/` (`level_start`, `chain.rs`),
`engine/crates/sf-core/src/level/`; `engine/conform/cases/game-whole.toml` only for `frames` if
the end moves.

## Interface

None new.

## Tests

- [ ] `sf-conform scan conform/cases/game-whole.toml` lists no frame before the final level's
      load and none in 222665–222679.
- [ ] The level-start and chain cases the change touches stay green (name them in PROGRESS).

## Done when

- [ ] Both divergences fixed or accepted with the owner's word, each in FINDINGS with its check.
