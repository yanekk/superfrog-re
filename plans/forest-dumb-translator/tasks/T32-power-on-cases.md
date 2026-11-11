# T32 — power-on-cases

**Phase:** 5 · **Depends on:** T29 · **Weight:** heavy

## Goal

The translated program from power-on: from the program's entry, through the title, Start, the loads and into level 1, on the frame bar. Loads are instant here and take seconds on the oracle (DESIGN §2.8), so the comparison re-aligns the two runs at each load's return.

## Design sections this implements

DESIGN §2.7, §2.8, §2.14.

## Files

- `fd-conform/src/sync.rs` (power-on start at the oracle's first execution of `h0+$0`; re-alignment at each return from a file routine of `docs/07`, `load_file`, `load_file_raw`, `read_marker`), `fd-conform/src/case.rs` (`start = "entry"`), new cases `engine/conform/cases/power-on-title.toml`, `power-on-level-1.toml`.
- `first-case.toml` runs with `--engine translated` too.

## Interface

```toml
# a case
start = "entry"            # the oracle's first frame at h0+$0; the translated run starts there with the machine's OS answers
align = "loads"            # frames inside a load are skipped on the oracle side; compare resumes at the load's return
```

What is compared at entry: the program's hunks and chip RAM the program owns, not OS memory, which the machine does not model.

## Tests

- [ ] On a stand-in: a run with an extra 50 frames inside a load aligns and passes; a difference just after the load fails.
- [ ] Full: `first-case`, `power-on-title` and `power-on-level-1` pass with `--engine translated`.

## Done when

- [ ] The three cases pass; the alignment rule is in `engine/conform/README.md`.
