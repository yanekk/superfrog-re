# T191 — chain-pause

**Split from:** T183 (its "Pause on P, as the original" and the `chain-pause` test)

**Phase:** 2 · **Depends on:** T183; blocks T21 · **Weight:** medium

## Goal

P in a level pauses and resumes as the original does. Today `level::pass::pause_check`
(h0+$71d8) `debug_assert`s it unported.

`pause` (h0+$7442), read by T183: `logic_running` cleared (so the vertical blank takes its
non-level path: sound, pointer, keyboard, `display_timer`, fades, no frog), a wait for the beam
past line `$38`, its line byte kept, a wait for fire to be let go, `key_down` cleared, then a busy
loop until `key_down` is `$19` again or fire is down, a wait for the same line byte, `key_down`
cleared, `logic_running` set, back into the pass (phase 9 of `level::frame::pass`). The pass is one
Rust function, so the pause needs the level scene to stop mid-pass across frames: phases 10–13
after unpausing.

## Design sections this implements

DESIGN §2.1.

## Files

- `engine/crates/sf-core/src/level/{pass.rs,frame.rs}`.
- `engine/conform/{scripts,cases}/chain-pause*`; `engine/crates/sf-conform/tests/chain.rs`.

## Interface

No new public API.

## Tests

- [ ] `chain-pause`: from W1-1's start (`sync = { level = "W1-1" }`), P pressed, held a frame,
      some frames paused, P again, play on: faithful and alone; in `tests/chain.rs`.
- [ ] Fire ending the pause too, in the same or a second case.

## Done when

- [ ] Green; `pause_check`'s `debug_assert` gone.
