# T01 — one-copy-each

**Phase:** 1 · **Depends on:** — · **Weight:** heavy

## Goal

Leave exactly one implementation of each Amiga chip piece that sf-core and sf-oracle repeat today:
the blitter's memory effect, the BLTSIZE decode, the Paula register filter `is_paula`, and chip
memory. Everything stays inside the crates it is in now; T02 moves the merged pieces out. It is a
pure refactor: no behaviour change, no tolerance touched.

## Design sections this implements

DESIGN §2.1 (rows 1–4), §2.3 (the pointer-wrap test), §2.4 (order), §2.5 (out of scope, with the
§7 exception for `level/bobs.rs`), §3.3 (no new files), §3.4 (the `StoryMem` serde form).

## Files

- `engine/crates/sf-core/src/front/blit.rs`: `bltsize_words`, `impl BlitMem for mem::Bus` may live
  here or in `mem.rs`; `Blit::run` gains the per-modulo mask for `Bus` callers only if §2.3's test
  says so; `cycles` uses `bltsize_words` with its arithmetic otherwise unchanged.
- `engine/crates/sf-core/src/level/objects.rs`: `Blit`, `blit` and `blit_words` deleted; `erase`,
  `draw_bob`, `draw_background` build a `front::blit::Blit`.
- `engine/crates/sf-core/src/level/scroll.rs`: `blit` deleted; `blit_planes` uses `Blit::run` and
  `bltsize_words`.
- `engine/crates/sf-core/src/level/panel.rs`: `blit_panel` deleted (the `measure.sh` pattern counts
  any `fn blit_panel`); its cost and register writes go to its two callers, `lives_icon_draw` and
  `lives_count`, in the same order.
- `engine/crates/sf-core/src/level/bobs.rs`: only `masks`'s two calls and the `use` line (DESIGN §7).
- `engine/crates/sf-core/src/front/mod.rs`, `front/story/mod.rs`: one `is_paula`.
- `engine/crates/sf-core/src/layout.rs`: the one `AmigaMemory`.
- `engine/crates/sf-core/src/front/story/mem.rs`: `StoryMem` as a newtype.
- `engine/crates/sf-oracle/src/amiga.rs` and its users: sf-core's `AmigaMemory`.
- `engine/crates/sf-conform/src/sync.rs`: `core_memory` removed, its callers pass the value through.
- Tests that build `StoryMem { chip, slow }` or call `core_memory` (e.g.
  `sf-conform/tests/story_draws.rs`), updated to the new shape only.

No new file under `engine/crates/`: every file there must be owned by an architecture chapter, and
`owns` changes only in T02 (DESIGN §3.3).

## Interface

```rust
// sf-core, front/blit.rs (moves to amiga-chips in T02)
/// BLTSIZE as (rows, words): height in bits 15–6, width in bits 5–0, 0 meaning 1024 / 64.
pub fn bltsize_words(size: u16) -> (u32, u32);
pub fn logic(m: u8, a: u16, b: u16, c: u16) -> u16;      // pub, no longer pub(crate)
impl<'a> BlitMem for crate::mem::Bus<'a> { /* w / set_w, so WriteLog and span see each word */ }

// sf-core, front/blit.rs or render/copper.rs
pub fn is_paula(reg: u16) -> bool;

// sf-core, layout.rs
pub trait Ram { fn byte(&self, addr: u32) -> u8; }
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AmigaMemory { pub chip: Vec<u8>, pub slow: Vec<u8> }
pub const SLOW_BASE: u32 = 0xC0_0000;
impl Ram for AmigaMemory { /* 0 off the map, as today */ }
impl AmigaMemory {
    // the oracle's readers, as today in sf-oracle: None off the map
    pub fn byte_at(&self, addr: u32) -> Option<u8>;   // name is the builder's; it must not
    pub fn word(&self, addr: u32) -> Option<u16>;     // shadow Ram::byte with a different type
    pub fn long(&self, addr: u32) -> Option<u32>;
}
impl sf_paula::ChipMemory for AmigaMemory { /* chip RAM only, past it reads 0, as today */ }

// sf-core, front/story/mem.rs
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StoryMem(pub AmigaMemory);   // own Default (512 KB each), own Debug, 24-bit mask kept
```

`AmigaMemory::hash` uses sf-oracle's private `hash_bytes`, which also checksums trace files, so it
stays in sf-oracle as a free function or an extension trait there (an inherent method on a foreign
type is not possible). The oracle's digest must stay bit-identical: cached traces store it.

The `StoryMem` newtype must serialise to the same bytes as today's `{chip, slow}` under bincode, so
save states written before the change still load.

## Tests

- [ ] First commit, before any merge: a unit test running a level-shaped blit (ascending, A→D and
      A/B/C→D) whose pointer passes `$7fffe` through a modulo, through both `objects::blit` and
      `Blit::run` over a `Bus` (or an equivalent `BlitMem`), recording whether per-modulo masking
      and start-only masking differ. The test stays, asserting the kept behaviour.
- [ ] `bltsize_words`: `0` → (1024, 64); `$0040` → (1, 64); `$0001` → (1024, 1); `$382` → (14, 2);
      `$ffff` → (1023, 63).
- [ ] A `Bus` blit through `Blit::run` logs the same `WriteLog` entries, in the same order, as the
      deleted `scroll::blit` did for the same copy (built in the test from the old code's behaviour).
- [ ] `blit_panel`'s replacement charges `Cost::PanelBlit` and `blit_words` and writes INTENA/DMACON
      in the same order (an existing beam or panel test may already pin it; if not, add one).
- [ ] `StoryMem` bincode bytes before and after are equal (a fixed `{chip, slow}` serialised with
      the old struct shape, defined in the test, decodes as the new newtype and re-encodes equal).
- [ ] `StoryMem`'s 24-bit mask: `b(0x0100_0400)` reads chip `$400` (existing
      `chip_and_slow_ram_are_where_the_cpu_sees_them` still passes).
- [ ] `AmigaMemory` both readers: `Ram::byte` gives 0 and the Option reader `None` past both RAMs;
      `ChipMemory::chip_word` past chip RAM gives 0.
- [ ] Every test named in DESIGN §4 passes unchanged.

## Done when

Word for word from the draft:

- `sh plans/amiga-toolkit-prep/measure.sh` prints: blitter memory-effect implementations 1, BLTSIZE
  decodes 3 (the helper and the two story ports), is_paula definitions 1, chip-memory containers 2
  (`AmigaMemory`, `StoryMem` newtype).
- The test line and the sf-conform runs below are green.

"The sf-conform runs below" are DESIGN §4's: from `engine/`, `cargo run -q --release -p sf-conform --
run conform/cases/render-*.toml conform/cases/slot-attract.toml conform/cases/front-*.toml conform/cases/complete-w1-1.toml` and
`cargo test -p sf-conform -- --ignored story`. Record their pass/fail in PROGRESS.

## Automated checks (the worker runs these)

```
sh plans/amiga-toolkit-prep/measure.sh
cd engine && cargo run -q --release -p sf-conform -- run conform/cases/render-*.toml conform/cases/slot-attract.toml conform/cases/front-*.toml conform/cases/complete-w1-1.toml
cd engine && cargo test -p sf-conform -- --ignored story
```
