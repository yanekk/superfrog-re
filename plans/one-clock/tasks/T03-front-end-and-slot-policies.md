# T03 — front-end-and-slot-policies

**Phase:** 3 · **Depends on:** T02 · **Weight:** heavy

## Goal

Put the front end's `Cx` and the slot machine's draw on the shared clock: their positions said with
`BeamPos`, their spending done through named policies in `amiga_clock::policy` that keep each one's
rounding exactly (whole lines for `Cx`, so note restarts cannot move). Then prove the whole step
with the full conformance run and the whole-game app test, and record it: the north-star column,
the findings on the policies, and the architecture README's step-2 line. It is the draft's T3. No
behaviour changes.

## Design sections this implements

DESIGN §2.5, §2.7, §3.3 (row T03), §4 (the T03 runs).

## Files

- `engine/crates/amiga-clock/src/policy.rs` (or wherever the builder puts the module).
- `engine/crates/sf-core/src/front/mod.rs`: `Cx::busy`, `Cx::blit_wait`, `Cx::unpack`'s line count,
  and where `Cx` reads its position.
- `engine/crates/sf-core/src/slot/draw.rs`: `spend` and the line sum above it, their callers.
- `plans/amiga-toolkit-prep/NORTH-STAR.md`: a log column "one-clock".
- `plans/one-clock/FINDINGS.md`: the policies.
- `docs/architecture/README.md`: one line under "Recommended order", step 2.

## Interface

```rust
// amiga_clock::policy — each today's arithmetic, named for its rounding
/// Lines `cycles` of work take when only whole lines are counted (Cx::busy, into unknown).
pub fn whole_lines(cycles: u32) -> u32;                      // cycles.div_ceil(LINE_CYCLES)
/// Work of `cycles` from `into` cycles into a line: (lines on, cycles into the new line).
pub fn exact_on_line(into: u32, cycles: u32) -> (u32, u32);  // Cx::busy, into known (T203)
/// The line a CPU wait for `need` cycles ends on, from `line`, a fetching line giving
/// LINE_CYCLES - `per_line` (Cx::blit_wait: per_line = slots * 123 / 100).
pub fn wait_lines(line: u16, need: u32, per_line: u32, fetching: impl Fn(u16) -> bool) -> u16;
/// Where `cycles` of work from `t` (CPU cycles from the frame's start) end, a fetching line
/// losing `taken` of its LINE_CYCLES (slot draw spend; taken = planes * words * lost / 10).
pub fn spend_stolen(t: u32, cycles: u32, taken: u32, wrap: u16, fetching: impl Fn(u16) -> bool) -> u32;
/// The CPU cycles free between `t` and `until` under the same steal (slot draw's line sum).
pub fn free_between(t: u32, until: u32, taken: u32, wrap: u16, fetching: impl Fn(u16) -> bool) -> u32;
```

`wrap` is the frame length the old code reduces lines by: 313 (`FRAME_LINES`) in both slot
functions today (DESIGN §2.2), not the slot's 312. `Cx` keeps `line: u16` and `into: Option<u32>`
as its state; the builder may hold them as a `BeamPos` where that leaves every value unchanged.
The fitted numbers (`123/100`, `CPU_LOST`, `BLITTER_LOST`, `cycles()*2/3`) stay at their call
sites; only the arithmetic moves.

## Tests

- [ ] Each policy equals the old function, kept in the test module as the reference, over a sweep:
      every line of a frame and past its end, cycle counts from 0 to several frames, `into` at 0,
      mid-line and the line's last cycle, the fetch set-ups the front end and slot use.
- [ ] `slot/draw.rs`'s existing `spend` tests pass unchanged.
- [ ] `front.rs` slot-attract and the front-end cases in the sf-conform suites, unchanged.
- [ ] Every check in DESIGN §4 for every task, green, no tolerance changed.
- [ ] `cargo run -q --release -p sf-conform -- all` green.
- [ ] `cargo test -p sf-e2e --test whole_game -- --include-ignored` green.

## Done when

- `Cx` and the slot draw spend only through `amiga_clock::policy`; `measure.sh`'s beam row lists
  only `BeamPos` and its clients (checked by reading the hits), constants ≤ 3, `PASS amiga-clock`.
- DESIGN §4's runs green, `sf-conform all` and `whole_game.rs` included, no tolerance changed.
- `NORTH-STAR.md` has a column "one-clock" with this branch's `measure.sh` values and M1 as this
  task's runs, J1/J2 left for the owner; `FINDINGS.md` names the policies; `docs/architecture/README.md`
  step 2 has a "Done (minimal) 2026-…" line naming `amiga-clock`.

## Automated checks (the worker runs these)

```
sh plans/amiga-toolkit-prep/measure.sh
python3 tools/test_architecture_docs.py
cd engine && cargo test -p sf-conform --test beam
cd engine && cargo test -p sf-conform --test frog_cases
cd engine && cargo test -p sf-conform --test alone
cd engine && cargo test -p sf-conform --test story -- --ignored
cd engine && cargo test -p sf-conform --test channels -- --ignored
cd engine && cargo run -q --release -p sf-conform -- all
cd engine && cargo test -p sf-e2e --test whole_game -- --include-ignored
```
