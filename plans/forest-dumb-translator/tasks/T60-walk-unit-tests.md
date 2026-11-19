# T60 — walk-unit-tests

**Split from:** T56 (pir's context warning, 2026-11-19). Carries no "Done when" item of T56's; it covers T56's walker, committed without a disk-free test.

**Phase:** 6 · **Depends on:** T56; blocks T40, T42 · **Weight:** light

## Goal

Smoke tests, no disks, for `fd-translated/src/seam/walk.rs` and the translator's shape table (`pcode_rt::shape`, `tools/pcode_rust.py` `shape_row`): a walk makes exactly the calls the generated code makes. On a committed fixture (`pcode-rt/tests/fixtures`, e.g. `dbf_loop.rs`, `nested_call.rs`, `flags_call.rs`), run the generated function on a recording bus and walk its `SHAPES_*` with the decisions the run took on the same recording bus: the same sequence of `poll`/`insn`/`sync` calls and the same `Regs::cycles` and `off_bus_cycles`. Plus: `Busy` and `Same` stay at the front until the loop ends; a custom write with no value stops naming the register; a write of the wrong register or size stops; `Until::Call`, `Until::Read`, `Until::Write` (stopping at a return to depth 1), `skip_call`.

Python: `test_pcode_rust.py` gains a check that `shape_row` gives `Flow::Dbcc` the loop target (the last CBRANCH, not the first), `Flow::Ret` for `rts`, and a custom register's size and direction (`btst`/`cmp` read).

## Files

- `crates/fd-translated/src/seam/walk.rs` (unit tests) or `crates/pcode-rt/tests/translated.rs`; `tools/test_pcode_rust.py`; `smoke.toml` labels.

## Tests

- [ ] Smoke: the walk's calls equal the generated function's on at least three fixtures.
- [ ] Smoke: each `Decision` and `Until` case above.
- [ ] Smoke: the shape rows' flow and custom fields.

## Done when

- [ ] The tests hold and are labelled smoke.
