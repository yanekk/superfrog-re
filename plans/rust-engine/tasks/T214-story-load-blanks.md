# T214 — story-load-blanks

**Phase:** 3 · **Depends on:** T213 · **Weight:** medium

Added by T213 with the owner's yes (2026-10-19).

## Goal

The story intro's music comes out of a load behind the original's, with a note lost. The original's
vertical-blank handler goes on running the music while the story loads its next scene; the engine
loads at once and runs nothing for the frames the load took. T30 fixed this for the game program's
loads (`Game::load_blanks`, `Game::take_load_paula`: the handler's sound on each blank the original
spent loading, mixed before the step after the load), but the story's loads get no blanks: in check
4's feed they read `blanks: 0`, because the runner counts only blanks the game program's handler
ran (`sync::game_owns_vbl`).

T213's channel check (`sf-conform channels`) finds it on `story-scene4-sound`: after the loads ending
on frames of play 2286 and 5221 the engine's music is about 5 frames behind the original's
(within the check's shift), channel 1's first note never plays (silent 13–18 frames where the
original's sounds at 524 and 875), and the scene's music stays apart from the original's for
hundreds of frames (channels 0 and 1 up to twice as loud at the same shift: the replayer comes out
of the load at another point of the song). `story-scene1-out` shows the same stretch.

So this task must:

1. **Run the story handler's sound on each blank the original spent in a story load**, as the game
   program's loads do: count them in the runner (the story's handler, not only the game's), feed
   them to the engine run alone (`FeedLoad::blanks`), and give their Paula writes to the mixer
   (`Game::take_load_paula` or its story equivalent), placed as T213 places a handler's.
2. Keep the story's state as the original's: the blanks' sound runs on the driver's state only, as
   `front::vbl::load_blank` does; checks 1–4 on the story cases stay green.

## Design sections this implements

DESIGN §2.1 (a sound no player could tell apart), §2.5.

## Files

- `engine/crates/sf-core/src/front/story/` (the story's handler and its loads), `sf-core/src/game.rs`
  (`load_blanks` for the story).
- `engine/crates/sf-conform/src/lib.rs` (counting a story load's blanks), `sf-conform/src/feed.rs`
  only if the feed must say more.
- `engine/crates/sf-conform/tests/channels.rs`: the story's channel check.

## Interface

`Game::load_blanks` and `Game::take_load_paula` serving a story load as a game one; nothing new in
the feed format unless the story's blanks need it (then `FEED_FORMAT` bumped).

## Tests

- `sf-conform channels conform/cases/story-scene4-sound.toml` green under the owner's tolerance
  (`channels::OWNER`), red today: channel 1 from frame of play 2286 for 13 frames.
- `story-scene1-out` likewise; a test in `tests/channels.rs` for one of them (ignored if slow).
- The story's cases in `tests/story.rs` still green.

## Done when

- [ ] A story load's blanks run the handler's sound in the engine run alone and reach the mixer.
- [ ] `sf-conform channels` green on `story-scene4-sound` and `story-scene1-out`.
- [ ] `story.rs`'s tests and the story cases under `sf-conform run` still green.
