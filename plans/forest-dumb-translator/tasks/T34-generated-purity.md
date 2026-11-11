# T34 — generated-purity

**Phase:** 5 · **Depends on:** T28 · **Weight:** light

## Goal

Check the generated code against the same rules as the pure crates, whenever it exists (DESIGN §2.11): no `fs`, `time`, `net`, `thread`, `env`, `process`, no clock types, no hash maps, no floats, and no integer table over 16 entries; and check that the app links no emulator.

## Design sections this implements

DESIGN §2.11, §1 Stance.

## Files

- `fd-conform/tests/purity.rs` (scan `build/translator/tree/` when present; when absent the test says it scanned nothing and passes, since the stub is already scanned), `pcode_rust.py` if it emits a table.
- `fd-conform/tests/app_links.rs` (new): `cargo tree -p fd-app` holds neither `fd-conform` nor `amiga-oracle`.

## Tests

- [ ] A made-up generated file with a 17-entry table fails the scan.
- [ ] Full: the real tree passes.

## Done when

- [ ] Both tests are in place and pass; `app_links` is smoke (no disks needed).
