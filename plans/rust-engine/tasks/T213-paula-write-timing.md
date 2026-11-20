# T213 — paula-write-timing

**Phase:** 3 · **Depends on:** T30 · **Weight:** large

Added by T30 with the owner's yes (2026-10-19). It must finish before the run ends.

## Goal

Wrong notes in normal play are an audible defect in the shipped game. The engine's Paula writes
carry no beam position: every write of a frame reaches the app's mixer at its first line. When the
music retriggers a channel that is still partway through a sample, its `DMACON` off and on land at
one instant, so the channel never notices DMA went off, never restarts, and plays the old sample on:
the new note is lost. On the original the driver's writes are spread over the frame (W5, after the
148-blank load: the off at line 4, the on at line 16), long enough for the channel to go idle and
restart. Found by T30: in the paced whole game the note at frame 108762 is lost (the app's mix
silent 108764–108767 in a headless replay, where the original is heard); spacing the writes a
line apart in a probe
(`sf-conform/examples/feed_audio.rs`, `retrigger_lost.rs`) made those frames match the original.
Measured crudely, 95 353 of 224 082 frames of play sound different by where the writes land.

The check that should have caught it could not: DESIGN §2.1 check 3 compares sound events (a
sample's start address and length, its period and volume), not the channels' output, and a lost
restart leaves a wrong note playing, not silence, so T30's whole-game "heard where the original
is" check sees it only where the old sample has run out, and even then lets a gap of up to four
frames through (it allows ±2): it is green on the whole game with this note lost.

So this task must:

1. **Give the engine's Paula writes timing that makes Paula restart notes as the original does:**
   the original driver's beam positions for its writes, or the minimum spacing that gives the same
   channel output, whichever proves equal against the original. Every path that feeds the app's
   mixer gets it: the level's and the front end's frames, the story intro, and a load's blanks
   (`Game::take_load_paula`, T30).
2. **Add a check that compares each channel's actual output against the original's over whole
   cases** (vAmiga's per-channel taps, `FrameCapture::taps`, against the app's mixer run on the
   engine's writes and chip RAM, `Paula::channel_taps`), within a tolerance the owner sets, since
   check 3's events cannot see this. Run it over `game-whole`, or `sf-conform all`.

T30's whole-game sound check (`sf-e2e` `whole_game.rs`, ignored) must stay green:
`cd engine && cargo test -q --release -p sf-e2e --test whole_game -- --ignored --nocapture`.

## Design sections this implements

DESIGN §2.1 (a sound no player could tell apart), §2.5.

## Files

- `engine/crates/sf-core/src/audio/` (where the driver's writes are made) and `sf-core/src/game.rs`
  (`FrameOut::paula`); `engine/crates/sf-paula/` only if the mixer must read the positions
  differently.
- `engine/crates/sf-conform/`: the per-channel output check, its cases and a test.
- `engine/crates/sf-e2e/tests/whole_game.rs`: unchanged unless the check's bar changes (owner).

## Interface

`PaulaWrite`'s `vpos`/`hpos` filled by the engine (today 0), or a documented spacing rule; the new
check as an `sf-conform` subcommand or test, documented in `engine/conform/README.md`.

## Done when

- [ ] The per-channel output check runs over whole cases against the original, with the owner's
      tolerance, and is green on `game-whole`.
- [ ] A test that a channel retriggered mid-sample restarts as on the original (fails today).
- [ ] T30's ignored whole-game test still green, its output summary in PROGRESS notes.
- [ ] `sf-conform all` still green (the engine's writes' order and events unchanged).
