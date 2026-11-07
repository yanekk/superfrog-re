# amiga-input

The Amiga's input one frame at a time: the joystick bits and the keys held by raw key code (`FrameInput`), the text form scripts and recordings write them in, and the keyboard as CIA-A's serial port presents it (`CiaKeyboard`). A game's engine and the oracle both take `FrameInput`, so one script drives either; `amiga-oracle` re-exports `FrameInput`, `Joy` and `KeySet`. Pure: no clock, file or device is read, so an engine's tested core may depend on it. It does not map host keys or gamepads to Amiga input (that is `amiga-app::keys` and `amiga-app::pad`), and it does not hold a game's own keyboard poll routine, which stays with the game.

## Start here

```rust
let i = FrameInput::joy(Joy::RIGHT | Joy::FIRE);           // port-2 joystick
let k = FrameInput::key(0x40);                              // space held
let both = FrameInput { joy: Joy::UP, keys: KeySet::NONE.with(0x19) };
assert_eq!(format_tokens(i), "R F");
assert_eq!(parse_tokens("R F K19")?, FrameInput { joy: Joy::RIGHT | Joy::FIRE, keys: KeySet::key(0x19) });
```

The keyboard as a game reads it (from `src/keyboard.rs`):

```rust
let mut kb = CiaKeyboard::power_on();  // or CiaKeyboard::from_keys(key_down, key_up) for a start from memory
kb.feed(input.keys);                   // once per frame
if kb.new { /* the game's poll reads kb.sdr ($BFEC01) and clears kb.new */ }
```

## API by job

Joystick
- `Joy(pub u8)`: `NONE`, `UP` (1), `DOWN` (2), `LEFT` (4), `RIGHT` (8), `FIRE` (16), `PORT1_BUTTON` (32, the left mouse button in port 1, which loaders and intros wait for), `ALL` (63). `contains(self, other: Joy) -> bool`, `bits(self) -> u8`, `|` combines.

Keys
- `KeySet(pub u128)`: one bit per raw key code 0x00 to 0x7F (0x19 is P, 0x40 space). `NONE`, `key(code: u8) -> KeySet` (top bit of `code` ignored), `contains(code)`, `with(code)`, `without(code)`, `codes() -> impl Iterator<Item = u8>` (lowest first), `to_bytes() -> [u8; 16]`. Serde-serialisable: its layout is part of save states, pinned by a test.

One frame
- `FrameInput { joy: Joy, keys: KeySet }`: `NONE`, `joy(joy: Joy)`, `key(code: u8)`. `Ord` and `Hash`, so it can key maps.

Text form
- `format_tokens(input: FrameInput) -> String`: joystick tokens `U D L R F M` in that order, then `K<hex>` per key, `-` for nothing.
- `parse_token(t: &str) -> Option<Result<FrameInput, String>>`: `None` for a token this format does not know, so a game's script reader can add tokens of its own.
- `parse_tokens(s: &str) -> Result<FrameInput, String>`: whitespace-separated tokens ORed together; empty text is nothing held.
- `JOY_TOKENS: [(Joy, &str); 6]`.

Keyboard hardware (`keyboard.rs`)
- `key_byte(code: u8, up: bool) -> u8`: the byte the keyboard sends, `!(code << 1 | up)`.
- `CiaKeyboard { pub sdr: u8, pub new: bool, .. }`: `power_on()` (`$7e` released, measured), `from_keys(key_down: u8, key_up: u8)`, `feed(&mut self, keys: KeySet)` sends a byte for each key gone down then each gone up, keeping the last. `Default` holds `$ff` with `new` false. Part of a game's saved state.

## Where the details are

`src/lib.rs` (types, tokens), `src/keyboard.rs`; `toolkit/docs/architecture/frame-interrupts-input.md`. Script files built on these tokens: `amiga-conform::script`, `amiga-oracle::InputScript`, `amiga-app::script_core` (host events, a different format).
