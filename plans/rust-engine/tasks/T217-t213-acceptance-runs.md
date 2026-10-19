# T217 — t213-acceptance-runs

**Split from:** T213 (carries its "Done when" items 3 and 4).

**Phase:** 3 · **Depends on:** T213, T215 · **Weight:** small (long runs)

## Goal

T213 changed where the engine's Paula writes land (front end, load blanks, story, slot machine,
level order) and added `FrameOut::unheard`, which the app now mixes (`FrameOut::mixed`). By
construction neither changes game state nor the order or values conformance compares, but the two
acceptance runs were not made on the final build:

1. T30's ignored whole-game app test:
   `cd engine && cargo test -q --release -p sf-e2e --test whole_game -- --ignored --nocapture`
   (about 75 min; the feed is made first if the trace cache lacks it). Green, its output summary in
   PROGRESS notes.
2. `cargo run -q --release -p sf-conform -- all` (about 95 min): 212/212 green.

Run each in the background with a time limit; fix what breaks or add the task that does.

## Files

- None expected; `plans/rust-engine/PROGRESS.md` and `FINDINGS.md` for the summaries, and what
  a red run points at.

## Interface

None.

## Tests

- `engine/crates/sf-e2e/tests/whole_game.rs` (ignored) and `sf-conform all`, as above.

## Done when

- [ ] Both green on the build that has T213 and T215, summaries in PROGRESS and FINDINGS.
