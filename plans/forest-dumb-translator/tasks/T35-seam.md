# T35 — seam

**Phase:** 6 · **Depends on:** T00, T28 · **Weight:** heavy

## Goal

The mechanism by which a hand-written routine replaces a translated one by address (DESIGN §2.9), at the granularity T00 recommended, with the check that the replacing and the replaced routine agree on the routine oracle. Later level plans use it for every rewrite the owner chooses.

## Design sections this implements

DESIGN §2.9; this task writes T00's answer into §2.9.

## Files

- `crates/fd-translated/src/seam.rs` (new): the override table consulted by the tree's `enter`/`by_addr` dispatch (`pcode_rust.py` emits the hook).
- `crates/fd-core/src/layout.rs`: `trait WriteBack { fn write_back(&self, mem: &mut dyn MemMut) }`, the reverse of `Transplant`, for the variables a mounted routine changes.
- `crates/fd-core/src/ported.rs`: `Ported` gains `mounted: bool`.
- `fd-conform/src/judge/seam.rs`, `fd-judge --seam <addr>`: replaced against replacing on the same states.
- `plans/forest-dumb-translator/DESIGN.md` §2.9 (the granularity, with T00's numbers).

## Interface

```rust
pub type Mounted = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;   // same shape as a translated fn
pub fn mount(addr: (u8, u32), f: Mounted);           // built from PORTED at start
// an adapter per ported routine (or per frame, per T00): read LAYOUT vars -> typed state -> call -> write_back
```

## Tests

- [ ] Made-up: a translated routine replaced by a Rust fn that does the same passes `--seam`; one that differs in one byte written fails, naming it.
- [ ] `write_back` after `transplant` with no change writes nothing (round trip).
- [ ] Full: two `PORTED` routines mounted pass `--seam` on 2 000 states.

## Done when

- [ ] `fd-judge --seam` works; DESIGN §2.9 states the granularity and why.
- [ ] `fd-translated` stays in `PURE`.
