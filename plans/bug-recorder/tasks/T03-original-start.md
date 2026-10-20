# T03 — original-start

**Phase:** 1 · **Depends on:** T02 · **Weight:** medium

## Goal

Bring the original to the same start the engine's `start_level` makes, with the same setup and seed,
as a sync point cases can name; and prove for every level and difficulty that the two starts are
equal, so a recording's comparison starts from the same state on both sides.

## Design sections this implements

DESIGN §2.8, §2.11 (every offered level reachable), §7 D2.

## Files

- `engine/crates/sf-conform/src/sync.rs`: `SyncPoint::Setup`, generalising what `Lives` does
- `engine/crates/sf-conform/src/case.rs`: its TOML form
- `engine/crates/sf-conform/src/lib.rs`: the engine side of a `Setup` case
- `engine/crates/sf-conform/tests/recorder_setup_start.rs` (new)
- `engine/conform/README.md`: the new sync form

## Interface

```rust
pub enum SyncPoint { /* … */ Setup { setup: LevelSetup, seed: Rng } }
```

```toml
sync = { level = "W1-1", lives = 5, points = 0, energy = 4, difficulty = "normal",
         spud = false, wings = false, seed = "1a2b3c4d 5e6f7a8b" }
```

`lives` in this form is the dialog's 3/5/7. The existing `{ level, lives = N }` form (raw
`frog_lives`) keeps its meaning; the parser tells them apart by the presence of any other setup key,
and the README says so.

The engine side of a `Setup` case is decided by T00: if the engine's own `start_level` equals the
original's start, it is used, which is what makes a case test the app's start; otherwise a
transplant, and the difference is a FINDINGS row.

## Tests

- [ ] TOML parse and refusal of the new form; the old `lives` form unchanged
- [ ] for each of the 29 levels, normal, default setup: engine `start_level` equals the original's `Setup` start at frame 0 by `level_entry.rs`'s rule (its `NOT_MODELLED` list minus what T00's start removes)
- [ ] W1-1 and W4-3 in easy and hard, and one non-default setup (points 450 000, energy 2, spud, wings, lives 7): equal at frame 0
- [ ] slow ones `#[ignore = "slow: …"]` with the time; the block keeps W1-1 normal and one non-default

## Done when

- [ ] `SyncPoint::Setup` exists and is documented in `engine/conform/README.md`
- [ ] the equality tests above pass (ignored ones run once by the worker, results in PROGRESS)
- [ ] the test command is green
