# T45 — custom-writes-compared

**Phase:** 5 · **Depends on:** T29; blocks T30, T31, T32 · **Weight:** medium

## Goal

The frame runner (`fd_conform::frames`, T29) compares every custom-chip write of each frame, not Paula's alone, as DESIGN §2.7's bar names them: "the custom-chip writes of the frame, Paula's in order". T29 compared Paula's only (its task's Files line named only Paula), found in T29's review; the owner approved this task on 2026-11-12 and settled its rule then:

- Paula's writes stay compared in order, as T29 does.
- Every other custom-chip write is compared per frame as a multiset of (register, value): the same writes, each as many times, order not checked, because §2.7 asks order only of Paula and the frame bar does not chase sub-frame timing.
- A first difference names the frame, the register, and both counts (the original's and the engine's) of the (register, value) that differs.
- An accepted difference is written into the case's notes with the measurement on the original that justifies it (§2.7), never as a runner special case.

## Design sections this implements

DESIGN §2.7.

## Files

- `fd-conform/src/frames.rs`: a fourth channel beside state, Paula's writes and the picture, with its `Outcome` (or an extended `WritesDiffer`), its own entry in `Verdict::firsts` under `Options::every_channel`, and its kind in `reading.rs` (`custom writes`).
- `fd-conform/src/translated_engine.rs`: the engine's custom writes of each frame through `FrameEngine` (a new method, e.g. `custom_writes()`).
- `pcode-machine` / `fd-translated` as needed. **Like with like:** the oracle keeps only the CPU's custom writes (`Amiga::cpu_writes`, "the Copper's are not kept"), while `pcode_machine::Machine::custom_writes` and `FrameOut::writes` hold the CPU's and the copper's together (read-only, T29 review). The engine side must hand over the CPU's writes alone, tagged at the source, not filtered by register.
- Paula's registers (`$96` DMACON, `$9A` INTENA, `$9C` INTREQ, `$9E` ADKCON, `$A0..=$DE` AUDx) are compared once: decide whether the new channel leaves them out or includes them, and say which in the README; T29's `compare::paula` already takes DMACON, INTENA and INTREQ writes as Paula's.
- `engine/conform/README.md`, `fd-conform/README.md`: the channel and the case key (`compare = [..., "custom"]` or folded into `"writes"`; pick one and say why).
- `tests/frames.rs`: extend the stand-in.

## Interface

```
fd-conform run cases/<case>.toml --engine translated      # now also judges the frame's other custom writes
fd-conform run-all --engine translated --report build/translated-cases.md
```

## Tests

- [ ] The stand-in (a copy of the original, `tests/frames.rs`) passes with every channel on; it fails when one non-Paula write of a frame is dropped, when one has its value changed, and **passes** when two non-Paula writes of a frame are swapped (order not checked); each failure names the frame and register.
- [ ] A stand-in that adds a copper-like write it did not get from the CPU fails, so the engine side cannot pass by mixing the copper's writes in.
- [ ] `--engine hand` unchanged (T29's `tests/engine_hand.rs`, full).

## Done when

- [ ] The frame runner compares the frame's non-Paula custom writes as a multiset and Paula's in order, every frame.
- [ ] `build/translated-cases.md` regenerated; FINDINGS gives the count of cases whose first difference is the new channel, and its commonest register.
- [ ] Before T30, T31 and T32 count a case as passing, it passes with this channel on.
