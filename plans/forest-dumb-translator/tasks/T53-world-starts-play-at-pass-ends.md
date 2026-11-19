# T53 — world-starts-play-at-pass-ends

**Split from:** T51 (owner, 2026-11-19); carries T51's "Five cases pass" for `world-2-start`, `world-3-start` and `world-4-start`.

**Phase:** 5 · **Depends on:** T51, T52; blocks T42 · **Weight:** medium

## Goal

Make `world-2-start`, `world-3-start` and `world-4-start` pass with `--engine translated` by applying the owner's existing pass-end rule for level play (DESIGN §2.7, "At the pass end, with a pass of slack", T30) unchanged to play in the world starts, with no new allowance. The load and the OS's stretch after it stay as T51 left them (owner, T51: lined up as part of the load), and so do T51's two frame-end memory allowances before play.

What T51 measured (2026-11-19, on T49's machine with T51's runner): `world-5-start` and `world-6-start` pass. The other three fail only in play, on memory judged at a frame's end that falls inside `game_frame` (docs/09: the original's `game_frame` returns after the frame's end on almost every moving frame); Paula's writes are equal in all three, the picture too.

| Case | Frames failing in play | First difference |
|---|---|---|
| `world-2-start` | 1 | `h0+$2c7d` at 6154 (pass 449), `h0_2d4c`'s slot counter, engine 1, original 2 |
| `world-3-start` | 18 | `h0+$2c7d` at 6180 (pass 454), engine 2, original 1; then a creature blit on screen (`h3+$e84`, 6183) and `BLTCON0=$cfca` a pass apart at 6183 |
| `world-4-start` | 1 | `h0+$1e61` (`char_y`'s low byte) at 6132 (pass 439), engine $a4, original $a5 |

World 3's blit is T52's kind (the original's CPU slower than the machine's while bobs are cut), so this task follows T52.

In play (the program has the machine: `level3_handler` in `$6C`, and passes end with `PASS_END`) judge the hunks' memory as `run_frames_inputs` judges a level case's state: at each pass end, paired by `irq_count`, a pass allowed to end `frames::PASS_LAG` frames apart. The picture stays per frame and Paula's writes in order, as the bar has them. Before play (the SCENA screen, the title) memory stays judged at frame ends with T51's allowances.

## Design sections this implements

DESIGN §2.7 (the pass-end rule, unchanged), success criteria ("each of worlds 2 to 6 matches the original over its first frames").

## Files

- `crates/fd-conform/src/frames.rs` (`run_entry_frames`, `Judge`): memory at pass ends in play; `TranslatedEngine::pass_ends` already holds RAM at each `PASS_END` write, the oracle's `watch_write`/`watched_memory` likewise.
- `engine/conform/cases/world-2-start.toml`, `world-3-start.toml`, `world-4-start.toml`: notes only, the `Fails today` comment replaced in each case that passes; a case's checks are never loosened.
- `docs/14-translator.md`, "Worlds 2 to 6": the new reading, dated.

## Interface

```
cargo run -p fd-conform -- run cases/world-2-start.toml --engine translated
cargo test -q --release -p fd-conform --test world_starts -- --ignored
```

## Tests

- [ ] A stand-in test (as `tests/power_on.rs`'s or `tests/world_starts.rs`'s) whose engine ends a play frame mid-pass a few instructions apart from the original and passes, and one whose state at a pass end differs and fails naming the pass.
- [ ] Full: `world_starts.rs`'s `world_N_start_passes_on_the_translated_program`, N 2 to 6, pass; the stand-ins and `power_on.rs`'s still pass.

## Done when

- [ ] Worlds 2, 3 and 4 pass their first 300 frames of play after the load with `--engine translated`, and 5 and 6 still do.
- [ ] No allowance beyond DESIGN §2.7's pass-end rule and T51's; any other difference is measured and taken to the owner.
- [ ] The reading in `docs/14-translator.md` is updated, and the `Fails today` comment is gone from every case that passes.
