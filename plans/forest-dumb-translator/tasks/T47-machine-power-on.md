# T47 — machine-power-on

**Phase:** 5 · **Depends on:** T32; blocks T33 · **Weight:** medium

## Goal

Two machine gaps T32's power-on cases measured, each fixed as a general rule of `pcode-machine` (or `fd-translated`'s power-on start), never as a special case for one address (DESIGN §2.2). Added with the coordinator agent's yes for the owner, 2026-11-12.

1. **The display Kickstart hands over at power-on.** The disk 2 prompt (`prompt_disk_2`, `h0+$7b70`) is blank on the translated machine on every frame from 3463 to 3700 and 3889 to 3967 (power-on-title, `glitch`, about 1 800 pixels, the text missing): the program sets its copper list (`h1+$ca0`, COPJMP1) but does not touch DMACON until the entry takes the machine (`$87f0`), so it relies on the DMA Kickstart left on. The entry saves what Kickstart left: DMACONR `$03f0`, INTENAR `$602c`, ADKCONR `$11..` (oracle, T32; `h0+$1e3a..$1e3f`). The machine starts with DMACON 0. Model the handover: the registers as Kickstart leaves them, without taking the OS's interrupts the machine does not run. Whether INTENAR and ADKCONR come along, and what that does to `sync::OS_VALUES` (`h0+$1e3a`, 6 bytes), is this task's to decide and record.
2. **The bus value a read of a write-only register takes.** The 68000's `clr` reads before it writes, and a write-only custom register takes the word on the bus (`pcode_fix.arith`, FINDINGS). The original puts the prefetched instruction word there; the machine another. Measured (power-on-level-1, oracle frame 5001, `mt_end` leaving the title): `AUD0VOL` first written `$47e0` on the original, `$4279` on the machine, likewise `AUD1VOL..AUD3VOL`; and `clr.w $dff088` (COPJMP1) writes `$4eb9` once on the machine, never on the original. Match the original's rule by measurement on the oracle, in general.

Not in scope: the translated processor being faster than the original's while the program draws (no display time-stealing, no OS interrupt time; frames 3462 and 3965 to 3976, 5998 to 6057). That is the gap T30 asked the owner about as a change to the frame rule; the owner's answer decides whether and where it is modelled (FINDINGS, T32).

## Design sections this implements

DESIGN §2.6, §2.7, §2.8.

## Files

- `pcode-machine/src/` (the power-on custom registers; the bus word a write-only read takes), `fd-translated/src/lib.rs` (`Start::PowerOn`) as needed.
- Unit tests in `pcode-machine` for each rule.

## Interface

```
cargo run -q --release -p fd-conform -- run cases/power-on-title.toml --engine translated
cargo run -q --release -p fd-conform -- run cases/power-on-level-1.toml --engine translated
```

## Tests

- [ ] `pcode-machine` unit tests: the registers at power-on as Kickstart leaves them; a `clr.w` of a write-only register writes the word the original's bus holds (a made-up program, values measured on the oracle).
- [ ] T13's instruction gate and T27's machine judge unchanged (`insn_gate.rs`, `translator_trial.py --whole --machine`).

## Done when

- [ ] power-on-title's picture is equal on the prompt's frames, and neither case's first difference is a Paula or custom write of frame 5001.
- [ ] The two power-on cases' remaining differences are the display time-stealing frames (owner's call, T30) and play's (T30, T31) only, named in FINDINGS.
