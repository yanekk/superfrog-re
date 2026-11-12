# fd-translated

The translated program as an engine (translator DESIGN §2.14, §3.3; T28): the tree
`tools/pcode_rust.py` generates, built in at compile time, run on `pcode-machine` a frame at a
time. Pure (in `fd-conform/tests/purity.rs`'s `PURE`).

## The tree

`buildscripts/fd_translated.rs` (outside this folder: the purity test scans every `.rs` in it)
includes `FD_TRANSLATED` if set, else `games/forest-dumb/build/translator/tree/` if it holds a
`mod.rs`, else a stub with no entries. Make the tree with
`python3 -I games/forest-dumb/tools/translate.py --build`. `TranslatedGame::generated()` says which.
A checkout with no `games/forest-dumb/build/` at all does not notice a first tree until
`cargo clean -p fd-translated` (Cargo would rebuild everything each time if it watched a missing path).

## Calling it

```rust
use fd_translated::{Control, Start, TranslatedGame, RUN_STACK};
let mut g = TranslatedGame::new(Start::PowerOn { program: &bytes, disks: Disks { df0: disk2 } });
// or Start::Snapshot { mem: &chip_then_slow, regs: Regs68k { .. }, custom: &custom, disks }
g.held_at_start(&snapshot_frames_input);   // a snapshot: the keys its keyboard already sent
g.set_input(first_frame_input);
// on a thread with RUN_STACK bytes of stack:
let stop = g.run(&mut |frame| {
    // frame.picture, frame.writes (cycle, reg, word; CPU and copper), frame.cpu_writes (the CPU's
    // alone), frame.paula, frame.chip, frame.slow
    Control::Next(next_input)          // or Control::Stop
});
```

- `run` calls `each` at every frame boundary, from inside the bus's `poll` at the first block past
  `Machine::frame_end()`, interrupt handlers included: the host runs handlers itself
  (`Machine::enter_interrupt` / `leave_interrupt`) so their blocks reach its `poll`.
  It resets `Regs::steps` each frame and sets CIA-A's interrupts into INTREQ (PORTS).
- It returns `None` when `each` said stop (final: the frames it was inside are gone), or the
  program's stop, after one last `each` with `stop` set and the last whole frame's picture.
- **Power-on** enters `h0+$0` with the stack at `POWER_ON_STACK`. The program waits for fire at the
  disk 2 prompt (`h0+$7b70`): hold it in the first frames' input.
- **Power-on** runs in user mode on `POWER_ON_STACK`, its supervisor stack at
  `POWER_ON_SUPERVISOR_STACK` (the top of slow RAM, where Kickstart 1.3 keeps it), which the first
  interrupt it takes switches to. `g.start_at_line(line)` before the run puts the beam on `line`, so
  the first frame is only the rest of it (the line the original's entry ran on, T32).
- **Watched returns** (T32): `g.watch_returns(&[(hunk, offset), ..])` before the run, and each frame's
  `returns` lists the watched routines that returned in it, `inside` those still running at its end.
  Read off `Regs::depth` at each block's `poll`: entered through the routine's entry only (a mount
  hides it); a function entered at its depth also means it returned, as a call is not a block's end.
- **Snapshot**: the original's RAM, registers and custom registers (as `Machine::from_snapshot`
  reads them); CIA-A's mask is set to the keyboard's (`$08`), which the oracle does not expose. The PC
  and every return address on the stack may be any instruction: the run enters through the tree's
  `resume` (every instruction an entry, `pcode_rust.py`) and, whenever a function returns with no
  Rust caller, resumes at the address its `rts` or `rte` popped.
- `TranslatedGame::with_program(Program { .. }, bases, start)` runs a made-up translated program
  (tests).

## The seam

A hand-written routine runs in place of the translated one at its address (translator DESIGN §2.9,
T35). Every generated function asks `Bus::mounted` on entry; `TranslatedGame::new` mounts each
`fd_core::PORTED` entry marked `mounted`, with its adapter in `src/seam.rs` (`seam::ADAPTERS`; a
mounted entry with no adapter stops the run at start, naming it). `g.mount((hunk, offset), f)` mounts
by hand (tests). An adapter reads `LAYOUT` and the values beside it through `seam::BusMem`, runs the
port, writes back what differs (`fd_core::WriteBack`) and leaves by `pcode_rt::rts`. It counts no
cycles. Judge one before mounting it: `cargo run -p fd-conform --bin fd-judge -- 'h0+$4d7e' --seam`.

## Shells

The game drives, so a shell that pulls a step runs `run` on its own thread and passes frames and
inputs over a channel: `fd-conform`'s `translated_engine` (`Engine` and `Transplant` for a case),
later `fd-app`.
