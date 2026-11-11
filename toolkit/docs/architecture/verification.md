+++
id = "verification"
title = "Verification"
tier = "component"
depth = "light"
kind = "shell"
mix = { amiga = 10, game = 0, data = 0, shell = 90 }
reuse = "small-changes"
owns = [
  "toolkit/README.md",
  "toolkit/PLAYBOOK.md",
  "toolkit/template/",
  "toolkit/CLAUDE.md",
  "toolkit/docs/",
  "toolkit/test.sh",
  "toolkit/smoke.toml",
  "toolkit/Cargo.toml",
  "toolkit/Cargo.lock",
  "toolkit/rust-toolchain.toml",
  "toolkit/.gitignore",
  "toolkit/tools/",
  "toolkit/crates/toolkit-checks/",
  "toolkit/crates/dry-fit/",
  "toolkit/crates/amiga-oracle/",
  "toolkit/crates/amiga-conform/",
  "toolkit/crates/amiga-e2e/",
  "toolkit/crates/amiga-record/",
  "toolkit/crates/amiga-app/",
  "toolkit/crates/amiga-disk/",
]
+++

# Verification

## What it is

The machinery that judges an engine against the original, by machine. It also takes the toolkit's
files with no hardware chapter of their own, so that every toolkit file has a toolkit owner:
the workspace files, `toolkit/tools/`, `amiga-app` and `amiga-disk`. Those two crates are described in
`toolkit/README.md`.

- **`amiga-oracle`** (`toolkit/crates/amiga-oracle/`): the original, run on vAmiga (MPL-2.0) at a pinned
  commit (`VAMIGA_COMMIT`) with seven core patches (`patches/`), built outside the repo by
  `toolkit/tools/setup/oracle.sh` and linked through a C shim (`shim/shim.cpp`, `shim/shim.h`,
  `src/ffi.rs`). `Amiga` steps one PAL frame per `Amiga::step` with the engine's own `FrameInput` and
  returns a `FrameCapture` (the display window's picture as 12-bit colour, the Paula writes with their
  beam positions, the DAC taps). It snapshots and restores the whole machine, reads and pokes RAM, and
  reports the CPU's registers, interrupts taken, blitter events and every CPU custom-register write.
  `cached_trace` memoises a scripted run outside git. `Cpu68k` (`src/routine.rs`,
  `shim/moira_shim.cpp`) is the routine runner: Moira, vAmiga's 68000 core, compiled alone over a flat
  16 MiB memory with no chipset, which calls one routine of the original and returns its registers and
  every byte it changed; traced, also the branches it took and the bytes it read. A game passes in its disks, their names and its cache roots.
  Denise's three collision checks (sprite to sprite, sprite to playfield, playfield to playfield) are
  on, so CLXDAT reads what the hardware would; vAmiga leaves them off and CLXDAT then reads `$8000`
  whatever is drawn. A snapshot carries the switches, so the shim sets them again after every restore
  (`fixedSettings`), and `BUILD` ends in a settings tag (`-clx`) that every trace and sync cache key
  includes, so a cache made under other settings is never reused (`tests/collision.rs`).
