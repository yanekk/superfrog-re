# T29 — engine-sound-ramp

**Phase:** 3 · **Depends on:** T25; blocks T15 · **Weight:** medium

Added by T25 with the owner's yes (2026-10-23).

## Goal

T25's W2-4 and W6-2 routes keep the original's state on every frame but fail `sf-bench`'s §4.1
faithful check on one sound frame each (FINDINGS, T25). Find the engine defect in `sf-core`'s
sound effects and fix it, so both routes pass §4.1, with no change to the planner or to the
faithful check's tolerances.

- W2-4, frame 7 658: channel 3, the original starts sample `$0714e8` (12 words), period 287,
  volume 33; the engine the same at volume 34. Frame 7 662: the original stops at 7 659, the
  engine writes period 286 (a ramp one tick on?).
- W6-2, frame 1 721: channel 3, the original sets volume 34 at frame 1 718; the engine nothing
  within 2 frames.

Both differences look like an effect's period/volume ramp (`sf-core/src/audio/sfx.rs`, `ramps`,
`$12d6a`) one tick out of step with the original's. Both routes keep the original mid-pass for long
runs (W6-2: 81 frames from 397), which the committed routes rarely do; the VBL ticks sound while
the main loop lags, so the trigger frame relative to the lag is a first suspect, not a finding.

## Files

- `engine/crates/sf-core/src/audio/**` (the fix).
- `engine/conform/cases/` and `engine/conform/scripts/`: one conformance case per route (or one
  cut to the frames that matter), green faithful after the fix and red before it.
- `plans/smart-planner/bench/tuning/results.jsonl`, `RESULTS.md`: W2-4 and W6-2 rerun.
- `plans/smart-planner/FINDINGS.md`.

## The routes

`sf-bench` is deterministic here (seed 1, neither run hit its budget): `cd engine && cargo build
--release -p sf-bench && SF_PLAN_LOG=1 ./target/release/sf-bench --set A --planner new --jobs 1
--levels W2-4,W6-2 --results ../plans/smart-planner/bench/tuning/results.jsonl --table
../plans/smart-planner/bench/tuning/RESULTS.md` regenerates them under `bench/tuning/routes/`
(13 + 5 min). T25's copies are on this machine at `~/.cache/superfrog-re/t25/new-A-W2-4.txt` and
`new-A-W6-2.txt`. A case for one: `sync = { level = "W2-4" }`, `script = "<path>"`, `frames` the
script's last line + 25, `resync = true`, the level completion case's `lag-limit` (as
`sf-bench`'s `verify::case`); `sf-conform scan` lists the differing frames, `sf-conform record
--paula` the original's Paula writes.

## Done when

- [ ] The defect is named in FINDINGS (what the engine did differently and why), fixed in
      `sf-core`, and each route is a conformance case that fails before the fix and passes after.
- [ ] `sf-conform all` stays green (the engine's existing cases, sound included).
- [ ] W2-4 and W6-2 rerun through `sf-bench` as above, the result lines committed; each completes
      §4.1, or its next difference is one FINDINGS row brought to the owner.
