# T25 — machine-sound-input

**Phase:** 4 · **Depends on:** T22 · **Weight:** medium

## Goal

Paula and the CIAs on the machine: Paula's writes collected per frame in order (for the bar, and for `amiga-paula` in the app), audio interrupts as the program enables them; CIA-A and CIA-B as T01 found the program uses them (fire buttons, keyboard serial data and handshake, timers, disk lines), and the joystick counters, fed from a per-frame input.

## Design sections this implements

DESIGN §2.7 (Paula's writes in order), §2.8.

## Files

- `crates/pcode-machine/src/{paula,cia,input}.rs` (new). The keyboard's serial path is `amiga-input`'s `CiaKeyboard` and the input types its `FrameInput`/`Joy`, extended there (naming no game) if they lack something; Paula's writes use `amiga-paula`'s `PaulaWrite` (plan review, 2026-11-11).

## Interface

```rust
pub struct Input { pub joy1: u16 /* JOY1DAT */, pub fire: bool, pub keys: Vec<u8> /* raw codes this frame */, pub mouse_left: bool }
impl Machine {
    pub fn set_input(&mut self, i: &Input);
    pub fn paula_writes(&self) -> &[(u64, u16, u16)];   // this frame, in order
}
```

The input shape follows `fd_core::FrameInput` where they overlap, so the app and the cases feed both engines the same way.

## Tests

- [ ] JOY1DAT and fire read back as set; a key code arrives through CIA-A SDR with its interrupt, and the handshake is honoured.
- [ ] Audio register writes appear in order with their cycles; an AUDxINT is raised when the program enables it and a channel's period expires (only if T01 shows the program uses audio interrupts).
- [ ] CIA timers count from cycles if the program uses them.

## Done when

- [ ] Tests pass without disks; README lists what of the CIAs is modelled, from T01.