- **`amiga-conform`** (`toolkit/crates/amiga-conform/`): the game-free half of a conformance harness:
  `compare` (picture and Paula writes, `Tolerance`, `PictureJudge`, `PassMatch`), `sound_events`,
  `channels`, `feed`, `report` (the diff PNG), `screenshot`, `script` (with a hook for a game's tokens).
  A game keeps its cases, memory map, levels, milestones and the run loop.
- **`amiga-e2e`** (`toolkit/crates/amiga-e2e/`): a game's real binary driven end to end, through its
  script layer (`App::run_script`) and an OS layer of real key events, menu clicks and window captures.
- **`amiga-record`** (`toolkit/crates/amiga-record/`): the game-free half of a bug recorder.
- **`toolkit-checks`** and **`dry-fit`**: the purity test, and one frame of a made-up game.

## Kind mix

`amiga` 10, `shell` 90, an estimate by reading. It is test tooling and a desktop shell. What counts as
`amiga` is the little that imitates hardware rather than driving the emulator that does:
`sound_events.rs` (Paula's latching: a sample's address and length as the DMA latches them at the
channel's start, period and volume as the frame leaves them), `Cpu68k`'s hardware mode (`set_hardware`,
`set_hardware_read`: it records custom writes, runs area-mode blits, reproduces a CPU read of a
write-only register as vAmiga does and steps `VHPOSR` so raster waits end), and `amiga-disk`'s planar
and palette readers.

## The checks

| Check | Compares |
|---|---|
| State, pass by pass (`compare::PassMatch`) | every compared variable at the end of each main-loop pass, the original's `n`th pass with the engine's `n`th |
| Picture within tolerance (`compare::PictureJudge`, `Tolerance::OWNER`) | the display window as 12-bit colour per pixel, each frame. A frame is tolerated when every differing pixel is at most one colour step off, or fewer than 64 differ. Any other difference may last at most 2 frames in a row and cover at most 1% of a case's frames. A differing window is never tolerated. Rows a draw raced the beam on can be excused (`compare::picture_except`) |
| Sound as events (`sound_events`) | per channel: sample started (address, length, period, volume), period or volume change, stopped, within ±2 frames |
| Plays alone | a game's milestones, in order, with the engine fed only the input and the original's pacing |
| Each channel's output (`channels`) | each channel's level frame by frame |
| Routine conformance (`Cpu68k`) | one ported routine against the original's on Moira, over many generated states |

The state, milestone and routine checks reach the engine through original addresses, the coupling in
[memory-model](memory-model.md). The picture check needs nothing Amiga-shaped; sound as events needs
Paula writes or an adapter.

## The vAmiga patches

Seven patches to the core at `VAMIGA_COMMIT`, applied by `toolkit/tools/setup/oracle.sh`; their SHA-1 is
part of the build path and of `amiga_oracle::BUILD`, so every trace and snapshot cache key changes with
them. Each hook is a null function pointer in the core that the shim installs; 0007 is a fix, not a hook.

| Patch | Hook or access | What it exposes | Why |
|---|---|---|---|
| `0001-custom-write-hook.patch` | `vamiga_custom_write_hook` in `Memory::pokeCustom16` | every custom-register write, CPU or copper, with the beam position, before it takes effect | Paula writes (`FrameCapture::paula`), beam anchors (`Amiga::blit_starts`), `Amiga::cpu_writes`, `Amiga::watch_write` |
| `0002-shim-access.patch` | the shim's access class a friend of `Emulator` | the emulator's private power-on and per-frame stepping | step one frame synchronously on the caller's thread, with no emulator thread or wall clock, so a run is deterministic and repeatable |
| `0003-keyboard-access.patch` | the shim's access class a friend of `Keyboard` | the key-down matrix | vAmiga snapshots do not carry the ports' and keyboard's held state; `seatInput` re-seats them after `Amiga::restore`, so a cached snapshot continues as the run that made it |
| `0004-audio-sample-hook.patch` | `vamiga_audio_sample_hook` in Paula's state machine (`penhi`, `penlo`) | each new DAC value per channel, with volume and beam position | `FrameCapture::taps`, the original's channel output |
| `0005-irq-entry-hook.patch` | `vamiga_irq_hook` in `Moira::willInterrupt`, `vamiga_blit_hook` in the blitter's begin, busy-clear and end | interrupts taken and blitter events with their beam position | `Amiga::irqs`, `Amiga::blit_events` |
| `0006-exec-hook.patch` | `vamiga_exec_hook` in `Moira::execute`, defined in `Moira.cpp` so the routine runner's private copy still links | each instruction the CPU executes, by its address | `Amiga::watch_pcs`, `Amiga::pc_hits`: proof a ported routine ran in play |
| `0007-das-table-once.patch` | `Sequencer::initDasEventTable` fills its table under a `std::once_flag` | nothing | the table of a line's disk, audio and sprite slots is one for every machine in the process and each set-up wiped and refilled it; a machine stepped on another thread meanwhile copied an empty line and ran a few instructions early. Machines may run on several threads only with this |

The shim adds what needs no patch: snapshots, RAM reads and pokes, the CPU's registers at the frame
boundary (`Amiga::cpu`), per-line CPU samples for profiling (`Amiga::line_samples`), and RAM copied at
the first CPU write of a value to a register (`Amiga::watch_write`). `Cpu68k` compiles a private copy of
Moira from the same source tree in its own namespace, with `shim/moira-stub/` standing in for the two
vAmiga headers Moira includes, because vAmiga's copy has its memory interface bound to the emulated
Amiga.

### Counting instructions run: `watch_pcs`, `pc_hits`

`Amiga::watch_pcs(&[u32])` replaces the set of watched instruction addresses (an empty slice turns it
off); after each step `Amiga::pc_hits()` gives `(pc, count)` for every watched address, ascending, zero
counts included. A case uses it to show a ported routine really ran in the original's play: the
per-line samples of `set_profile` miss short routines. The hook is called before each instruction's
handler, after the interrupt and STOP checks, so an instruction an interrupt pre-empts counts once,
when it runs after the return, and a STOPped CPU counts nothing. A step's counts include the start of
the next frame the step runs into, as `Amiga::irqs` does. The watched set is not in a snapshot; set it
again on a fresh machine before restoring. Counts are zero after `watch_pcs` or `restore` until the
next step. Checks: `toolkit/crates/amiga-oracle/tests/exec_counts.rs`, which also proves the run is
byte-identical with no list, an empty one and a full one.

Cost, measured 2026-11-05 on the Kickstart idle screen (3 300 frames a run, CPU time, best of seven
interleaved rounds on a machine busy with a parallel build, load 6 to 13): 611 frames a second before
the patch, 606 after it with no list, 562 with 200 addresses (8 % slower). The shim installs the core
hook only when a machine first watches, so a process that never watches pays one null check per
instruction. The owner accepted these busy-machine figures in place of an idle-machine one (2026-11-05;
the earlier 565 to 571 was measured at a game's sync point, a different workload).

A game may also patch the original in emulated RAM rather than the core, by `Amiga::poke`.

### Branch coverage of a routine: `set_trace`, `conditional_branches`

A generator of states for a routine is judged by which of the routine's branches its states take on
the original. `Cpu68k::set_trace(true)` makes each call record every conditional branch it executes
(`Bcc` other than `BRA`/`BSR`, and `DBcc`) with the ways it went, and every byte address it read in data
space; `Cpu68k::last_trace()` returns them. A branch is taken when the next instruction is not the one
after it, so a `DBcc` that loops is taken and its exit is not taken. The recording is in the routine
runner's own instruction loop and its memory reads, with no vAmiga patch (`0006-exec-hook.patch` is one
process-wide pointer, and runners run on several threads). Tracing changes nothing a call returns
(`tracing_changes_nothing_a_call_returns_and_is_off_by_default`).

Reads are the accesses Moira makes with the data function code: operands, the stack, custom-chip
registers. Instruction fetches and PC-relative operands (`move.w tab(pc),d0`, a `(pc,d0)` jump table)
use the program function code and are not listed.

`Cpu68k::conditional_branches(entry)` walks the code statically from `entry` with Moira's disassembler
for lengths: both ways of each conditional branch, `BRA`, and into and past each `BSR`, and each
`JSR`/`JMP` whose target is absolute or `(d16,pc)`. It stops at `RTS`, `RTE`, `RTR`, `ILLEGAL`. What it
misses: code reached only through a register (`jmp (a0)`, `jsr (a1,d0)`, a jump table, a pointer read
from memory), which shows up only once a traced call executes it; and it walks past a callee that never
returns as if it did, so it may list branches after such a call that cannot run. It does not tell code
from data, so a direct branch into data would decode the data; the walk stops after 2^20 instructions.

A game's routine rig builds its coverage report on these: the walk's branches plus every executed one,
each with how many states took it each way, one never executed at 0 and 0. learned on: Forest Dumb Forever's `Rig::coverage` (T60), which agreed exactly with a hand-written witness's 28 branches and minimum counts.

Cost, measured 2026-11-05 on an Apple-silicon Mac shared with a parallel build, release build, 2 000 states of two routines
through a game's rig, median of four runs. Before tracing existed a survey took 7.7 ms and 4.8 ms. With
the code in and recording off, 8.0 ms and 4.9 ms (about 3 % slower: one test per instruction and per
read). With recording on, 15.4 ms and 6.3 ms (1.9 and 1.3 times a survey with it off), the cost being
the per-instruction opcode test, the per-call branch table and the read bitmap. learned on: Forest Dumb Forever's `camera_update` and `h0_29ca`, `fd-conform/tests/rig_branches.rs`, `measure_the_cost_of_recording`.

## What must be preserved

| Part | Behaviour to keep | Check that proves it |
|---|---|---|
| The oracle's determinism (patch 0002) | The same inputs from the same start give the same frames | `a_disk_less_boot_reaches_the_insert_disk_screen_the_same_way_twice` (`toolkit/crates/amiga-oracle/tests/boot.rs`) |
| Snapshot restore | A restored snapshot continues as the run that made it | `a_snapshot_taken_mid_boot_continues_as_the_uninterrupted_run` (same file) |
| Instruction counts (patch 0006) | The hook only observes; a pre-empted instruction counts once; a restored run counts as the one that made the snapshot | `watching_does_not_change_the_run`, `an_instruction_an_interrupt_pre_empts_counts_once`, `a_restored_run_counts_as_the_run_that_took_the_snapshot` (`toolkit/crates/amiga-oracle/tests/exec_counts.rs`) |
| Machines on several threads (patch 0007) | Setting a machine up never wipes the slot table running machines share | `setting_a_machine_up_never_wipes_the_table_running_machines_share` (`toolkit/crates/amiga-oracle/tests/machines_apart.rs`) |
| `Cpu68k` | A routine touching the chipset is refused outside hardware mode; relocations, timeouts and exceptions behave | `custom_chip_and_cia_accesses_are_errors_with_the_pc`, `max_instr_stops_an_infinite_loop`, `relocation_points_an_absolute_reference_at_the_relocated_target` (`toolkit/crates/amiga-oracle/tests/routine.rs`) |
| `Cpu68k` tracing | Off by default and changes no result; branches and their ways, data reads only, the static walk's reach | `tracing_changes_nothing_a_call_returns_and_is_off_by_default`, `states_setting_one_condition_leave_exactly_the_others_uncovered`, `reads_are_data_space_only`, `the_walk_follows_direct_calls_and_not_indirect_jumps` (`toolkit/crates/amiga-oracle/tests/routine_branches.rs`) |
| Paula latching in `sound_events` | Writes reduce to the same events whatever their order in a frame; a sample's address and length latch at its start | `sound_events_from_writes`, `sound_events_reordered_in_a_frame_or_shifted_2_frames_match`, `sound_events_3_frames_apart_or_a_different_sample_fail`, `a_change_just_before_its_channel_stops_need_not_be_met` (`toolkit/crates/amiga-conform/tests/faithful.rs`) |
| The tolerance | The owner's numbers stay | `the_owners_tolerance_is_unchanged`, `the_owners_channel_tolerance_is_unchanged`, `the_glitch_budget_of_1_percent_fails_when_exceeded` |
| Pass matching | State compared by pass count, a pass a frame late on one side not a difference | `state_is_matched_by_pass_count` |
| Beam-race excusal | Only the given rows are excused | `picture_except_ignores_exactly_the_given_rows`, `race_rows_are_raster_lines_from_the_pictures_top` |
| Purity | The pure crates read no clock, file, network or random source | `toolkit/crates/toolkit-checks/tests/purity.rs` |

## Reuse

**`small-changes`**: the machinery carries; the content does not. The A500 configuration (OCS, 512K chip
plus 512K slow, Kickstart 1.3) suits most OCS games and is a change of a line for others; the core
patches name no game. A game writes its sync points, milestones, cases, scripts and run loop; the kinds
of sync point, the milestone list and the method of planning routes carry as methods.

## Sources

`toolkit/crates/amiga-oracle/`, `toolkit/crates/amiga-conform/`, `toolkit/crates/amiga-e2e/`,
`toolkit/README.md`.
