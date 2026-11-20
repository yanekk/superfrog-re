---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - tools/smoke.sh
---

# Forest Dumb Forever, the translator — Design

## 1. Purpose

Stages 1 to 7 of `games/forest-dumb/docs/translator-brief.md`, after the trial (stage 0, `plans/forest-dumb-translator-trial/`, 84 of 85 judgeable routines with no hand edit, owner's go 2026-11-07). A static translator turns the whole of `FOREST_DUMB_FOREVER` into Rust, instruction for instruction, with no model per routine. The translated program, run on the toolkit's chip models, is the starting engine for everything after level 1; hand-written Rust replaces translated routines where the owner chooses, each checked against the routine it replaces; and the translator becomes the first pass any session uses to turn the original's code into Rust, in place of Muse.

### Success criteria

- Every instruction kind the program uses (the census, T04) matches Moira on generated states: d0-d7, a0-a7, X N Z V C, memory written, cycles (T13).
- The translated program, from the level 1 sync point and from power-on, passes the level 1 cases the hand engine passes, and each of worlds 2 to 6 matches the original over its first frames (T30 to T33).
- The app runs the translated program from power-on with sound, level 1's hand engine mounted through the seam, proven by `fd-e2e` (T40).
- A fresh session given a porting-style task and the repo's docs only runs `tools/translate.py` unprompted, 3 runs of 3 (T21).

### Stance

- Option B (owner, 2026-11-06): translated first, hand-written where chosen. For this game from level 2 on it amends `plans/rust-engine/DESIGN.md` §7; Superfrog's engine is unchanged.
- The shipped app holds no emulator: the translated code is compiled Rust, the chips are the toolkit's models, and the app never links `fd-conform` or `amiga-oracle`.
- The faithful bar is unchanged (`knowledge/self-testing-faithful-bar.md`), judged by machine on the oracle.
- Ghidra is reused for decoding, analysis and P-code before anything it does is rewritten; its decompiled C is not a source.
- Translated code never guesses: an address with no translation, or a write into a translated instruction, stops the run naming the address.

---

## 2. Behaviour specification

### 2.1 What is translated

The whole main program: every instruction reachable from the program's entry `h0+$0`, every routine in `re/symbols.toml`, the interrupt handlers the program installs (`level3_handler`, `level2_handler`), hunk 4 (the music player), the targets of every jump table, and every address the original was seen executing on the oracle (`re/entries.toml`, §2.3). Not the boot block, the crack's programs or Kickstart, which are out of scope (§8); the OS calls the program makes are answered by the machine (§2.8).

### 2.2 Pass, and the hand-edit rule

Carried from the trial: a result passes only with no hand edit. A hand edit is a change to generated Rust, a special case for one address in the translator or runtime, or a hint in `re/translator-hints.toml` (owner, 2026-11-07). General fixes, by mnemonic or P-code pattern, are the work and are not hand edits; every one re-runs the whole check it affects. Ghidra 12.0's known defects (`abcd`/`sbcd`, X on compares, a `bra.w` marked `CALL_RETURN`, the empty relocation table) are corrected this way. The reason is that a hint or an address special case would have to be found again for every later level and every later game; a pattern fix is paid once.

### 2.3 Finding the code

Entries come from four sources, each recorded with its source in `re/entries.toml` (addresses and source names only, never listing text, so it is committed like `translate.toml`): the program entry, the symbols, the vectors the program writes (read from its own code by the export), and the oracle trace (T02), which records every instruction address the original executed across level 1's cases, the power-on run through the title and the world 2 to 6 starts. An oracle-traced address is a measurement, not a hint, because it names where the original actually ran; it does not count as a hand edit. A jump table whose targets neither the analysis nor the trace finds needs a hint, which counts as a hand edit and is reported.

### 2.4 The stops

- `untranslated: hN+$off`: control reaches an address with no translation.
- `write-into-code: hN+$off`: a store lands on a byte of a translated instruction; nothing is written.
- `unsupported: <op> at hN+$off`: an instruction the translator cannot emit; it costs that instruction's routines, not the run.
- `hang`: the global step cap (counted across calls, not per call) or the call-depth cap is reached. The trial's per-call counter let endless recursion overflow the Rust stack (trial FINDINGS); a global count and a depth cap make it a stop like any other.

A stop ends the run. In the judge it fails that state; in a case it fails the case naming the frame; in the app it is §2.10.

### 2.5 Writes into code (`h0_6852`)

The stop stays: translated code cannot follow a write into its own instructions. A judged state on which the original also writes into one of its executed instruction bytes is outside the game's reach (the generator gave a value the game never does) and is reported as `self-modifying` and left out of the routine's count, with the count shown. The trial's `h0_6852` is exactly that case (slot `d7` up to 33 where the game gives 0 to 4). On the game's own paths `fd-conform code-writes` must still report 0 changed instructions, now over level 1's cases, the power-on run and the world 2 to 6 starts (T15). The reason: a generated state is not a game state, and counting the original's own self-modification against the translator charges a gap in the generator to it.

### 2.6 Instructions, condition codes and cycles

Each instruction kind of the census (mnemonic, size, source and destination addressing mode) is compared with Moira on generated states: registers, X N Z V C, every byte written, cycles. Cycles are counted per instruction in the generated code, the fixed part as a constant and the data-dependent part (shifts by register, `mulu`/`muls`, `divu`/`divs`, `dbcc`, `movem`, taken or not-taken branches) by a runtime helper using Moira's rules. Bus contention with DMA is not modelled, as in Moira's routine mode; the frame bar (§2.7) decides whether that matters.

### 2.7 Timing: the frame bar (owner, 2026-11-11)

The bar is per frame: at the end of every frame the game's state (the hunks' memory and the chip RAM the game owns), the picture and the custom-chip writes of the frame, Paula's in order, equal the original's. Inside a frame, timing only has to be good enough to keep those equal; a difference that never shows at a frame boundary is not chased. Each acceptance of a difference is written into the case's notes with the measurement on the original that justifies it, as level 1 did. Cycles are counted (§2.6) because the game waits on the beam (`wait_line_1`, raster waits) and on the blitter, and the beam position comes from counted cycles. Level 1's report asked that this rule be the owner's before the build, so that sub-frame timing is not settled case by case.

