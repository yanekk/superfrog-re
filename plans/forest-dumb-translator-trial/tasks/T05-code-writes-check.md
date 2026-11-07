# T05 — code-writes-check

**Phase:** 1 · **Depends on:** T00 · **Weight:** light

## Goal

A static translator cannot follow a program that rewrites its own instructions, so the gate asks whether level 1's real path does (DESIGN §2.6, §2.7). The judge only sees generated states; this task runs level 1's conformance scripts on the original and reports any change to bytes of instructions the game executed, frame by frame.

## Design sections this implements

DESIGN §2.6, §2.7, §4.

## Files

- `games/forest-dumb/engine/crates/fd-conform/src/code_writes.rs` (new)
- `games/forest-dumb/engine/crates/fd-conform/src/main.rs` (a `code-writes` subcommand)
- `games/forest-dumb/engine/crates/fd-conform/tests/code_writes.rs` (new), `games/forest-dumb/smoke.toml`

## Interface

```
cargo run -p fd-conform -- code-writes [--cases <glob>] [--max-frames N]
  → one JSON line per case, then a total:
  {"case":"stretch-run","frames":n,"executed_bytes":n,"changed":[{"at":"h0+$1234","frame":n,"from":"4e75","to":"4e71"}]}
  {"total":{"cases":n,"frames":n,"changed":n}}
```

- Every case script under `conform/cases/` that starts at level 1 (the sync point or power-on) runs on the oracle within its own frame cap. Each frame, the T31 counter's newly executed instruction addresses join a set; their bytes (the instruction's full length, from the listing or from T01's export when present) are snapshotted when first executed and compared every frame after.
- A change is reported, never fixed or ignored. A write restored within one frame is not seen; the report says so.

## Tests

- [ ] On one short level 1 case (smoke, a few hundred frames): runs, reports `executed_bytes` > 0, and no change, or the change it finds is named in `FINDINGS.md`.
- [ ] A poke into an executed instruction's byte between frames (through the oracle's memory write, test only) is reported with its address and frame.
- [ ] Missing disks or Kickstart name the path and fail.

## Done when

- [ ] The full run over every level 1 case is done and its total line is in `FINDINGS.md`, dated.
- [ ] Tests pass and are labelled in `smoke.toml` (the full run is a tool run, not a test).
