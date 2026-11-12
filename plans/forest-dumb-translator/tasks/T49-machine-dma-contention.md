# T49 — machine-dma-contention

**Split from:** T46 (owner, 2026-11-12); carries T46's "Every case above passes" for its four cases below.

**Phase:** 5 · **Depends on:** T46; blocks T36 · **Weight:** heavy

## Goal

Model DMA bus contention on `pcode-machine` (owner, 2026-11-12, T46), so the CPU loses the bus slots the chips take, and make T46's four cases pass on the translated program. DESIGN §2.7 declined contention in T30 as large and uncertain; T46 measured that all four of its cases fail on it and nothing else:

- `character-run-slide-buttons`, pass 79 (oracle frame 6481): the pass started at the line 233 interrupt reads JOY1DAT at line 300 on the machine and after the frame's end (past line 313, the next step) on the original, so the machine takes up a frame late (`joy_up` 0, then `char_x` `$1f3` against `$1f0`). The original's read slid past the frame's end at step 6478; the machine's stayed at lines 298 to 300.
- `character-edge-pit`, pass 289 (frame 6691): the same, the machine reading at line 308, the original past 313; the release is seen a frame late (`joy_up` 1 against 0).
- `camera-right-left`, `camera-bottom-right`, pass 93 (frame 6395): docs/10's "A creature start the next pass cuts": the original's edge redraw scan is cut by the next pass before it starts the creature (`h0_681c`); the machine finishes it and starts the creature a pass early (`creature_slots+$24` `$3ff02` against `$3ff01`, `+$20` one step on).

Model what takes the CPU's bus on the A500 (OCS): bitplane fetches (the planes BPLCON0 enables, between DDFSTRT and DDFSTOP on the lines DIWSTRT/DIWSTOP open), the copper, sprites, disk and audio slots, and the blitter (its cycles and BLTPRI); the 68000's accesses on even cycles. Judge it against the oracle by measurement (the beam at a CPU read or write, as `amiga-oracle`'s line samples or `watch_write` show), never by tuning to a case. Every fix is general (DESIGN §2.2); a difference the bar admits is accepted only with its measurement in the case's notes (DESIGN §2.7). When this lands, re-measure what T30's pass slack (`frames::PASS_LAG`, `custom_carried`) still has to admit and record it; changing that rule is the owner's.

## Design sections this implements

DESIGN §2.2, §2.7, §2.8.

## Files

- `crates/pcode-machine` (a contention model beside `beam.rs`, `blitter.rs`, `copper.rs`; its README); `crates/fd-translated` if the cycle count it hands the bus changes.
- DESIGN §2.7's paragraph on contention, through the owner. Case files only for notes; a case's checks are never loosened.

## Interface

```
cargo run -p fd-conform -- run cases/<case>.toml --engine translated
cargo run -p fd-conform -- run-all --engine translated --report ../build/translated-cases.md
```

## Tests

- [ ] Machine unit tests: a CPU loop timed against the beam with 0, 4, 5 and 6 lowres planes, with the copper on, and beside a running blit, each value measured on the oracle first.
- [ ] A test on the original's and the machine's beam at `read_joystick`'s JOY1DAT read over a scrolling stretch (`character-run-slide-buttons`), equal to the line the bar needs.
- [ ] Full: T46's four cases pass with `--engine translated`, and T30's 19 still do (`run-all --engine translated` over `camera-*`, `character-*`, `stretch-*`, `picture-*`, `w1-idle`, `stub-level`).

## Done when

- [ ] Every case above passes; any acceptance is in the case's notes with its measurement.
- [ ] The routine gate on the machine (`translator_trial.py --whole --machine`) is no worse than before.
- [ ] If the model needs more than a session, the worker asks to add a task rather than stretching this one.
