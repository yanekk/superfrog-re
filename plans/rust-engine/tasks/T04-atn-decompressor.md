# T04 — atn-decompressor

**Phase:** 1 · **Depends on:** T03 · **Weight:** medium

## Goal

Port the game's `ATN!` decompressor to Rust, byte-identical to the emulated original on every chunk,
so the shipped game unpacks its data with no emulator. The algorithm outline is in docs/03; the
authority is the 68000 routine at stage-1.5 `+0x2DA`.

## Design sections this implements

DESIGN §2.4.

## Files

- `engine/crates/sf-disk/src/atn.rs`
- `engine/crates/sf-disk/tests/atn_reference.rs` — compares against `tools/atn.py` output generated
  into a temp dir by running the venv Python at test time.
- `docs/03-atn-codec.md` — the algorithm as ported, and the straggler results.
- `CLAUDE.md` — replace "do not try to reimplement it" with: the Rust port in `sf-disk/src/atn.rs` is
  the product decoder; `tools/atn.py` (emulated) is the reference it is tested against.
- `README.md` status table.

## Interface

```rust
pub enum AtnError { BadMagic, Truncated, Overrun { at: usize }, LengthMismatch { expected: u32, got: u32 } }
pub fn decompress(chunk: &[u8]) -> Result<Vec<u8>, AtnError>     // chunk = carved bytes starting at "ATN!"
```

## Tests

- [ ] all 103 chunks that decode in Python decode identically in Rust.
- [ ] the 3 stragglers (`d1_c16`, `d2_c39`, `d4_c16`): Rust result recorded (decodes or which error);
      if it decodes, length equals `decomp_size` and `d4_c16` parses as T7MP or the reason it doesn't is noted.
- [ ] truncated input, zero-length, wrong magic, a `decomp_size` lie: each returns its error, no panic.
- [ ] fuzz: 10 000 random mutations of a real chunk never panic and never allocate beyond `decomp_size`.

## Done when

- [ ] Byte-identical on all 103 reference chunks, in the test block.
- [ ] CLAUDE.md and docs/03 updated as above.
- [ ] Straggler outcome in FINDINGS (their check against the game's memory is T07's).
