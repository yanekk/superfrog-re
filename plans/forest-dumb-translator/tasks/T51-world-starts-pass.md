# T51 — world-starts-pass

**Split from:** T33 (owner, 2026-11-18, T33 review); carries T33's "Five cases pass" and its full test "each world's case passes its first 300 frames of play after the load with `--engine translated`".

**Phase:** 5 · **Depends on:** T33, T49; blocks T42 · **Weight:** medium

## Goal

Make the five world starts pass on the translated program once T49 models DMA contention on `pcode-machine`. T33 built the cases (`world_index` set as `load_world` is entered on both sides, `set` with `when`) and measured them, 2026-11-18: in each, the 13 loads line up and `world_index` is the world's on both sides; the first difference is the processor running fast while the SCENA screen is built, the same cause as `power-on-level-1` (T32, T47). Each case file holds its measurement in its header comment.

| Case | First difference (`h0+$7a9d`) | Copper start, picture | Paula |
|---|---|---|---|
| `world-2-start` | 5977 (engine 0, original 8) | 6034, 6036 | 6183 |
| `world-3-start` | 5998 | 6054, 6056 | 6203 |
| `world-4-start` | 5951 | 6008, 6010 | 6153 |
| `world-5-start` | 5843 | 5899, 5901 | 6043 |
| `world-6-start` | 5837 | 5892, 5894 | 6033 |

Re-run the five first, on T49's machine; fix what shows next. Every fix is general, in the translator, `pcode-rt` or the machine, never for one case, one world or one address (DESIGN §2.2); a difference the frame bar admits is accepted only with its measurement written into the case's notes (DESIGN §2.7), under the owner's rule there. If contention as T49 lands leaves a kind above, measure it on the oracle and ask the owner before changing the model or the bar.

T33's review left one claim read-only: that no world start needs a state besides `world_index` (`session_loop` doing what `next_world` does before `level_init`). Check it against `next_world`'s listing; a state it sets that a world reached by play would carry is set by the same `set` with `when`, written in the case.

## Design sections this implements

DESIGN §2.1, §2.2, §2.7, success criteria ("each of worlds 2 to 6 matches the original over its first frames").

## Files

- Whatever the failures lead to in `tools/pcode_fix/`, `crates/pcode-rt`, `crates/pcode-machine`; each fix with a test that fails without it.
- `engine/conform/cases/world-2-start.toml` … `world-6-start.toml`: notes only, replacing the `Fails today` comment in each case that passes; a case's checks are never loosened.
- `docs/14-translator.md`, "Worlds 2 to 6": the new reading, dated.

## Interface

```
cargo run -p fd-conform -- run cases/world-2-start.toml --engine translated
cargo test -q --release -p fd-conform --test world_starts -- --ignored
```

## Tests

- [ ] Each fix's own test (made-up program or machine unit test).
- [ ] Full: `world_starts.rs`'s `world_N_start_passes_on_the_translated_program`, N 2 to 6, pass; the stand-ins and the 300-pass test still pass.

## Done when

- [ ] Five cases pass their first 300 frames of play after the load with `--engine translated`; any acceptance is in the case's notes with its measurement, under the owner's frame rule (DESIGN §2.7).
- [ ] The `next_world` claim is checked against the listing, and any state it needs is set in the cases.
- [ ] The reading in `docs/14-translator.md` is updated, and the `Fails today` comment is gone from every case that passes.
- [ ] If the failures need more than a session, the worker asks to add a task rather than stretching this one.
