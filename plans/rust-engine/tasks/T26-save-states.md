# T26 — save-states

**Phase:** 3 · **Depends on:** T24 · **Weight:** light

## Goal

Save and load the whole game anywhere through the menu, with a few slots, tagged with the engine build
and cache hash and refused when either differs (DESIGN §2.8).

## Design sections this implements

DESIGN §2.8, §2.11, §3.3 (`save`/`load`), §3.5.

## Files

- `engine/crates/sf-core/src/savestate.rs` — `Game::save`/`load` (serde + bincode, pure).
- `engine/crates/sf-app/src/states.rs`; `menu.rs` — Game → Save State ▸ 1–5 (⌘1–5 with shift), Load State ▸ 1–5.
- `engine/crates/sf-core/tests/savestate.rs`, `engine/crates/sf-e2e/tests/states.rs`

## Interface

```rust
pub struct StateHeader { pub magic: [u8; 4], pub engine_build: String, pub cache_hash: [u8; 32], pub frame: u32 }
pub enum StateError { NotAState, OtherBuild { theirs: String }, OtherCache, Corrupt }
```

## Tests

- [ ] save at frame N, run M frames → frames F1; load, run M → identical frames, Paula writes and probe, for N spread over every scene kind.
- [ ] other build, other cache, truncated, random bytes → the matching `StateError`.
- [ ] save encoding deterministic.

## End to end (the worker drives this)

- suite: `sf-e2e` · sizes: 2×
- [ ] save via the menu mid-level, play on, load → the screen equals the frame at save time.
- [ ] a state file from a build with a different `engine_build` → the message, game continues untouched.

## Done when

- [ ] Tests and e2e green.
- [ ] States written with `atomic_write` into `--home/states/`.
- [ ] The refusal message names both builds.
