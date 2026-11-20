# T03 — pcode-translator

**Phase:** 1 · **Depends on:** T02 · **Weight:** heavy

## Goal

The translator itself: it reads the P-code export (T01's format) and writes one Rust module that runs every exported routine instruction for instruction on `pcode-rt`. No model, no per-routine knowledge; everything about the program arrives in the export and in the hints file. It is tested on made-up programs only, so its tests can be committed.

## Design sections this implements

DESIGN §2.2 (no special cases for an address), §2.3, §2.5, §2.8 (unsupported ops), §3.3, §3.4.

## Files

- `games/forest-dumb/tools/pcode_rust.py` (new; standard library only; names no game)
- `games/forest-dumb/tools/test_pcode_rust.py` (new)
- `games/forest-dumb/engine/crates/pcode-rt/tests/translated.rs` and `tests/fixtures/` (new: made-up exports and the test that runs their translation)
- `games/forest-dumb/smoke.toml` (labels)

## Interface

```
python3 -I games/forest-dumb/tools/pcode_rust.py <pcode.json> --out <tree.rs> [--hints <toml>]
  → prints {"functions":N,"blocks":N,"unsupported":N,"computed_unresolved":N}; exit 2 on a bad input, naming it
```

The emitted module:

```rust
// Generated from <pcode.json>; never committed when made from a game.
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store};
pub const CODE: &[(u32, u32, u32)] = &[ /* (hunk, start, end) per translated instruction run */ ];
pub fn entry(addr: &str) -> Option<fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>>;   // "hN+$off"
pub fn h0_1fec(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> { … }        // one per function
```

- One Rust function per call target. Inside it, basic blocks are arms of a `match` in a `loop`, keyed by instruction address; the step counter is checked once per block (`Stop::Hang` at `STEP_CAP`).
- `unique` varnodes become locals; `register` varnodes go through `Regs`; `ram` through `load`/`store`; hunk-relative references through `img.h`.
- `CALL` is a Rust call followed by the return-address check below; `CALLIND` and `BRANCHIND` dispatch over the function table or the routine's own block labels and return `Stop::Untranslated` otherwise; `RETURN` returns `Ok(())` when the popped address is the caller's expected return address, and otherwise looks it up the same way (68000 code may return to a pushed address).
- An op or `CALLOTHER` the translator does not handle emits `Stop::Unsupported` at that instruction only.
- With `--hints`, each hint adds a computed target or marks bytes as data; without it nothing about any address is assumed.
- The `bus` parameter is `&mut dyn Bus` or generic; choose by build time of a tree of ~9 000 instructions and say which in the commit.

## Tests

- [ ] Python: each P-code op maps to the expected Rust; an unknown op becomes one `Stop::Unsupported` and translation continues.
- [ ] Made-up exports (written as JSON by hand or produced from hand-written bytes, never from a game): a loop with `dbf`, a nested call, a call returning flags the caller branches on, a jump table resolved by a hint and the same table unresolved without it (`Untranslated`), a store into its own code (`WriteIntoCode`), an endless loop (`Hang`). `crates/pcode-rt/tests/translated.rs` builds each translation and checks registers and memory after it runs.
- [ ] The translator is deterministic: two runs on one input write identical files.
- [ ] The fixtures' translations are committed beside them (`tests/fixtures/*.rs`; made-up code, so allowed) and `translated.rs` includes them, so `cargo test` needs no Python. A Python test regenerates each and fails if it differs from the committed file, which is how a translator change reaches the Rust tests.

## Done when

- [ ] All tests above pass in `cargo test -q -p pcode-rt` and the Python suite; labelled in `smoke.toml`.
- [ ] `pcode_rust.py` contains no address, routine name or other fact of any game.
