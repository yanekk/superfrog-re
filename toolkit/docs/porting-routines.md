# Porting the original's routines: the rulebook

What the first game's port taught about turning 68k routines into Rust that matches the original, as
rules a porting task names in its Done when. It replaces reading the first game's whole findings log
before touching a mechanic. Like the playbook, each rule has one user so far: a second game that
confirms or breaks one corrects this file in the commit that shows it (root CLAUDE.md, rule 2).

The bar these rules serve is `knowledge/self-testing-faithful-bar.md`: state equal pass by pass, the
picture within tolerance, judged by machine on the oracle.

## 1. Before porting a routine

- A name, a comment or a note read from the listing is a reading, not evidence. Only a test that runs
  the original's code settles what it does. Readings were wrong often: a collision rule written in
  three places described a mask the engine never used, a comment had a sign backwards, and every
  level was drawn transposed until a review compared it with the data.
  - learned on: Superfrog. `plans/engine-architecture/` T08, T11, T12.
  - learned on: Forest Dumb Forever. `plans/forest-dumb-start/` T05 review.
- List where the original can be stopped partway through the routine before porting it: raster or
  beam waits, interrupts switched off and on, loops the main program leaves and re-enters on a later
  pass, values latched once and used later. The engine runs a routine whole; where the original can
  be interrupted in the middle, the two drift, and finding these one red case at a time was the
  largest single cost of the first port.
  - learned on: Superfrog. `plans/rust-engine/FINDINGS.md`, T101, T184, T185, T194-T197, T212.
- Decide whether the screen memory is game state. If objects or tile animations write into the
  display buffers and the next frame reads them back, the buffers are state and the original's blits
  must be ported; drawing from the map cannot match.
  - learned on: Superfrog. `plans/rust-engine/` T13, which forced T14-T18.
- Check the port register (section 4) for the routine's address before starting. A routine another
  task owns is called, never ported a second time.

## 2. Reading the machine code: the traps

Each of these cost at least one defect on the first game.

- Branch displacements are signed. Add them with `wrapping_add` on the address, or a backward branch
  panics in a debug build. Run the routine tests in debug, which is where the panic shows.
  - learned on: Superfrog. `plans/rust-engine/` T183.
- Respect the operation size. `.b` and `.w` touch only the low byte or word of a data register, and
  the rest keeps its old value. Sign-extend exactly where the 68000 does: `ext`; `moveq` (8 to 32 bits);
  absolute short addresses (`$8000.w` is `$ffff8000`); word index registers (`d8(An,Xn.w)`); branch
  and `d16` displacements; and any word operand to an address register. An address register is always
  written whole: `movea.w`, `adda.w`, `suba.w`, and `addq`/`subq` to it act on all 32 bits, and
  `cmpa.w` compares the sign-extended word.
