# T02 — amiga-chips-crate

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Move the pieces T01 left as one copy each, plus the copper, out of sf-core into a new crate
`engine/crates/amiga-chips` that depends on nothing of Superfrog and names nothing of it, so the
toolkit has its second crate beside `sf-paula`. A pure move: no behaviour change, no tolerance
touched. It also records the step in the architecture docs and the north-star log.

## Design sections this implements

DESIGN §2.2 (row 5), §3.1 (crate rules and what follows from them), §2.5 (frozen chapter bodies).

## Files

- `engine/crates/amiga-chips/Cargo.toml`, `src/lib.rs` and module files (names are the builder's;
  one of them holds `AmigaMemory`, the "memory file" below).
- `engine/crates/sf-core/Cargo.toml`: depends on `amiga-chips`.
- `engine/crates/sf-core/src/front/blit.rs`: a `pub use amiga_chips::…` re-export plus the
  `impl BlitMem for State` and `for StoryMem` it keeps.
- `engine/crates/sf-core/src/render/copper.rs`: a one-line `pub use amiga_chips::…` re-export.
- `engine/crates/sf-core/src/layout.rs`, `front/mod.rs`, `front/story/*`, `mem.rs`: re-export or
  import the moved items; `impl BlitMem for mem::Bus` stays in sf-core.
- `engine/crates/sf-oracle`: imports `AmigaMemory` from wherever keeps it compiling (sf-core's
  re-export or `amiga-chips` directly; the latter adds `amiga-chips` to its Cargo.toml).
- `engine/crates/sf-conform/tests/purity.rs`: `amiga-chips` on `PURE` and on `ALLOWED`.
- `engine/Cargo.toml`: `[profile.dev.package.amiga-chips] opt-level = 2`.
- `docs/architecture/display-hardware.md`, `docs/architecture/memory-model.md`: front matter `owns`
  only.
- `docs/architecture/README.md`: one line under "Recommended order", step 4.
- `plans/amiga-toolkit-prep/NORTH-STAR.md`: a log column.

## Interface

```toml
# engine/crates/amiga-chips/Cargo.toml
[package]
name = "amiga-chips"
version.workspace = true
edition.workspace = true
publish.workspace = true

[dependencies]
sf-paula = { path = "../sf-paula" }
serde = { version = "1", features = ["derive"] }
```

```rust
// amiga_chips, public: what row 5 lists, with the signatures T01 left
pub struct Blit { .. }            // with cycles(), run<M: BlitMem>()
pub fn word_times(..);            // byte-for-byte, as are cycles and FIXED_TO
pub fn logic(..);
pub trait BlitMem { .. }
pub fn bltsize_words(size: u16) -> (u32, u32);
pub trait Ram { .. }
pub struct AmigaMemory { .. }     // + Ram, the Option readers, sf_paula::ChipMemory
pub const SLOW_BASE: u32;         // and the chip/slow constants they need
pub fn is_paula(reg: u16) -> bool;
pub mod copper;                   // all of today's render/copper.rs, its tests included
```

Every existing `sf_core::…` path to these items keeps resolving through the re-exports, so callers
outside sf-core need not change.

## Tests

- [ ] The moved unit tests (`minterms`, the copper tests, T01's new tests) run in `amiga-chips` and
      pass unchanged.
- [ ] `purity.rs` passes with `amiga-chips` on `PURE`: no float, clock, file or table over 16
      elements in it, and its tree is only `amiga-chips`, `sf-paula` and serde's crates.
- [ ] `the_app_does_not_depend_on_test_tooling` still passes.
- [ ] Every test named in DESIGN §4 passes unchanged, and `python3 tools/test_architecture_docs.py`.

## Done when

Word for word from the draft:

- The crate exists under the rules above; `measure.sh` lists `PASS amiga-chips` beside
  `PASS sf-paula`, and the T1 rows are unchanged.
- `docs/architecture/display-hardware.md`'s `owns` gains `"engine/crates/amiga-chips/"`;
  `memory-model.md`'s gains `"engine/crates/amiga-chips/src/<memory file>"` (longest prefix wins);
  `python3 tools/test_architecture_docs.py` passes.
- `docs/architecture/README.md` gets one line under "Recommended order", step 4's repeated ports:
  the blitter and chip memory are merged and live in `amiga-chips`.
- `plans/amiga-toolkit-prep/NORTH-STAR.md`'s log gets a column "amiga-chips" with this branch's
  `measure.sh` values and M1 as this task's runs; J1 and J2 left "not asked" for the owner.

"The rules above" are DESIGN §3.1. "This task's runs" for M1 are the test block and DESIGN §4's
sf-conform runs, run on this task's branch; the full `sf-conform all` is not part of this plan.

## Automated checks (the worker runs these)

```
sh plans/amiga-toolkit-prep/measure.sh
python3 tools/test_architecture_docs.py
cd engine && cargo run -q --release -p sf-conform -- run conform/cases/render-*.toml conform/cases/slot-attract.toml conform/cases/front-*.toml conform/cases/complete-w1-1.toml
cd engine && cargo test -p sf-conform -- --ignored story
```
