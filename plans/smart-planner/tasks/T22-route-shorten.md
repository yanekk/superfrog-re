# T22 — route-shorten

**Phase:** 1 · **Depends on:** T08 · **Weight:** medium

Added by the 2026-10-22 amendment (owner, D23): the experiment's routes ran up to 57% longer than the
committed ones (19–23% in original mode), against M4's 25%.

## Goal

Make the explorer's route shorter after it first finds the exit: keep searching for a shortening
budget and keep the shortest exit, then cut detours with a shortcut pass, deterministically.

## Design sections this implements

DESIGN §2.12, §1 M4, M8, D23.

## Files

- `engine/crates/sf-plan/src/core/shorten.rs` (T08's stub; `mod.rs` already declares it, so this task
  does not edit `mod.rs`, `lib.rs` or `explore.rs`, which T09 and T10 may hold at the same time)
- `engine/crates/sf-plan/src/run.rs` (the shortening budget and the shortcut pass in the shell's loop;
  T08's file, this task only adds the call after the first exit and fills `RunOptions`' shortening
  fields; `plan_engine`'s signature stays T08's)
- `engine/crates/sf-plan/tests/shorten.rs`

## Interface

```rust
// RunOptions (T08): shorten_share 0.2 and shorten_min_rounds (about 60 s of rounds, measured here) are
// `sf-plan`'s defaults (T11); RunOptions::default() keeps shortening off, so earlier tests that pin a
// route still hold. After the first exit: max(share × rounds so far, min) more rounds, shortest exit
// kept; the wall-clock budget still caps it (DESIGN §2.12).
/// The shortcut pass's bookkeeping (core): for states i < j on `route` (sampled every 8 frames from
/// `start`), the next pair to try as an explorer run from i with `Goal::Key { key: j's key,
/// within_frames: (j − i) − 1 }` (T08); a hit is spliced in and kept only if the whole route replays
/// to the exit with no more hearts lost. Repeats until a pass finds nothing or its rounds are spent.
pub struct Shortcut { /* route, pairs tried, rounds left */ }
impl Shortcut {
    pub fn new(start: &Game, facts: &Facts, route: Vec<FrameInput>, rounds: u64) -> Self;
    pub fn next(&mut self) -> Option<(Game, Goal)>;              // the state at i and the goal j
    pub fn offer(&mut self, found: Option<Vec<FrameInput>>);     // the explorer's result for it
    pub fn route(&self) -> &[FrameInput];
}
// run.rs drives each pair's explorer on the same parts and threads as plan_engine, seeded from
// --seed and the pair's index, so the pass is deterministic for any thread count (M8).
```

A shortened route is accepted only if it replays from `start` to the exit on the engine with no more
hearts lost than the original route; otherwise the longer one stands.

## Tests

A test over ~30 s is `#[ignore]` with its time noted (DESIGN §5).

- [ ] A route with a deliberate detour (T08's engine-timed fixture `tests/fixtures/w1-1-engine.txt`
      with a walk out and back spliced in; committed routes die early on the engine) is shortened
      by at least the detour's frames, and still exits.
- [ ] A shortened route never loses more hearts than the route it came from.
- [ ] Deterministic: same seed and rounds → same inputs, with 1 and with 8 threads (through
      `plan_engine` with shortening on).

## Done when

- [ ] Tests green; W2-1, W3-2 and W5-2 engine-only, unguided: route frames before and after shortening,
      against the committed routes, in one FINDINGS row (the experiment, before: 2 503, 10 962,
      7 578 frames; committed 2 524, 6 994, 6 094).
