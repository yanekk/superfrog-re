# T09 — fd-first-case

**Phase:** 5 · **Depends on:** T00, T02, T03, T08 · **Weight:** medium

## Goal

The game's conformance runner and the first case, from the stub in
`games/forest-dumb/conform/case-stub.txt`: boot the disk from power-on on the oracle and on our
side, feed both the same input frame by frame, and compare the one modelled variable at the idle
title screen, state only (DESIGN §2.8). When this passes, the machine has compared our code with
the original for the first time, through the same harness the whole rebuild will use.

## Design sections this implements

DESIGN §2.8, §3.2, §3.4, §5 (the seatbelts), §5.3.

## Files

- `games/forest-dumb/engine/crates/fd-conform/` — `Cargo.toml` (depends on `amiga-oracle`,
  `amiga-conform`, `amiga-disk`, `fd-core`, `fd-tools` by path), `src/lib.rs`, `src/main.rs`,
  `src/disks.rs` (the locate and unzip, on `fd-tools`), `tests/first_case.rs`, `tests/purity.rs`.
- `games/forest-dumb/engine/conform/README.md` — how to run it, the game's own runner shape.
- `games/forest-dumb/engine/conform/cases/first-case.toml` — the case.
- `games/forest-dumb/engine/conform/scripts/boot-to-title.txt` — the sync script: power-on to
  the title with the input T00 measured (the crack intro's click, the wait), in amiga-conform's
  script format.
- `games/forest-dumb/test.sh` — completed: the engine workspace plus the Python tests, then the
  case test runs inside `cargo test` like the other real-disk tests (it needs the disks and the
  Kickstart on this machine, and fails loudly naming where it looked when they are absent).
- `games/forest-dumb/CLAUDE.md` — the conform runner and the case in the index.
- `README.md` (root) — the sentence on game two's start and the playbook, the monorepo report's
  noted follow-up, plus the link.

## Interface

The case file follows Superfrog's case format (the stub points at it): `sync = { script =
"boot-to-title", frame = <T00's frame> }`, `frames = <N idle>`, `compare = ["vars"]`, `vars =
["<the variable>"]`. The runner:

```
cargo run -p fd-conform -- run cases/first-case.toml   # one case, prints the verdict
```

## Tests

- [ ] `first_case.rs` runs the comparison in `cargo test`: the engine's variable equals the
      oracle's at the layout address for every compared pass of the idle title, or the failure
      names the pass and the two values.
- [ ] The workspace purity test (`tests/purity.rs`, Superfrog's shape): `fd-core`'s dependencies
      on the allowlist, no toolkit crate depends on a game crate.
- [ ] A missing disk or Kickstart fails with the path it looked at, not a panic.

## Done when

- [ ] `cargo run -p fd-conform -- run cases/first-case.toml` exits 0 and prints the comparison
      verdict, and the same comparison passes inside `cargo test`.
- [ ] `sh games/forest-dumb/test.sh` is green end to end (the worker runs it; the end gate
      re-runs it), and the root README and the game's CLAUDE.md carry the new pieces.
- [ ] Nothing derived is committed; the case and its script are committed input files, as the
      stub says they must be.

## Outside actions

- Fetch and build vAmiga — `worker` (DESIGN §5.3, already built on this machine).