- `clr` on the 68000 reads before it writes. On a write-only custom chip register that is two writes:
  first the word then on the data bus, usually the prefetch but not always (after audio DMA it was the
  DMA's word), then zero.
  - learned on: Superfrog. `plans/rust-engine/` T20, T107.
- `or` is not `add`, even where the operands never overlap in the test data. Port the operator the
  original uses.
  - learned on: Superfrog. `plans/rust-engine/` T56.
- A hardware read that clears or changes state (a keyboard byte, a collision register) is read the
  number of times the original reads it. A second read returns something else.
  - learned on: Superfrog. `plans/rust-engine/` T37, T191.
- A write to INTENA or INTREQ that raises the interrupt level is seen one instruction late: the CPU
  samples the level at one bus access inside each instruction, before that write lands, so the
  interrupt is taken after the next instruction. Changes to the status register's mask were not
  measured.
  - learned on: Superfrog. `plans/rust-engine/` T96, T194.
- A write through a null or stale pointer still lands somewhere in the original. Port where it lands,
  or name it as an exclusion; never drop it silently.
  - learned on: Superfrog. `plans/rust-engine/` T43, T44.
- A value of -1, or a flag that is set for one or two frames only, is not the event it resembles.
  Read what the original's own code tests, not what the value looks like.
  - learned on: Superfrog. `plans/rust-engine/` T104, T115, T132.
- A gained life is not a lost one. Any tool that watches lives compares the direction of the change.
  - learned on: Superfrog. `plans/rust-engine/` T205.
- Cycle costs, where the bar needs them at all, are counted from the instructions on each path, never
  fitted as one constant per routine and never fitted on a net total. Fitted totals hid opposite errors
  for weeks.
  - learned on: Superfrog. `plans/rust-engine/` T128, T129, T208.

## 3. Proving a port

- A routine test runs the original routine alone on the CPU (`amiga-oracle`'s `routine.rs`) over
  generated states and compares memory, the registers it returns with, and the hardware writes it
  makes. A test that compares memory only lets a wrong return value through.
  - learned on: Superfrog. `plans/rust-engine/` T73 review.
- The generated states reach every branch, the bounds included. A generator that never hit a table's
  last entry let an off-by-one through.
  - learned on: Superfrog. `plans/rust-engine/` T43.
- Every fix carries a test that turns red when the fix is removed, and the Done when says so. Fixes
  nothing defended were undone by later merges.
  - learned on: Superfrog. `plans/rust-engine/` T49, T190, T199.
- A routine test does not prove the routine runs in play. A routine is covered only when a level
  case shows the original running it and the engine matching through it. One enemy passed its routine
  test and never ran in any case.
  - learned on: Superfrog. `plans/rust-engine/` T41 review.

## 4. Cases and the port register

- A case fails when it compares nothing: no pass finished, no frame compared, no variable read. A case
  stayed green while the original never finished a pass.
  - learned on: Superfrog. `plans/rust-engine/` T85, T120.
- A case asserts its goal (the exit reached, the item taken, the next level entered), not only that
  both sides agree. Two sides that fail the same way agree.
  - learned on: Superfrog. `plans/rust-engine/` T149, T192.
- Every exclusion in a case names the task that removes it, and a test fails once the exclusion is no
  longer needed.
  - learned on: Superfrog. `plans/rust-engine/` T42.
- A game keeps one port register: every original routine address and object type, with the Rust
  function that ports it. A second entry for the same address fails a test. An object type the level
  can spawn that has no entry is reported as an event the cases fail on, never skipped. Parallel
  workers ported the same routine up to three times, and the dispatch quietly kept the last one; an
  unported object spawned in a level unseen.
  - learned on: Superfrog. `plans/rust-engine/` T41-T44, T201, T204.
- A judge rule (what counts as an item taken, a death, an exit) gets a unit test on the short-lived
  pattern that fooled it, and a judge change reruns every case and lists the ones whose milestones
  moved.
  - learned on: Superfrog. `plans/rust-engine/` T115.
- Probe a divergence from a sync point cut near it, not from power-on. A whole-game probe cost 20
  minutes; a short case from a cached sync point cost seconds.
  - learned on: Superfrog. `plans/rust-engine/` T203, T209.

## 5. Working beside other tasks

- Format only the files the task touched (`rustfmt <file>`), never `cargo fmt` over a crate.
  - learned on: Superfrog. `plans/toolkit-extract/` T10 reformatted 75 files; `plans/rust-engine/` T71.
- A tool that writes a dump, a trace or a recording has a size cap from its first commit.
  - learned on: Superfrog. `plans/rust-engine/` T05 wrote 38 GB and stopped the run.
- A crate created or moved carries the `opt-level` override its users need, in the root manifest of
  every workspace that builds it. A moved crate that lost it ran the soak at 31 frames a second with
  every hash equal.
  - learned on: Superfrog. `plans/toolkit-extract/` T07, found by T11.
- Time limits in tests are sized for a loaded machine, and a run is bounded by frames where it can be.
  - learned on: Superfrog. Escapes 20da6b74, 513851eb.
- A red test on the base branch is a task the day it is seen, not a finding. A red left for days hid
  newer reds behind it.
  - learned on: Superfrog. `plans/rust-engine/FINDINGS.md`, `the_frog_walks_step_counts...` red from
    T196 to the end of the run.
