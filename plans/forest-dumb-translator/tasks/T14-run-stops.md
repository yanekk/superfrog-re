# T14 — run-stops

**Phase:** 2 · **Depends on:** T04 · **Weight:** medium

## Goal

Make every way a translated run can go wrong end as a named stop. The trial's step counter is per call, so endless recursion overflowed the Rust stack (SIGABRT) instead of stopping as a hang. This adds a global step count and a call-depth cap, and adds the interrupt-poll hook the machine needs (T23) at every block start, so emission changes once.

## Design sections this implements

DESIGN §2.4, §2.8 (interrupt poll), §5.2.

## Files

- `crates/pcode-rt/src/lib.rs`: `Regs::steps`, `Regs::depth`, caps, `Stop::Hang { why: Steps | Depth }`, `Bus::poll(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop>` with a default that does nothing.
- `tools/pcode_rust.py`: emit the global step check, depth increment and decrement around calls, and `bus.poll` at block starts.
- `fd-conform/src/judge/translated.rs`: run each state on a thread with a large stack (named size).

## Interface

```rust
pub const STEP_CAP_GLOBAL: u64;   // set by measurement: 10x the longest frame's steps seen in T16, recorded
pub const DEPTH_CAP: u32;          // 68000 code's deepest call chain seen x4, recorded
```

## Tests

- [ ] `bsr` to itself stops `hang (depth)`, no abort (made-up program).
- [ ] Two routines calling each other in a loop through `jsr` stop `hang (depth)`.
- [ ] A loop spread across many calls stops `hang (steps)`.
- [ ] `poll` is called once per block on a made-up program with a counting bus.

## Done when

- [ ] No made-up program can abort the process; every stop is a `Stop`.
- [ ] The trial's 84 routines still pass, and the run time is within 10% of before.
