# T216 — bonus-channel3-alone

**Split from:** T213 (the owner's follow-up, 2026-10-19).

**Phase:** 3 · **Depends on:** T213 · **Weight:** medium

## Goal

In `game-whole`, WB-1 frames of play 180 887–180 943, the engine run alone plays another effect on
channel 3 than the original: another envelope from 180 887, and at 180 932 an effect the original
does not play (`sf-conform channels`; `examples/channel_window` shows both sides' levels). From the
original's own state (a scratch case `sync = { script = "game-whole", frame = 197500, df1 = true }`,
`script-from = 197500`) the engine's channel 3 matches the original's, so the engine run alone
reaches WB-1 with a gameplay state that differs (which object plays what, when): check 4 allows
it, the sound does not.

## Do

Find the state that differs (`SF_CONFORM_ALONE_TRACE`, `examples/irq_writes`, `all_writes`), fix
it in the engine, and drop `game-whole.toml`'s known `channels` stretch (T215).

## Files

- `engine/crates/sf-core/` where the differing state is found (WB-1's objects or their sound).
- `engine/conform/cases/game-whole.toml` (its `channels` stretch dropped).

## Interface

None new.

## Tests

- `sf-conform channels conform/cases/game-whole.toml` green with no known stretch (T215's ignored
  test); a case or unit test at the state found, red before the fix.

## Done when

- [ ] `sf-conform channels conform/cases/game-whole.toml` green with no known stretch.
- [ ] `sf-conform run --alone` on the WB-1 cases still green.