**At the pass end, with a pass of slack (owner, 2026-11-12, T30).** The original's `game_frame` returns after the frame's end on almost every moving frame (docs/09, 2 977 of 3 000), and `pcode-machine` had no DMA contention then, so a frame's end found the two sides at different points of the same pass. The bar's state and writes are therefore judged at each pass end, where the CPU writes `INTREQ=$4010` (`PASS_END`), paired by `irq_count`; the writes are cut into passes at that write in each side's own stream. A pass may end one frame apart on the two sides (`frames::PASS_LAG`), and a non-Paula custom write may fall one pass early or late (`main_loop`'s blits are cut by the pass end wherever they have got to), its counts agreeing again by the next pass end. Paula's writes stay in order, pass by pass. The picture stays per frame. Modelling DMA contention was the other option, declined then as large and uncertain; T49 (owner, 2026-11-12) modelled it on `pcode-machine` (2026-11-18), and this bar stands as set: with `PASS_LAG` 0, 9 of the 24 level cases still end a pass a frame apart, both ways.

**Cases lined up by loads (owner, 2026-11-19, T51).** From a load's return until the program takes the machine back (`level3_handler` in `$6C`), no frame is compared and the taking back counts as a return (`frames::OsTail`): the original's OS runs its interrupts there, about 5% of the CPU, which the machine never does (§2.8). `beam_sample_unread` is left out of memory (`sync::BEAM_VALUES`). A byte differing at a frame's end passes when one side's values over three frames are the other's a frame on (`frames::one_frame_on`); a one-frame glitch that reverts still fails.

### 2.8 The machine

The translated program runs on a machine that gives it what the Amiga gave the original:

