# T28 — translated-engine

**Phase:** 5 · **Depends on:** T26, T27 · **Weight:** heavy

## Goal

The translated program as an engine: a crate that includes the generated tree at build time (a stub when there is none), runs it on the machine a frame at a time, and implements `fd-conform`'s `Engine` and `Transplant`, so the cases and later the app drive it like the hand engine.

## Design sections this implements

DESIGN §2.10 (no tree), §2.14, §3.3.

## Files

- `games/forest-dumb/engine/crates/fd-translated/` (new, pure): `build.rs` (includes `build/translator/tree/` or `FD_TRANSLATED`, else a stub), `lib.rs`, `README.md`. In `PURE`.
- `fd-conform/src/translated_engine.rs` (new): the thread-and-channel adapter implementing `Engine` and `Transplant` for `TranslatedGame`.

## Interface

```rust
pub struct TranslatedGame { /* Machine, Regs, foreground state */ }
pub enum Start<'a> { PowerOn { program: &'a [u8], disks: Disks }, Snapshot { mem: &'a [u8], regs: Regs68k, custom: &'a [u16; 256], disks: Disks } }
impl TranslatedGame {
    pub fn new(start: Start) -> Self;
    pub fn generated() -> bool;                      // false on the stub
    /// Runs the program; at every frame boundary (inside `Bus::poll`) calls `each` with the frame
    /// and takes the next frame's input from it. Returns when `each` says stop or the program stops.
    pub fn run(&mut self, each: &mut dyn FnMut(&FrameOut) -> Control) -> Option<Stop>;
}
pub enum Control { Next(Input), Stop }
pub struct FrameOut { pub picture: Picture, pub writes: Vec<(u64,u16,u16)>, pub paula: Vec<(u64,u16,u16)>, pub stop: Option<Stop> }
```

The foreground (`main_loop`, the title's loop) is ordinary Rust calls, so it cannot be paused mid-call and resumed by a pure crate (threads are outside `PURE`). So the engine drives: `run` calls back at each frame boundary. Shells that need a pull-style step (`fd-conform`'s `Engine::pass`, the app's loop) run `run` on a thread of their own and hand frames and inputs across a channel; that adapter lives in `fd-conform` here and in `fd-app` in T37. It must stay deterministic, because the cases compare frame by frame.

## Tests

- [ ] On the stub: `generated()` is false and `run` returns `untranslated` at once.
- [ ] Made-up program with a foreground loop and a level 3 handler: `each` is called once per frame and the counters advance as expected.
- [ ] Full: from a snapshot at level 1's sync point, 50 frames run with no stop.

## Done when

- [ ] The adapter implements `Engine` and `Transplant`; level 1's `w1-idle` case runs on it (pass not yet required).
- [ ] The crate is in `PURE` and builds without disks.
