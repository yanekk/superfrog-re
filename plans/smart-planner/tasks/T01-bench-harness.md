# T01 — bench-harness

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal

The instrument every target is measured with: `sf-bench`, which runs a planner on a benchmark set under
the unaided rules, verifies each route the way the completion cases are verified, records per-level
numbers to a resumable results file and renders the side-by-side table. It is its own crate,
`sf-bench`, which runs every planner as a child process, so the new planner (crate `sf-plan`, T03 on)
later plugs in as one more adapter and the two crates never collide.

## Design sections this implements

DESIGN §1 success criteria, §2.9 (killed run), §3.4, §4.1, §4.2, §5.2, D9, D10.

## Files

- `engine/crates/sf-bench/Cargo.toml` (new crate; deps sf-core, sf-conform, sf-oracle; bin `sf-bench`)
- `engine/crates/sf-bench/src/{main,sets,adapters,verify,results,table}.rs`
- `engine/crates/sf-bench/tests/bench.rs`
- `engine/crates/sf-conform/examples/{plan_exit,plan_exit_w6,explore}.rs` (one `PLAN_STATS` stderr line each)
- `plans/smart-planner/bench/` (created empty with a README line on regenerating RESULTS.md)

## Interface

```
sf-bench --set A|B|C --planner new|plan_exit|plan_exit_w6|explore|best-old
         [--levels W1-1,W1-4] [--jobs N] [--budget-min 30] [--seed 1] [--follow-standalone]
         [--results plans/smart-planner/bench/results.jsonl] [--table plans/smart-planner/bench/RESULTS.md]
sf-bench --table-only            # re-render RESULTS.md from results.jsonl

pub enum Set { A, B, C }
pub struct Start { level: LevelId, kind: StartKind }        // StartKind::AtLevel | Snap(PathBuf, u32 frame) | App
pub fn starts(set: Set) -> Vec<Start>;                      // 24 normal levels; B from whole_game leg snaps
pub trait Adapter { fn name(&self) -> &str; fn command(&self, s: &Start, seed: u32, out: &Path, budget: Duration) -> Command; }
pub fn verify(set: Set, s: &Start, script: &InputScript) -> Verdict;   // Verdict { completed, route_frames, hearts_lost, reason }
pub struct LevelResult { planner, set, level, commit, completed, wall_s, engine_frames, oracle_frames,
                         confirm_share, route_frames, ref_frames, hearts_lost, reason }
```

Old-tool adapters pass only level, `--snap`, `--out`, `--frames`, and `--coins 0` where the tool needs
it; none of `--via/--avoid/--ground/--jump/--launch/--fly/--skip/--to/--stomp/--hurt`. `--follow-standalone` (set B, new planner
only) passes the leg's standalone committed route as `--follow`. `best-old` runs
the three in turn, each with the full budget, and keeps the best (D10). Engine and oracle frame counts
come from a `PLAN_STATS` stderr line each adapter's tool prints at exit; T01 adds that one line to
`plan_exit`, `plan_exit_w6` and `explore` (the only edit to them).

Set B starts: run `whole_game --until W6-4` once into a scratch dir if the leg snaps are missing; the
leg's start frame is the snapshot's frame number. `ref_frames` is the committed route's exit frame
(`complete-*`/`*-complete` for A and C, `chain-*` for B).

## Tests

- [ ] `starts(A)` and `starts(C)` list the 24 normal levels in order; `starts(B)` the 24 normal legs
      (no WB-1, WW-1).
- [ ] `verify` on set A accepts `complete-w1-1` and rejects it truncated 200 frames before the exit.
- [ ] results.jsonl: append, reload, skip-on-rerun for the same (planner, set, level, commit); a
      truncated last line is ignored, not fatal.
- [ ] A result whose `PLAN_STATS` line has `hints` other than 0 is recorded as not completed,
      `reason=hinted` (DESIGN §2.10).
- [ ] A planner child exceeding budget + 5 min is killed and recorded as `reason=timeout`.
- [ ] The table renders baseline and new columns side by side with M1–M9 and C1–C4 rows from fixture
      results.

## Done when

- [ ] `sf-bench --set A --planner plan_exit --levels W1-1 --budget-min 30` writes one result line and a table.
- [ ] Tests above pass in the test block (the slow verify test may be `#[ignore]` with its time noted).
- [ ] Killing `sf-bench` leaves no `sf-oracle`/planner child running (checked with `pgrep`).