- Memory as the oracle's A500 lays it out (chip RAM and slow RAM), the five hunks at `HUNK_BASES`, data loaded from the program file and the disks at start. No data is compiled into the code (§2.11).
- The custom registers, with the blitter (`amiga-chips` `Blit`, busy for `word_times` cycles), the copper run per frame and the picture (`amiga-chips` `copper`, `display`, `sprite`), Paula's writes (`amiga-paula`), CIA-A and CIA-B as the program reads and writes them (joystick fire, keyboard, timers and disk lines as T01 finds them), and VPOSR/VHPOSR from the cycle count.
- Interrupts: INTENA/INTREQ as on the 68000. A pending enabled interrupt is taken at the next instruction boundary of the generated code, by calling the translated handler with the 68000 exception frame pushed, so `rte` returns into the interrupted code. Every instruction after a block's first starts with `insn_start` (`Bus::insn`); the machine answers from a cheap test (`Machine::quiet`: below its next possible interrupt cycle, nothing pending above SR's mask) and polls only past it (owner, 2026-11-19, T50). Block granularity, the planner's choice, failed the frame bar: `route-W1-1` (pass 1981) and `route-both-W1-1` (pass 1976) took line 233's interrupt after `h0_6852`'s creature start, one block, where the original takes it at `$6858`, so the creature started a pass early.
- The OS: every library call and OS structure the program uses, as T01 measures them (`OpenLibrary`, the `dos.library` file calls, what `system_on`/`system_off` hand over, the OS's interrupts and keyboard during the title), is answered by the machine; Kickstart's own code is never run or translated. Files are read from the disk images by `amiga-disk`'s OFS reader. Loads finish at once (owner, 2026-11-11), because waiting for a pretend floppy buys the player nothing.
- Disk 2's prompt does not appear: both images are present, so the first open succeeds, as on an Amiga with disk 2 in the drive.

### 2.9 The seam

A hand-written routine replaces a translated one by address. The list is `fd-core/src/ported.rs` (`PORTED`), extended with whether an entry is mounted. A mounted routine is entered through an adapter that copies the variables it reads from the machine's memory into the hand engine's typed state (`LAYOUT`, `Transplant`), runs it, and writes what it changed back (a new write-back, the reverse of `Transplant`). A replacement is checked against the translated routine it replaces on the routine oracle, on the same states, and the cases stay green with it mounted. Level 1's hand engine is kept and mounted this way as the first user (owner, 2026-11-11). Nothing else is rewritten in this plan; each later level plan asks the owner (owner, 2026-11-11).

The mechanism (T35): every generated function asks the bus on entry whether a routine is mounted at its address (`Bus::mounted`), so a mount catches a `bsr`, a table `jsr` and a jump into the entry alike; `fd-translated`'s `Seam` answers from `PORTED`'s `mounted` entries, each with an adapter in `seam.rs` that leaves as `rts` does. `fd-judge --seam hN+$off` holds an adapter to the translated routine on its generator's states: bytes written off the stack, custom writes in order, the return, and registers only when named, because ports keep no scratch registers and every caller of a mounted routine reloads or restores them.

The granularity (T00, 2026-11-11): both pass. Per routine, `char_sprite_position` and `camera_update` mounted alone or inside translated `game_frame` pass 2 000 of 2 000 with no registers compared. Per frame, `game_frame` (`h0+$1fec`) mounted whole (transplant, `game_frame_on`, write back what differs) passes 2 000 of 2 000, writing back 34.5 bytes a pass, 59 at most. Level 1 is mounted as the frame (T36), because the hand engine's routines share typed state across a pass that per-routine adapters would copy in and out at every call, and one mount is one thing to keep green. The seam stays by address, so a later level may mount single routines; the two routine adapters are the pattern and the seam's own check.

The mount's time (T55, 2026-11-19, owner): the frame mount spends the original's time on the machine; a mount that counts no cycles moved CLXDAT's read to the pass's start and gave the pass's time to `main_loop`. Each custom-chip access is made by the original's instruction at its place in the pass, ticked from `pcode_rt::cycles` (CLXDAT, JOY1DAT, the raster copy, the BBUSY wait and registers of every blit, every sound's line wait and registers); the rest of each callee is a lump of its cycles, measured once on the translated program over the level 1 cases with the waits for slots taken out, ticked a few accesses at a time through `poll` so contention, the frame's end and interrupts fall inside the pass. A rule is general, from the state the routine reads; no translated frame runs beside the hand engine (owner). T55's rule is a fixed cost a callee plus one a blit, and 39 of 50 cases pass mounted (death-spikes, which passed instantly mounted, draws a creature cut against the beam two lines early); where a routine's time varies with its state (`camera_update`, `h0_2fae`, `h0_2dd6`, the creature restore and cut, `char_update`) it misses by up to 7 000 cycles, enough to move JOY1DAT's read across a frame's end, so T56 times those routines path by path, each checked on the routine judge's states.

The mount's path (T56, owner: exact paths, 2026-11-19): a right total per routine is not enough. Held to each routine's exact cycle count on the translated program, only `contact-route` and `enemy-contact` of T55's eleven failures passed, because a bus wait falls on each instruction's place in the pass, not on the routine's total. So the translator emits one shape per instruction (`pcode_rt::shape`: its ticks, its `bus.sync` calls, whether it starts a block), and a mounted callee is walked (`seam::walk`): its timing rule (`seam::rules`) works out from the hand engine's typed state the decisions the original meets (each branch, `dbcc` count and computed call), and the walk makes each instruction's calls in the generated code's order, its custom-chip accesses for real. Each rule is held to the translated routine's counted cycles on the routine judge's states (`fd-conform/tests/mount_timing.rs`), the frame on `fd-judge --seam 'h0+$1fec'` (2 000 of 2 000). 18 of `game_frame`'s 20 callees are walked; `char_hit_check` and `char_update` stay a lump of their measured cycles (`frame.rs` `COSTS`) until T58, and a blit outside a walk stops the run. Measured 2026-11-19 (`fd-conform run-all --engine translated --mount level-1`): 47 of 50 cases pass mounted, against 39 with T55's rule; the mount's CLXDAT read lands on the translated program's line (`CLX_LINES_OFF` 0, from 2), though it is `char_hit_check`'s, because every routine before it is walked. The three left (`mount_level_1.rs` `T58`): `route-W1-1` and `route-both-W1-1` pass translated alone and fail mounted on the two lumps; `power-on-level-1` fails translated alone (T53).

### 2.10 The app

- It runs the translated program from power-on: the program's entry, the title, the menus, the levels (owner, 2026-11-11). `--level 1` starts at level 1, reached by running the translated program headless through the title with Start pressed, so the start is the program's own and not a separate path. The boot block and the crack are not run (§8).
- Level 1 runs with the hand engine mounted (§2.9).
- `--engine hand` starts today's hand engine instead, as before this plan (owner, plan review 2026-11-11): it keeps the two side by side for comparison, and lets the existing app tests run where no translation is generated.
- It plays sound: Paula's writes go through `amiga-paula` to `amiga-app`'s audio, as Superfrog's app does (owner, 2026-11-11).
- On a stop (§2.4) the game freezes, a line names the stop and its address, and a bug recording of the play from its start is saved to the bug folder automatically, in the existing bug zip format; the player can then quit (owner, 2026-11-11). The reason: the original would have carried on, so a stop is always a translator or machine defect, and the recording is what a session needs to fix it.
- Only the owner runs it (owner, 2026-11-11): the translated code is generated on the owner's machine from the owner's disks into `build/` and never committed or distributed. Built where no translation exists, the app says which command generates it and exits non-zero.

### 2.11 Generated code and the table bar (owner, 2026-11-11)

The generated crate holds instructions only. Every data byte, the program's own tables included, comes from the program file and the disks at start, as on the original. The 16-integer table bar and the purity rules (`fd-conform/tests/purity.rs`) are checked on the generated code too, whenever it exists (T34). The reason is that data in the code would be a second copy of the game's data with nothing checking it against the disk.

### 2.12 The translator as the default first pass (stage 7)

`tools/translate.py` (the name is taken over from Muse's tool, because every doc, pack and habit already points sessions there) is the one command: name a routine (`hN+$off` or a symbol) or a task, get the Rust path and one pass line from the judge. `games/forest-dumb/CLAUDE.md`, `docs/11-translation.md`, `tools/README.md` and every pack `tools/task_pack.py` writes say "translate with the translator first". A porting task becomes: check the translated routines on the oracle, then the owner's chosen rewrites, each starting from the translated routine and checked against it through the seam. This lands once the instructions and code-finding hold (T16), before the machine and the app (owner, 2026-11-11), because routine work gains from it at once.

### 2.13 Muse is replaced by Haiku, after a trial (owner, 2026-11-11)

- Muse no longer translates; `translate_batch.py`, Muse's `translate.py` and their tests are deleted, with the OpenRouter key handling and dollar caps. The ports Muse already wrote stay in `fd-core`. Revoking the OpenRouter key at the provider is the owner's.
- `tools/generate.py` (a model writes a routine's test-state generator, judged by branch coverage) and `tools/reshape.py` (a model drafts the readable rewrite of a routine the owner chose to hand-write, now from the translated routine) stay, switched to Haiku through the owner's Claude subscription (`claude -p --model haiku`, no tools), capped by a number of calls per run instead of dollars.
- Before either is the default, T19 reruns level 1's recorded sets on Haiku. Pass mark (owner, 2026-11-11): each of T61's 5 generators reaches at least as many of its routine's branches as Muse's did, and at least 20 of T62's 21 rewrites pass level 1's rig tests with no hand fix. Below it, the owner decides between no model and accepting Haiku; until it passes, sessions write generators by hand.
- The reason Haiku and not none: generators are needed for every routine a later level checks and every seam replacement, and Muse wrote them for cents where an Opus session costs about a dollar a routine.

### 2.14 The unhappy paths

- Ghidra, the disks, Kickstart or the export missing: every tool names the path it looked in and exits non-zero; tests that need them are in the full set and fail naming it, never skip.
- No generated tree: crates that include it build a stub with no entries (as `fd-conform`'s `FD_TRANSLATED` does today); every run on the stub stops `untranslated` at once, and the app says how to generate it (§2.10).
- A generated tree that does not compile: the tool reports the compiler's first error and the routine it came from and exits non-zero; output is never edited by hand.
- Two runs at once: the judge build is shared, so `build/translate/judge.lock` is taken, as the trial does.
- A case that fails only across a load: loads are instant (§2.8), so frames are re-aligned at each load's return (T32); a difference inside the realigned frames is a real failure.
- Haiku unavailable (not logged in, quota spent): `tools/model.py` returns `no-model` and the tool's result says so; nothing falls back to another model.
- The default check's sessions run away: each is capped by turns and wall time (§5.2).

---

## 3. Architecture

### 3.1 The boundary

```
pure    pcode-rt            P-code semantics, Bus trait, cycles, stops. No clock, I/O, floats, hash maps.
pure    pcode-machine       the machine of §2.8 over amiga-chips, amiga-paula, amiga-disk. Inputs: program bytes, disk images, input per frame.
pure    fd-translated       the generated tree (from build/), the engine wrapper, the seam.
pure    fd-core             level 1's hand engine, unchanged in kind.
shell   fd-app, fd-conform, fd-e2e, Ghidra export, Python tools.
```

`pcode-machine` and `fd-translated` join `PURE` in `fd-conform/tests/purity.rs` with `pcode-rt`; the generated tree is scanned by the same rules when it exists (§2.11). If the purity test fails, the fix is to move the code, never to relax the test. The reason: everything on the pure side can be stepped frame by frame from the oracle's state in tests, and the app links only pure crates plus its shell. `pcode-rt`, `pcode-machine`, `pcode_rust.py` and `ExportPcode.java` name no game; every game fact (hunk bases, entries, files) is input, so the later move to the toolkit is a file move (owner, 2026-11-11: game tier now).

### 3.2 Modules

| Part | Owns | Depends on |
|---|---|---|
| `tools/ghidra/ExportPcode.java`, `tools/pcode_export.py` | Whole-program export, seeded from `re/entries.toml` and symbols (T03) | Ghidra 12.0 |
| `tools/pcode_census.py` | The census of instruction kinds and their encodings (T04) | `pcode.json` |
| `tools/pcode_rust.py`, `tools/pcode_fix/` | Generated Rust, split in modules; corrections one file per instruction family (T04, T06) | standard library |
| `amiga-oracle` `Cpu68k::step` | One instruction on Moira: regs, SR, writes, cycles (T05) | Moira |
| `fd-judge --insn` | Per instruction kind, translated against Moira (T06) | `pcode-rt`, `amiga-oracle` |
| `crates/pcode-rt` | Bus, Regs, stops, global step and depth caps, cycle counter, interrupt poll hook | none |
| `crates/pcode-machine` | §2.8 | `pcode-rt`, `amiga-chips`, `amiga-paula`, `amiga-disk` |
| `crates/fd-translated` | `TranslatedGame`: build-time include of `build/translator/tree/`, frame stepping, `fd-conform::Engine` and `Transplant`, the seam (T28, T35) | `pcode-machine`, `fd-core` |
| `fd-conform` | Cases with `--engine translated`, power-on and later-world cases, Paula writes compared, code-writes over all paths | as today plus `fd-translated` |
| `fd-app` | Runs `TranslatedGame` from power-on, sound, the stop | `fd-translated`, `amiga-app` |
| `tools/translate.py` | Stage 7's one command (T18) | export, `pcode_rust.py`, `fd-judge` |
| `tools/model.py` | Haiku through `claude -p`, call cap, `calls.jsonl` (T17) | `claude` CLI |
| `tools/default_check.py` | Stage 7's check (T21) | `claude` CLI |

### 3.3 The decision function

Each generated function is `fn(r: &mut Regs, m: &mut dyn Bus, img: &Image) -> Result<(), Stop>`, a function of its arguments only. `TranslatedGame::run(each)` runs the machine and calls `each` at every frame boundary (beam at line 0 by counted cycles) with the picture, the frame's custom writes, Paula's writes and any stop, taking the next frame's input back. The engine drives, rather than being stepped, because the foreground is ordinary nested Rust calls that a pure crate cannot suspend; shells that need a step run it on a thread of their own (T28, T37). Everything the app and the cases need is a function of the start state and the inputs.

### 3.4 Data flow

Program file and symbols to Ghidra to `build/translator/pcode.json`; `pcode_rust.py` to `build/translator/tree/` (one Rust module per part, T04 sets the split); `fd-translated`'s build script includes it (`FD_TRANSLATED` names another). At run time the machine loads the hunks at `HUNK_BASES` and the disks, and starts at `h0+$0` (power-on) or from a snapshot of the oracle's memory and registers (a case's sync point).

### 3.5 Storage

Everything generated derives from the game's code and stays in `games/forest-dumb/build/` (git-ignored): `pcode.json`, `census.json`, `tree/`, judge and trial outputs, Haiku's `calls.jsonl`. Tools write through a temporary file and a rename; a crash leaves the previous file or none, and the next run overwrites. Committed: the tools, crates, tests on made-up programs, `re/entries.toml`, `re/os-calls.toml`, and reports holding names, addresses and verdicts only.

---

## 4. Testing

| Layer | Proves | Cannot prove |
|---|---|---|
| Unit tests in `pcode-rt`, `pcode-machine` | Each helper, register and chip effect on its edges | That Ghidra's P-code is right |
| Translator tests on made-up programs (committed bytes) | Emitted Rust compiles and runs to expected registers, memory, cycles | Anything about the game |
| Instruction check against Moira (T06 to T13, full) | Each census kind equal on generated states, all columns | Instructions the census does not hold |
| Routine judge, plain and on the machine (T16, T27, full) | Each routine equal on its generator's states | Branches no state reaches |
| Cases with the translated engine (T29 to T33, full) | The frame bar from the sync point and from power-on | Paths the cases do not take |
| `fd-e2e` (T40) and the drill (T41) | The real app through the real input path | What only the owner's hands reach (§5.1) |

Everything that needs Ghidra, the disks or the generated tree is in the full set. Every new test file gets a label in `games/forest-dumb/smoke.toml`; a crucial feature gets a smoke test that needs no disks and an `[[area]]`.

---

## 5. Environment — read this before running anything

| | |
|---|---|
| OS | macOS, arm64 |
| Language / runtime | Rust 1.99.0 pinned (`games/forest-dumb/engine/rust-toolchain.toml`); Python 3.15.1 in the repo's venv |
| Ghidra | 12.0 at `/opt/homebrew/Cellar/ghidra/12.0`, extensions as the trial recorded |
| Oracle | vAmiga at the pinned commit in `~/.cache/amiga-toolkit/vamiga/`, Kickstart 1.3 at `~/amiga-roms/kick1.3.rom`, the disk zips in the main checkout's `games/forest-dumb/` |
| Claude CLI | `claude` 2.1.322 at `~/.local/bin/claude`, logged in through claude.ai (`claude auth status`, 2026-11-11); `claude -p --model haiku` answered in 1.7 s, model `claude-haiku-5-5` |
| **Deliberately absent** | No 68000 assembler (test programs are bytes). No OpenRouter after T17. No Kickstart code is run or translated by the machine |

**The test command.** The block's lines come from `.pir/settings.json` and are not re-derived; this plan adds no install step (`claude` is already on the machine). `tools/smoke.sh` runs the smoke set; `~/.pir/superfrog-re/settings.json` gives it 120 minutes. The smoke set is already over its 15-minute budget: recent runs took 786 to 1 122 s (`~/.cache/superfrog-re/smoke-times.tsv`, 2026-11-11), so any task adding to smoke reads that file first, and translated-program tests go in full. While working: `cd games/forest-dumb/engine && cargo test -q -p <crate>`, and `venv/bin/python -m unittest discover -q -s games/forest-dumb/tools -p 'test_<x>*.py'`; drop `-q` for detail. Nothing forces colour here (trial, 2026-11-07).

**Dependencies.** None new on crates.io; the new crates depend only on workspace and toolkit crates. Python stays on the standard library. Anything else asks the owner first.

**End to end.** `fd-e2e` (`cargo test -p fd-e2e`, the toolkit's `amiga-e2e` underneath) drives the real app by script and by real key events, and captures frames, audio and windows. The translated app's tests join it (T40) at the sizes `fd-e2e` already uses; the backend is the owner's disks plus the generated tree, both on this machine, so they are in full, with a smoke test that runs the app on the stub tree and checks the "not generated" message.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| Haiku's trial result below the pass mark (T19) | The owner decides between no model and accepting it |

### 5.2 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| Global step cap and call-depth cap (`pcode-rt`) | T14 sets and records them | Endless loops and recursion stop as `hang` |
| Judge cases and wall-clock limit | 2 000 states, 120 s a run | Each judge run is bounded |
| Ghidra headless timeout | 1 800 s | An import never runs unbounded |
| Case frame caps | each case names its own | No oracle or translated run is unbounded |
| App `--max-frames`, `--timeout` | as `fd-app/src/args.rs` | Every scripted app run ends |
| `tools/model.py --max-calls` | 40 a run | Haiku spend per run is bounded |
| `tools/default_check.py` | 3 sessions, `--max-turns 15`, 10 minutes each | The stage 7 check cannot run away |
| `judge.lock` | always | One judge build at a time |

Never run the unbounded version of any of these to find something out.

### 5.3 Outside the code — who acts

| Action | Command (exact, wrapped) | Bin | Why this bin | Way back | Expected cost | Login check |
|---|---|---|---|---|---|---|
| Run Ghidra headless on the program | `python3 -I games/forest-dumb/tools/pcode_export.py` | `worker` | Local; writes only `build/translator/` | Delete `build/translator/` | none | none |
| Fetch a crates.io dependency already on the policy | `cargo build` | `worker` | Public read-only fetch | Revert the lockfile | none | none |
| Ask Haiku for a generator or a draft rewrite | `python3 -I games/forest-dumb/tools/generate.py …`, `… reshape.py …` (through `tools/model.py`) | `worker` | Subscription usage, capped at 40 calls a run; about $0.003 a call at list price | none needed, nothing outside changes | small share of the owner's subscription | `claude auth status` |
| Haiku trial batch (T19) | `python3 -I games/forest-dumb/tools/haiku_trial.py` | `worker` | 26 tasks, capped at 3 tries each | none needed | under 100 Haiku calls | `claude auth status` |
| Run the stage 7 check | `python3 -I games/forest-dumb/tools/default_check.py` | `worker` | Exception (owner, plan review 2026-11-11): moved down from `ask` by the owner's choice. Three full sessions on the default model through the owner's subscription, a larger share than a Haiku call, bounded by §5.2 | none needed | three short sessions, 15 turns each at most | `claude auth status` |

Nothing here touches a cloud account, a domain or a device. Revoking the OpenRouter key at the provider is the owner's own and is not a task.

---

## 6. Recovery

Nothing here can lock anyone out. A bad Ghidra project: delete `games/forest-dumb/build/translator/ghidra/` and rerun the export. A broken generated tree: delete `build/translator/tree/`; every crate falls back to the stub. Level 1's hand engine stays in `fd-core`; until T37 the app runs it as today.

---

## 7. Decisions and rationale

Owner's, 2026-11-11, unless marked.

- Stage 7 lands after instructions and code-finding hold (T16), before the machine and the app (§2.12).
- Muse is replaced by Haiku after a trial with the pass mark of §2.13; Muse's translate tools are deleted. Retiring all four tools was chosen first and changed the same day: Haiku keeps the two jobs a model is cheaper at.
- Only the owner runs a translated app (§2.10).
- Game tier now; `toolkit/PLAYBOOK.md` step 6 gains one `learned on:` line, "translate first, port by choice"; the toolkit move waits for a second game.
- Level 1's hand engine is kept and mounted through the seam (§2.9); nothing else is rewritten in this plan.
- The app starts at power-on, `--level 1` jumps (§2.10); it plays sound, pulled forward from the sound plan; a stop freezes, names the address and saves a bug zip.
- The frame bar (§2.7); instant loads (§2.8); data from disk with the table bar checked on generated code (§2.11).
- Planner: `re/entries.toml` from the oracle trace is a measurement, not a hint (§2.3), because it names where the original ran rather than a guess about the code.
- Planner: interrupts are taken at block boundaries (§2.8), because instruction granularity would cost a check after every instruction for a difference the frame bar admits. Reversed by the owner 2026-11-19 (T50): the frame bar did not admit it (`route-W1-1`, `route-both-W1-1`, a creature start one pass early); interrupts are taken at instruction boundaries, the check costing about 7% of a case's run.
- Planner: the OS is answered by the machine, not run (§2.8), because Kickstart is a separate program outside the brief, and the program uses a small, measurable set of calls (T01).
- Planner: corrections live in `tools/pcode_fix/`, one file per instruction family (T06), so the parallel flag and cycle tasks do not edit one file.
- Planner: the new command takes `translate.py`'s name (§2.12).
- Planner: the translated engine calls back per frame instead of being stepped (§3.3), because suspending nested Rust calls needs a thread, which the pure side forbids; the thread lives in the shells.
- Planner: no prototype; the app's picture is the original's, and the stop line is the only new thing on it.

---

## 8. Explicitly out of scope

- Levels 2 to 6 as levels, the title and menus as their own plan, the ending: each its own plan; this one only reaches their first frames, because they need their own cases and the owner's choice of rewrites.
- The boot block, the crack's programs and Kickstart's code (brief): the program starts at its entry with the machine answering the OS.
- Superfrog's engine, and Superfrog's cases as a second proving ground for the translator: a separate plan if the owner wants one.
- Map editing; an app for anyone but the owner.
- Sub-frame timing beyond the frame bar, unless a case fails on it. DMA bus contention is modelled since T49 (2026-11-18), disk and audio slots aside.
- Load times matching the floppy's.
