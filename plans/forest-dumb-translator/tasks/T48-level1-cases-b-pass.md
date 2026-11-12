# T48 — level1-cases-b-pass

**Phase:** 5 · **Depends on:** T30, T31; blocks T36 · **Weight:** heavy

## Goal

T31's finish line, carried (owner, 2026-11-12, T31 review): make the translated program pass T31's level 1 cases from the level sync point, as the hand engine does: collect-*, contact-route, death-*, edge-latch-route, enemy-*, level-end, mechanic-*, route-W1-1, route-both-W1-1 (in `engine/conform/cases/`, 18 cases).

T31 built and merged its share: sprite and playfield collisions on `pcode-machine` (`collide.rs`), the oracle's unread CLXDAT at a case's start (`Amiga::clxdat`), keys held at a snapshot not sent again. Re-run in T31's review, 0 of 18 cases pass and every first difference is one of T30's four defects (Paula writes one frame early 4, `char_x` 7, `char_y` 6, `edge_latch_left` 1); none is CLXDAT. This task starts once T30 has fixed those, re-runs the 18 cases and fixes what shows next. T31's builder saw later differences that look like read timing (the engine reading 5 to 10 lines early, the missing DMA contention).

The frame rule for timing differences is with the owner (T30's question). This task works under whatever the owner decides there and never settles that rule itself. Every fix is general, in the translator, `pcode-rt` or the machine, never for one case or one address (DESIGN §2.2); a difference the frame bar admits is accepted only with its measurement written into the case's notes (DESIGN §2.7).

## Design sections this implements

DESIGN §2.2, §2.7, §2.8.

## Files

- Whatever the failures lead to in `tools/pcode_fix/`, `crates/pcode-rt`, `crates/pcode-machine`; case notes for accepted differences. Each fix with a test that fails without it.
- Case files only for notes; a case's checks are never loosened.

## Interface

```
cargo run -p fd-conform -- run cases/<case>.toml --engine translated
cargo run -p fd-conform -- run-all --engine translated --report ../build/translated-cases.md
```

## Tests

- [ ] Each fix's own test (made-up program or machine unit test).
- [ ] Full: every case above passes with `--engine translated`, with T45's custom-writes channel on.

## Done when

- [ ] Every case above passes; any acceptance is in the case's notes with its measurement, under the owner's frame rule from T30.
- [ ] If the failures need more than a session, the worker asks to add a task (CLAUDE.md) rather than stretching this one.
