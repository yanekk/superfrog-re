# T04 — judge-translated

**Phase:** 1 · **Depends on:** T00, T02 · **Weight:** medium

## Goal

Level 1's judge compares one port with the original, built in through `FD_PORT`, one build per routine. The trial judges a whole translated tree, any routine of it, by the same bar. This task adds that mode to `fd-judge`, so one build serves all 87 routines and the two stops are reported beside the verdict.

## Design sections this implements

DESIGN §2.2, §2.4, §2.5, §3.2, §7 (one build, not 87).

## Files

- `games/forest-dumb/engine/crates/fd-conform/build.rs` (a third built-in, `FD_TRANSLATED`, stubbed when unset)
- `games/forest-dumb/engine/crates/fd-conform/src/judge/translated.rs` (new: `Bus` for `Mem`, the `Cpu` ↔ `Regs` copy, running an entry)
- `games/forest-dumb/engine/crates/fd-conform/src/bin/fd-judge.rs` (`--translated`)
- `games/forest-dumb/engine/crates/fd-conform/Cargo.toml` (`pcode-rt`, on purity's `ALLOWED` if it needs listing)
- `games/forest-dumb/engine/crates/fd-conform/tests/judge_translated.rs` (new), `games/forest-dumb/smoke.toml`

## Interface

```
FD_TRANSLATED=<build/translator/tree.rs> fd-judge 'hN+$off' --gen <name> --translated [--cases N]
  → the existing line plus "untranslated":n,"write_into_code":n,"unsupported":n,"stopped_hangs":n,"first_stop":".."
```

- The state's `Cpu` is copied into `Regs` (d0-d7, a0-a7), the entry runs with `Image { bases: &HUNK_BASES, code: CODE }`, and `Regs` is copied back. Comparison is exactly the existing one (registers, bytes written off the stack, custom writes in order), so `pass` means what it means for Muse's ports.
- A `Stop` fails that state and is counted by kind; `first_stop` names the first.
- The translated tree's top-level return: the entry is called as a subroutine with the rig's return address on the stack, as the original is.
- Without `FD_TRANSLATED` the stub tree has no entries and every routine reports `untranslated`: the judge's negative check.

## Tests

- [ ] With the stub, `--translated` fails every state and says `untranslated`.
- [ ] `translated.rs` exposes the judging as a library function taking the entry function and `CODE` as arguments, so tests pass hand-written functions without `FD_TRANSLATED`. A hand-written `pcode-rt` function that does what a small game routine does (one already passing as a Muse port, e.g. `h0+$5462`) passes, and the same with one constant flipped fails, naming the first differing value.
- [ ] A tree whose function writes into `CODE` reports `write_into_code` and fails.
- [ ] The old `FD_PORT` path and `tests/judge.rs` are unchanged and pass.

## Done when

- [ ] `cargo test -q -p fd-conform --test judge_translated --test judge --test purity` passes; labelled in `smoke.toml`.
- [ ] `docs/11-translation.md` gains a short section on `--translated` beside the service it extends.
