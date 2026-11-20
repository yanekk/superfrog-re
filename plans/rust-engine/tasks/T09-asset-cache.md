# T09 — asset-cache

**Phase:** 1 · **Depends on:** T03, T08 · **Weight:** light

## Goal

Make import happen once: encode `GameData` into a versioned, self-checking cache file, decode it on
launch, and decide on launch what to do (use the cache, re-import from remembered disks, or ask for
the disks). The decision is pure; the shell later only reads files and acts on the decision.

## Design sections this implements

DESIGN §2.2, §2.11, §3.5.

## Files

- `engine/crates/sf-assets/src/cache.rs`
- `engine/crates/sf-assets/src/launch.rs`
- `engine/crates/sf-assets/tests/cache.rs`

## Interface

```rust
pub const CACHE_FORMAT: u32 = 1;
pub fn cache_encode(data: &GameData, engine_build: &str) -> Vec<u8>;    // header: magic, CACHE_FORMAT, engine_build, blake3 of body
pub fn cache_decode(bytes: &[u8]) -> Result<GameData, CacheError>;     // BadMagic | OtherFormat(u32) | Corrupt | Truncated
pub struct LaunchFacts { pub cache: Option<Result<(), CacheError>>, pub remembered_disks: Option<Vec<Offered>> }  // gathered by the shell
pub enum LaunchAction { UseCache, Reimport { reason: String }, AskForDisks { reason: String } }
pub fn decide(facts: &LaunchFacts) -> LaunchAction
```

## Tests

- [ ] round trip: `cache_decode(cache_encode(d)) == d` for the real `GameData`.
- [ ] each corruption (flipped byte in body, truncated, other format number, wrong magic) is detected.
- [ ] `decide`: good cache → UseCache; bad cache + good remembered disks → Reimport; bad cache + missing or changed disks → AskForDisks; no cache, no disks → AskForDisks; reasons are human sentences.
- [ ] encoding is deterministic: same input, same bytes.

## Done when

- [ ] Tests in the test block.
- [ ] Decoding the cache is faster than a fresh import (measured, both numbers in PROGRESS notes).
- [ ] `decide` covers every combination in a table test.
