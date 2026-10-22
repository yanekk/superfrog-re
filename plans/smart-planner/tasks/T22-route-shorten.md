# T22 — route-shorten

**Phase:** 1 · **Depends on:** T08 · **Weight:** medium

Added by the 2026-10-22 amendment (owner, D23): the experiment's routes ran 19–53% longer than the
committed ones, against M4's 25%.

## Goal

Make the explorer's route shorter after it first finds the exit: keep searching for a shortening
budget and keep the shortest exit, then cut detours with a shortcut pass, deterministically.

## Design sections this implements

DESIGN §2.12, §1 M4, M8, D23.

## Files

- `engine/crates/sf-plan/src/core/shorten.rs`, `engine/crates/sf-plan/src/core/mod.rs`
- `engine/crates/sf-plan/src/run.rs` (the shortening budget in the shell's loop; T08's file, this
  task only adds the call after the first exit)
- `engine/crates/sf-plan/tests/shorten.rs`

## Interface

```rust
pub struct ShortenOptions { pub after_exit_share: f32 /*0.2*/, pub after_exit_min: Duration /*60 s, shell side*/,
                            pub shortcut_frames_cap: u32 }
/// The shortcut pass: for states i < j on `route` (sampled every 8 frames from `start`), a short
/// explorer run from i targets j's archive key within (j − i) − 1 frames; a hit replaces the stretch.
/// Repeats until a pass finds nothing or `rounds` is spent. Rounds, never time (M8).
pub fn shortcut(start: &Game, facts: &Facts, route: &[FrameInput], seed: u64, rounds: u64) -> Vec<FrameInput>;
```

A shortened route is accepted only if it replays from `start` to the exit on the engine with no more
hearts lost than the original route; otherwise the longer one stands.

## Tests

- [ ] A route with a deliberate detour (W1-1's route with a walk out and back spliced in) is shortened
      by at least the detour's frames, and still exits.
- [ ] A shortened route never loses more hearts than the route it came from.
- [ ] Deterministic: same seed and rounds → same inputs, with 1 and with 8 threads.

## Done when

- [ ] Tests green; W2-1, W3-2 and W5-2 engine-only, unguided: route frames before and after shortening,
      against the committed routes, in one FINDINGS row (the experiment, before: 2 503, 10 962,
      7 578 frames; committed 2 524, 6 994, 6 094).
