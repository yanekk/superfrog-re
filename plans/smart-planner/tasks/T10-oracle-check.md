# T10 — oracle-check

**Phase:** 2 · **Depends on:** T02, T08, T09 · **Weight:** medium

Rewritten by the 2026-10-22 amendment (D24): stretch and repair, as the experiment's `--original`
loop (`plans/smart-planner/spike-explore/goexp.rs`, `main`).

## Goal

Plan against the original: the engine plans a whole stretch to the exit, the original plays it in
lockstep, and where they part the original's state is what planning continues from, because the
original lags in busy passes and the engine does not.

## Design sections this implements

DESIGN §2.6, §2.4 (deaths, `in_wall_loop`), §2.9 (oracle hang), D24.

## Files

- `engine/crates/sf-plan/src/oracle.rs`, `engine/crates/sf-plan/tests/oracle.rs`
- `engine/crates/sf-plan/src/lib.rs` (`pub mod oracle` only; T22 may run beside this task and does not
  touch lib.rs)
- `engine/crates/sf-plan/src/trace.rs` (T09's; reused, changed only if a stretch needs it)

## Interface

```rust
pub struct OracleRun { pub amiga: Amiga, pub inputs: Vec<FrameInput>, pub oracle_frames: u64, pub confirm_time: Duration }
pub fn start_at_level(id: LevelId, cap: u32) -> OracleRun;          // sync::reach(AtLevel)
pub fn start_at_snap(path: &Path, cap: u32) -> OracleRun;            // sync::at_snapshot_file
pub fn plan_original(run: OracleRun, facts: Facts, opt: Options, ro: &RunOptions)   // T08's RunOptions
    -> Result<(Route, Stats), (Route, Stats, Reason)>;
pub struct Stats { pub engine_frames: u64, pub oracle_frames: u64, pub confirm_share: f32, pub repairs: u32,
                   pub confirmed_frames: u32 /* last frame the original agreed to, DESIGN §2.9's reason */ }
```

Each stretch: transplant `sync::core_memory(amiga.memory())` into a `Game` (`Start::Transplant`),
`plan_engine` (T08, with the run's `RunOptions`, so the plan confirmed is the shortened one once T22
lands, DESIGN §2.12; whether repair stretches also get the shortening minimum is this task's choice,
recorded in FINDINGS) with the options' guide, play the plan on the oracle with `sync::step_inserting`
and the engine copy beside it, and stop at the first parting (DESIGN §2.6 step 2: cells more than
one apart, life lost but not a 1-up, `DYING`, `sync::in_wall_loop`, a heart the engine kept). Keep
to 16 frames before it (snapshot, restore, replay the kept frames), then the next stretch, with the
plan's rest as `--follow` (T09 `follow_options`, its trace from `core::repair::trace_engine` on the
transplanted state). The same parting cell three repairs running is
accepted. The exit on the original ends it.

## Tests

A test over ~30 s is `#[ignore]` with its time noted (DESIGN §5).

- [ ] W2-1 from `AtLevel` in original mode reaches the exit and passes DESIGN §4.1's check (what
      `sf-bench`'s `verify` does: `sf_conform::run` and the exit milestone; not by linking sf-bench) (`#[ignore]`, its time noted; the experiment: 85 s, 1 repair). The fast test plans W1-1's
      first stretch and checks the kept frames agree with the oracle's cells.
- [ ] A plan that loses a heart on the original but not on the engine parts there (fixture: a
      transplanted state and a plan that walks into an enemy only the original has in its way, or a
      unit test of the parting rule on recorded traces).
- [ ] The same parting cell three times running is accepted, not repaired a fourth time.
- [ ] A route that enters the `frog_tiles` wall loop on the original parts there (fixture snapshot).
- [ ] `Stats.confirm_share` is between 0 and 1 and oracle frames are counted.

## Done when

- [ ] Tests green; W5-2 and W1-4 in original mode, unguided, planned within 30 min each and passed by
      DESIGN §4.1's check (owner question open at the amendment review: W1-4 guided instead); time, repairs, confirm share and hearts lost in a FINDINGS row (the experiment: W5-2
      in 482 s, 8 repairs, 3 hearts lost before the heart check existed).
