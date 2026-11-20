# T203 — whole-game-green

**Split from:** T21 (its done-when "The whole-game case is green in `sf-conform -- all`, and its
run time is in PROGRESS notes")

**Phase:** 2 · **Depends on:** T21, T202; blocks T23, T29 · **Weight:** medium

## Goal

`game-whole` (T202) green faithful and alone in `sf-conform all`, its run time in PROGRESS. Each
divergence T202 recorded is fixed here or, if it needs a task of its own, proposed to the owner
with its cause. A route that fails only because the chain arrives with other lives or score is
re-planned for the chain (`examples/plan_exit`, `examples/explore`).

## Files

As the divergences need; `engine/conform/cases/game-whole.toml`; a test in
`sf-conform/tests/chain.rs` (`#[ignore]` if slow, named in `conform/README.md`).

## Interface

None new.

## Tests

- [ ] `game_whole` green faithful and alone; no `unported` on the way.

## Done when

- [ ] Green in `sf-conform all`; run time in PROGRESS notes.
