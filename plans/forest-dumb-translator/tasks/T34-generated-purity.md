# T34 — generated-purity

**Phase:** 5 · **Depends on:** T28 · **Weight:** light

## Goal

Check the generated code against the same rules as the pure crates, whenever it exists (DESIGN §2.11): no `fs`, `time`, `net`, `thread`, `env`, `process`, no clock types, no hash maps, no floats, and no integer table over 16 entries. That the app links no emulator is already held by `fd-app/tests/deps.rs` (`cargo tree`, refuses `amiga-oracle`, `fd-conform`, the e2e crates); it is not duplicated (owner, plan review 2026-11-11).

## Design sections this implements

DESIGN §2.11, §1 Stance.

## Files

- `fd-conform/tests/purity.rs` (scan `build/translator/tree/` when present; when absent the test says it scanned nothing and passes, since the stub is already scanned), `pcode_rust.py` if it emits a table.

## Tests

- [ ] A made-up generated file with a 17-entry table fails the scan.
- [ ] Full: the real tree passes.

## Done when

- [ ] The scan is in place and passes; `fd-app/tests/deps.rs` still passes with `fd-translated` and `pcode-machine` in the app's tree.
