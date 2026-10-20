# T10 — oracle-check

**Phase:** 2 · **Depends on:** T02, T08, T09 · **Weight:** medium

## Goal

Plan against the original: the engine plans, the original confirms, and the original's state is what
planning continues from, because the original lags in busy passes and the engine does not. The
confirmation mode (chunked or stretch-and-repair) is the one T02's FINDINGS row decided.

## Design sections this implements

DESIGN §2.6, §2.4 (deaths, `in_wall_loop`), §2.9 (oracle hang).

## Files

- `engine/crates/sf-plan/src/oracle.rs`, `engine/crates/sf-plan/tests/oracle.rs`

## Interface

```rust
pub enum Confirm { Chunked { commit: u32 /*48*/ }, Stretch { max: u32 } }
pub struct OracleRun { pub amiga: Amiga, pub inputs: Vec<FrameInput>, pub oracle_frames: u64, pub confirm_time: Duration }
pub fn start_at_level(id: LevelId, cap: u32) -> OracleRun;          // sync::reach(AtLevel)
pub fn start_at_snap(path: &Path, cap: u32) -> OracleRun;            // sync::at_snapshot_file
pub fn plan_original(run: OracleRun, opt: Options, mode: Confirm, threads: usize, budget: Duration)
    -> Result<(Route, Stats), (Route, Stats, Reason)>;
pub struct Stats { pub engine_frames: u64, pub oracle_frames: u64, pub confirm_share: f32 }
```

Each chunk or stretch: transplant `sync::core_memory(amiga.memory())` into a `Game`
(`Start::Transplant`), plan on the engine, play on the oracle with `sync::step_inserting`, reject on
life lost (not a 1-up), `DYING`, or `sync::in_wall_loop`; on rejection try the next candidate, then
backtrack (bounded count). The oracle's snapshot/restore is used for candidates, as in `plan_exit`.

## Tests

- [ ] W1-1 from `AtLevel` in original mode reaches the exit and the route passes the faithful case check
      (`#[ignore]` if over the test block's budget; its time noted; the fast test runs the first 600
      frames and checks the oracle agrees with the route's recorded cells).
- [ ] A route that enters the `frog_tiles` wall loop on the original is rejected (fixture snapshot).
- [ ] `Stats.confirm_share` is between 0 and 1 and oracle frames are counted.

## Done when

- [ ] Tests green; W1-4 in original mode planned within 30 min, time and confirm share in FINDINGS.
