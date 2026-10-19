# T215 — channel-check-neighbours

**Split from:** T213 (carries its "Done when" item 1: the per-channel check green on `game-whole`
with the owner's tolerance).

**Phase:** 3 · **Depends on:** T213 · **Weight:** small

## Goal

T213's `sf-conform channels` judges each frame of play's channel level against the original's same
frame. On `game-whole` (T213's last build) 74 runs pass the owner's tolerance (`channels::OWNER`:
256, 25 %, runs of 2), nearly all a few lines' offset on a slow sound (the slot machine's effect,
WB-1's and the final level's channel-3 effects: a frame holds a few dozen of their sample bytes,
so where they fall moves its level ±50 %). The owner (2026-10-19) approved judging a frame against
the original's range over it and the frames either side: a sub-frame timing allowance, under which
a note never played still fails. Measured with a probe on that build: 74 runs fall to 4, all
channel 3 in WB-1, frames of play 180 887–180 943, owned by T216.

## Do

1. In `sf-conform/src/channels.rs` `judge`, an engine frame (at the chosen shift) differs on a
   channel when its level is further than the tolerance from the original's range over frames
   `n-1..=n+1` (`None` frames left out of the range). Unit tests: a slow sound a few lines off
   passes; a lost note (T213's `a_late_sound_lines_up_and_a_lost_note_fails`) still fails.
2. A case may list a known stretch: `[[exclude]] what = "channels", frames = [from, to]` with its
   note and task (`case.rs` `Excluded`), runs inside it reported, not failed. `game-whole.toml`
   lists 180 880–180 950 for T216.
3. An ignored test running `channels` over `game-whole` (feed from the trace cache; about 30 min
   uncached), and `engine/conform/README.md`'s section updated.

## Done when

- [ ] `sf-conform channels conform/cases/game-whole.toml` green, the T216 stretch listed as known.
- [ ] `tests/channels.rs` (title retrigger) still green; the new unit tests green.
