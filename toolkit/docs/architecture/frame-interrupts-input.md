+++
id = "frame-interrupts-input"
title = "The frame, interrupts and input"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 70, game = 0, data = 0, shell = 30 }
reuse = "as-is"
owns = [
  "toolkit/crates/amiga-input/",
]
+++

# The frame, interrupts and input

## 1. What it is

A game of this era need have no game loop in the modern sense. Its work can be split between programs
that run against each other on the beam: a **vertical-blank interrupt** every frame at 50 Hz, a
**copper interrupt** it may ask for, and a **main program** paced on a counter or a raster wait. Nor
need it have an input routine: code reads the hardware where it wants it, the joystick through
`JOY1DAT`, fire as CIA-A PRA bit 7 low, the keyboard through CIA-A's serial register.

`toolkit/crates/amiga-input/` holds the part that is the machine's:

- `src/lib.rs`: `Joy`, `KeySet` and `FrameInput`, one frame's joystick bits and keys held by raw key
  code, taken by an engine and the oracle alike, so one script drives either. `format_tokens` and
  `parse_tokens` are the text form scripts and recordings write.
- `src/keyboard.rs`: `CiaKeyboard`, the keyboard as CIA-A's serial port presents it. The keyboard sends
  a byte for each key going down or up (`feed`: downs then ups, the last kept), the serial data
  register holds the last one and its flag says it came in. `key_byte` is `!(code << 1 | released)`.
  `from_keys` rebuilds the state from a start in memory. The poll that reads it is the game's, at the
  game's addresses, if it polls at all.

## 2. Kind mix

`amiga` 70, `shell` 30. The key codes, joystick bits and CIA are the Amiga's; the token format is test
tooling.

## 3. How the timing shows, in any game with this shape

A native loop that ran "update everything, then draw" once a frame loses each of these:

- Work in the handler runs every frame; work in a paced main program does not. Objects in the main
  program do not move on consecutive frames while the pass is on time.
- A late pass has the handler's work run in the middle of it, and its later calls read what the handler
  left.
- Input holds still within a frame, and is read at whichever rate the reading code runs.
- A polled keyboard is acted on where the main program reads the poll's result, and the poll's phase
  carries across scene changes.
- A copper interrupt's blits can run past the frame's end, and its register writes then belong to the
  next frame.

## 4. What depends on it

Every conformance script and recorded bug is a list of `FrameInput` in this token format
(`amiga-conform`, `amiga-record`, `amiga-oracle`). `amiga-app` holds the key table a desktop app feeds
it from.

## 5. What must be preserved

| Behaviour to keep | Check that proves it |
|---|---|
| Joystick bits and held keys | `joy_bits_combine_and_test`, `keyset_holds_raw_codes` (`toolkit/crates/amiga-input/src/lib.rs`) |
| The token format | `tokens_round_trip_and_refuse_what_they_do_not_know`, `every_joy_token_and_a_full_key_set_round_trip` (same file) |
| The keyboard's bytes, and its encoding in a game's save state | `the_keyboard_sends_a_byte_per_change`, `from_keys_puts_back_the_last_decoded_byte`, `save_state_bytes_are_unchanged` (`toolkit/crates/amiga-input/src/keyboard.rs`) |

## 6. Native sketch (non-binding)

Two schedulers rather than one loop: a per-frame step and a per-pass step behind an explicit frame
boundary, with a "where in the frame" value in place of cycle-charged cut points. The keyboard can be
an event queue with an adapter that reproduces the CIA: the last byte wins, and the poll's rate is
kept.

## 7. Reuse

**`as-is`**: joystick bits and raw key codes are the Amiga's and the token format is generic. The
skeleton of the handlers carries as a method (a blank handler, a paced main loop, a copper interrupt,
frames cut mid-pass, a per-frame hardware snapshot); their contents are each game's. Another game may
use the keyboard interrupt instead of polling, or run its logic at 50 Hz.

## 8. Sources

`toolkit/crates/amiga-input/`.
