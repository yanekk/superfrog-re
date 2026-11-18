# T50 — level1-cases-b-rest

**Split from:** T48 (owner, 2026-11-12); carries T48's "Every case above passes" for its fourteen cases below.

**Phase:** 5 · **Depends on:** T48, T49; blocks T36 · **Weight:** heavy

## Goal

Make all 18 of T48's level 1 cases pass on the translated program from the level sync point, as the hand engine does, once T49 models DMA contention on `pcode-machine` (owner, 2026-11-12, T48). T48 re-ran them after T30 and T31: death-countdown, death-esc, death-pit and level-end pass; the fourteen below fail, every first difference on sub-frame timing the machine without contention runs fast. Each case file holds its measurement in a `T48 (2026-11-12)` note.

- A creature start the next pass cuts (docs/10): the machine's edge-redraw scan starts a creature (`h0_681c`) before the line 233 pass that cuts the original's, so it runs a pass ahead. `collect-ledge`, `collect-return` (pass 58, `creature_active+$1`; machine frame 6359 line 226, original frame 6360 line 163), `death-water` (pass 131, `creature_active+$2`), `enemy-hop`, `enemy-kind4`, `mechanic-spring` (pass 93, `creature_slots+$20`; machine frame 6393 line 213, original frame 6394 line 141), `route-W1-1` (pass 101, BLTCON0 `$0fca` 5 against 4).
- The joystick read across the frame's end (T46's measurement, T49's second case kind): the script turns the stick on the step before each first difference, in `char_x`. `contact-route`, `enemy-contact` (pass 116), `edge-latch-route`, `enemy-route` (pass 124), `mechanic-vanish` (pass 118), `route-both-W1-1` (pass 142).
- A redraw the pass cuts short: `death-spikes`, the picture only at pass 1343 (oracle frame 7744), the status bar's lives, which the original's cut redraw has not read; state and writes equal on all 4 315 passes.

Re-run the 18 first, on T49's machine; fix what shows next. Every fix is general, in the translator, `pcode-rt` or the machine, never for one case or one address (DESIGN §2.2); a difference the frame bar admits is accepted only with its measurement written into the case's notes (DESIGN §2.7), under the owner's rule there. If contention as T49 lands leaves one of the kinds above, measure it on the oracle and ask the owner before changing the model or the bar.

## Design sections this implements

DESIGN §2.2, §2.7, §2.8.

## Files

- Whatever the failures lead to in `tools/pcode_fix/`, `crates/pcode-rt`, `crates/pcode-machine`; case notes for accepted differences, replacing T48's note in each case that passes. Each fix with a test that fails without it.
- Case files only for notes; a case's checks are never loosened.

## Interface

```
cargo run -p fd-conform -- run cases/<case>.toml --engine translated
cargo run -p fd-conform -- run-all --engine translated --report ../build/translated-cases.md
```

## Tests

- [ ] Each fix's own test (made-up program or machine unit test).
- [ ] Full: all 18 cases (collect-*, contact-route, death-*, edge-latch-route, enemy-*, level-end, mechanic-*, route-W1-1, route-both-W1-1) pass with `--engine translated`, with T45's custom-writes channel on.

## Done when

- [ ] All 18 cases pass; any acceptance is in the case's notes with its measurement, under the owner's frame rule (DESIGN §2.7).
- [ ] T48's `T48 (2026-11-12)` note is gone from every case that passes.
- [ ] If the failures need more than a session, the worker asks to add a task rather than stretching this one.
