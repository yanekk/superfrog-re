# T211 — app-whole-game-paced

**Phase:** 3 · **Depends on:** T21, T25, T26, T27, T28; blocks T30 · **Weight:** medium

Added by T29 with the owner's yes (2026-10-18). Carries T29's End to end item "a full scripted
game through the app".

## Goal

The real app plays the whole game (`conform/cases/game-whole.toml`, script `game-whole`) from
power-on to the end sequence, fed the original's pacing exactly as `sf-conform`'s engine run alone
is fed it, and its frames equal that headless run's frame for frame (same binary logic, so exact).
Since the headless run alone meets the original's milestones (T209, check 4), so does the app's.

The headless run alone (`sf-conform/src/lib.rs` `AloneRun::frame`) is fed per frame, all from the
oracle: the `Pacing` and whether the step ran twice, a load's vertical blanks
(`Game::load_blanks`), its loading-screen frames (`set_screen_frames`), its end line
(`set_load_line`), the RNG seeds read from the clock (`poke` of `rng_state` when the engine asks
for a seed), and the frames skipped under a loading screen (stepped `OnTime` on their inputs).

1. `sf-conform` writes that feed, one record per frame of play, into the trace cache (outside git,
   DESIGN §3.5), keyed as its traces are, while it runs the case alone (or by a subcommand that
   does). Its format is versioned and documented where it is written.
2. The app reads it as a test-only file beside `--script` (e.g. `--pacing FILE`, accepted only with
   `--script`); without `--script` it never reads one. It feeds the game in the same order as
   `AloneRun::frame`, sharing the code with `sf-conform` where it can (a small function in
   `sf-core` or `sf-conform` both call) so the two cannot drift.
3. A capture mode that writes a hash per frame (picture and state) instead of a PNG: 224 082 PNGs
   are several GB. The headless side writes the same hashes for its run alone.
4. An `sf-e2e` test, `#[ignore]`d unless asked (it runs ~75 minutes at the PAL rate; say in
   README how to run it), compares the two hash lists and reports the first frame that differs;
   a shorter stretch (e.g. power-on to W1-2's start) runs in the test block if it is bounded and
   quiet.

## Design sections this implements

DESIGN §2.1 check 4, §2.5 (normal play never lags: the feed exists only under `--script`), §3.5.

## Files

- `engine/crates/sf-conform/src/lib.rs` (write the feed), `engine/crates/sf-app/src/{args,main,capture}.rs`
  (read it, hash capture), `engine/crates/sf-e2e/tests/` (new `whole_game.rs`), READMEs.

## Done when

- [ ] The app's whole-game run, fed the recorded pacing, gives the same per-frame hashes as the
  headless engine's run alone of the same script and feed, run once and its time in PROGRESS notes.
- [ ] The app without `--script` cannot read a feed (a test says so).
- [ ] The short stretch's test is in the test block, or PROGRESS says why not.
