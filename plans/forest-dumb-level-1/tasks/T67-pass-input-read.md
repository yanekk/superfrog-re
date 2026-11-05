# T67 — pass-input-read

**Phase:** 2 · **Depends on:** T06 · **Weight:** small

Added by T41 with the owner's yes (2026-11-05). `run_with` (`fd-conform/src/lib.rs`) gives pass `n` the input of the step in which `irq_count` reached `n`. While the camera scrolls a pass runs past its frame's end, and the original's `read_joystick` (`h0+$8dfc`) then reads `JOY1DAT` in the next step: measured on the slide right from frame 6400, 57 of 76 passes whose start and end steps held different input read the end step's. So a case changing input while the camera scrolls fails on `joy_up` (T41's slide with up released at frame 6490, pass 89) though the engine is right. With the camera still, a pass starts and ends in one step and the pairing holds (T06's `input_timing.rs`). T42, T43, T44 and T15 wait for it (`blocks T42, T43, T44, T15`).

## Goal
Each pass the engine runs gets the input the original actually read for that pass, not the input of the step the pass started in. The comparison gets no looser anywhere: every variable, picture and pass compared today is still compared.

## Design sections this implements
§2.1, §2.12, §4.1.

## Reuses
T06's runner, T31's counter (`Amiga::watch_pcs`; one `read_joystick` hit a pass, so the step a hit falls in is the step whose input that pass read), T41's `character-run-slide` case and scripts.

## Files
`fd-conform/src/lib.rs` (`run_with`), `fd-conform/tests/` (the failing-without-it test), `engine/conform/cases/character-run-slide-buttons.toml` and its script, `engine/conform/README.md` ("How a level case compares"), `smoke.toml`.

## Interface
`run_with` unchanged in signature. Decide and record what a pass that runs only `play_keys` (paused, no `read_joystick`) and the keyboard (`play_keys` reads `last_key_byte` at its own time) get, measured, not assumed.

## Tests
- [ ] A slide case that changes buttons mid-scroll (up pressed and released while sliding right from frame 6400) passes on `Game` and fails with the runner's old pairing.
- [ ] `input_timing.rs` and T41's `character_run.rs` stay green; T06's shifted-input mutants still fail.

## Done when
- [ ] Each pass's input is the input of the step its `read_joystick` ran in, measured by the counter, and the README says so.
- [ ] The fix commit carries `Escape: forest-dumb-level-1 T06`.
- [ ] No case loosens: no exclusion added, no variable dropped.
- [ ] Smoke label for any new test file.
