---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - tools/smoke.sh
---

# Forest Dumb Forever, translator trial — Design

## 1. Purpose

Stage 0 of the translator brief (`games/forest-dumb/docs/translator-brief.md`). A static translator reads the main program `FOREST_DUMB_FOREVER` and writes Rust that runs it instruction for instruction, with no model in the loop per routine. This plan builds its first version, runs it over `game_frame`'s call tree, judges every routine on the routine oracle, and puts the result in front of the owner, who says go or no-go for the rest of the translator (stages 1 to 6, a second plan). The trial goes first because everything after it rests on whether translation works, and the judge, the generators and the inventory it needs already exist.

### Success criteria

- Every routine of `re/translate.toml` (87, `game_frame`'s tree and `redraw_edge_a..d`'s) has a row in the trial table: judged, or named unjudgeable with its reason.
- The table states, per routine: pass or fail with no hand edit, unresolved jumps, writes into code, whether it would pass with hints, and Muse's batch result beside it.
- A run of level 1's cases on the original reports whether any write lands in an instruction the game executed.
- The owner has seen the table and the gate reading, and the answer is in `FINDINGS.md` with its date.

### Stance

- The owner chose translated-first, hand-written where chosen (option B, 2026-11-06), for this game from level 2 on. Superfrog's engine is unchanged.
- No model translates a routine. A per-instruction translator's cost is paid once per kind of instruction, not per routine, which is the reason the brief exists.
- Ghidra is reused before anything it already does is written: its 68000 decoding, its analysis and its P-code, the description of each instruction's effect its own emulator runs (owner, 2026-11-06). Ghidra's decompiled C is not a source, because it drops exact flags, cycle counts and the order of hardware writes and is not meant to compile.

---

## 2. Behaviour specification

### 2.1 What is translated

The routines of `games/forest-dumb/re/translate.toml`, each with every callee reached by direct branch or call, the same 87 Muse was given in level 1's T40, including the four `too-big`, the three `blocked` and `game_frame` itself, because a translator has no size limit. Sound routines are translated too; only judging them is impossible (§2.4).

### 2.2 Pass, and the hand-edit rule

A routine passes when the judge (`fd-judge`, level 1's T33) finds the translated tree, entered at that routine, equal to the original on all of its generator's states (2 000): registers d0-d7 and a0-a6, every byte written off the stack, every custom-chip write in order. This is the bar Muse's ports met, so the two columns compare one for one. Condition codes and cycles are not compared in the trial; cycle counting is stage 1's.

A routine passes only with no hand edit (owner, 2026-11-07). A hand edit is any change to the generated Rust, any special case for one address in the translator or runtime, or any hint (§2.3). General fixes to the translator or runtime are the point of the trial and are not hand edits; every one re-runs every routine.

### 2.3 Hints

A hint is a line in `games/forest-dumb/re/translator-hints.toml` stating one fact about one address that the analysis could not find: a jump table's bounds, bytes that are data, a computed target. A routine that needs a hint does not pass for the gate (owner, 2026-11-07), because the gate tests the translator, and every hint would be needed again in every later level and every later game. The trial runs twice, without and with the hints file, so the table shows which routines would pass with hints. The file holds addresses and facts only, never listing text, as `translate.toml` does.

### 2.4 Unjudgeable routines

A routine with no generator (the two sound routines, level 1 being silent) or one the original never returns from in the judge's hardware mode (`no-oracle`, as `translate.py` defines it) cannot be judged by anyone. It is listed by name with its reason and left out of the gate's count (owner, 2026-11-07), because counting it a failure would charge a gap in the checker to the translator.

### 2.5 The two stops

Translated code never guesses. When control reaches an address with no translation (a computed jump to an unknown target, a return into data), it stops with `untranslated: hN+$off`. When it writes into a byte of a translated instruction, it stops with `write-into-code: hN+$off`. Either stop fails that state, and the table counts them per routine. They exist because a silently wrong path is the failure the faithful bar cannot afford, and stage 2 builds on exactly these two reports.

### 2.6 Writes into code on the original

Separately from the judge, level 1's conformance cases are run on the oracle and every frame the bytes of instructions the game has executed (level 1's T31 instruction-address counter) are compared with their loaded values. Any change is reported by address and frame. The judge only sees generated states; this check sees the game's real path through level 1. It misses a write restored within one frame, which is stated in the report.

### 2.7 The gate

Go when at least 90% of the judgeable routines pass with no hand edit, and §2.6 finds no write into executed code on level 1's path (brief, kept by the owner 2026-11-07). Muse's column is information and not part of the gate. The go is the owner's: T08 shows the table and the reading and records the answer. On a no-go the translator and the report still merge (planner's assumption, confirmed by the owner with the requirements, 2026-11-07), so what was measured is not lost; the second plan is then not written.

### 2.8 The unhappy paths

- Ghidra missing or its analysis failing: the export names the path it looked in and exits non-zero; tests that need Ghidra are in the full set and fail naming it, never skip.
- Disks, Kickstart or listing missing: the tools name the path, as `translate.py` does.
- An opcode or P-code op the translator does not know: translation of that instruction emits a stop naming it (`unsupported: <op> at hN+$off`) rather than failing the whole tree, so one rare instruction costs its routines, not the run. The table counts these.
- A judge build failing on generated code: the trial tool reports the compiler's first error and the routine it came from, and the run ends non-zero. A translator bug is never worked round by editing output.
- Two trial runs at once: they share `build/translator/` and the judge build, so the tool takes the existing `judge.lock` and refuses a second run.
- Level 1's branch moving after T00: ignored. The trial runs on the copy T00 brought in.

---

## 3. Architecture

### 3.1 The boundary

```
pcode-rt (new crate, pure)    P-code semantics over a register file and a Bus trait. No clock, I/O, floats, hash maps or network.
pcode_rust.py (new tool)      P-code export in, Rust source out. A pure function of its input file; touches only the paths it is given.
ExportPcode.java, fd-judge, trial tool, code-writes check    the shell: Ghidra, the oracle, the file system.
```

`pcode-rt` joins `PURE` in `fd-conform/tests/purity.rs`, because the second plan links it into the app; if that test fails, the fix is to move the code, never to relax the test. Generated code is compiled only into `fd-judge` in this plan; how the table bar (no committed tables over 16 integers) applies to generated code that the app later links is the second plan's to settle with the owner.

The translator's instruction and runtime parts name no game (brief; root CLAUDE.md "Before building something"): `pcode-rt`, `pcode_rust.py` and `ExportPcode.java` take every game fact (hunk bases, entry points, hints) as input. They are built in the game's folder because this game is their only user, and the move to the toolkit is a file move when a second game uses them.

### 3.2 Modules

| Part | Owns | Depends on |
|---|---|---|
| `games/forest-dumb/tools/ghidra/ExportPcode.java` (T01) | Per instruction: hunk-relative address, length, mnemonic, P-code ops, flow targets; relocated constants marked hunk-relative | Ghidra's API |
| `games/forest-dumb/tools/pcode_export.py` (T01) | Imports the program through `toolkit/tools/ghidra_headless.py` with `re/symbols.toml`, seeds the entries from `re/translate.toml`, reads the file's `HUNK_RELOC32` entries and hands them to the script, writes `build/translator/pcode.json` | `ghidra_headless.py` (extended to take an extra script) |
| `crates/pcode-rt` (T02) | `Bus` trait, `Regs` (Ghidra's 68000 register space), P-code op helpers, `Stop` | none |
| `games/forest-dumb/tools/pcode_rust.py` (T03) | `pcode.json` to one Rust module, one function per entry | standard library |
| `fd-judge --translated` (T04) | Builds `build/translator/tree.rs` in (`FD_TRANSLATED`), implements `Bus` for `judge::Mem`, judges an entry | `pcode-rt`, level 1's judge and generators |
| `crates/fd-conform/tests/code_writes.rs` and its runner (T05) | §2.6 | `amiga-oracle`, level 1's cases and T31 counter |
| `games/forest-dumb/tools/translator_trial.py` (T06) | Runs export, translate, build, judge for all routines twice (§2.3), writes `build/translator/trial.json` and `table.md` | the above, `build/translate/batch.json` |

### 3.3 The decision function

Each generated function is `fn hN_off(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop>` (or generic over `B: Bus`; T03 chooses by build time): a function of the registers and memory it is given and nothing else. Control flow inside a routine is a loop over a block label; a direct call is a Rust call; a computed jump or call dispatches over the table of translated entries and returns `Stop::Untranslated` otherwise. Flags live in `Regs`, because 68000 code returns them across `rts` and callers branch on them.

### 3.4 Data flow

`FOREST_DUMB_FOREVER` (from `fd-tools extract`) and `re/symbols.toml` go to Ghidra, which writes `build/translator/pcode.json`. `pcode_rust.py` writes `build/translator/tree.rs`. `fd-judge` builds it in and judges each routine with its generator. `translator_trial.py` collects the lines into the table, reading Muse's results from `build/translate/batch.json`.

Ghidra loads the hunks at its own addresses, while the oracle loads them at `HUNK_BASES` (`fd-conform/src/lib.rs`). The export therefore writes every address as `hN+$off`, and every operand the program file's `HUNK_RELOC32` entries relocate as a hunk-relative reference. The relocations are read from the file, not from Ghidra, whose Amiga loader records none (measured at plan review, 2026-11-07: 0 recorded against 4 254 in the file; owner chose reading the file, 2026-11-07, because it is exact and keeps the code independent of the load address). The generated code resolves those through bases given at run time, so it is independent of where the program is loaded, which the second plan needs anyway.

### 3.5 Storage

Everything generated derives from the game's code and lives in `games/forest-dumb/build/translator/` (git-ignored): `pcode.json`, `tree.rs`, `trial.json`, `table.md`, the Ghidra project. A crash mid-run leaves partial files that the next run overwrites; the table is written last, through a temporary file and a rename. What is committed: the tools, `pcode-rt`, the judge mode, tests on made-up programs, `re/translator-hints.toml`, and the report with its table (`docs/13-translator-trial.md`), which holds routine names, addresses and verdicts only, as `docs/11-translation.md`'s batch table does.

---

## 4. Testing

| Layer | Proves | Cannot prove |
|---|---|---|
| `pcode-rt` unit tests | Each P-code op helper on its edge cases (carry, signed overflow, sizes 1/2/4) | That Ghidra's P-code for an instruction is right |
| `pcode_rust.py` tests on made-up programs | The emitted Rust compiles and runs to the expected registers and memory | Anything about the game |
| Export test (full set, needs Ghidra) | A hand-written 68000 program exports with the right addresses, flow and relocations | The game's own analysis |
| Judge on generators (T06, T07) | The translated routine equals the original on its 2 000 states | Branches no state reaches; condition codes; cycles |
| Code-writes check (T05) | No executed instruction byte changes across level 1's cases, frame by frame | A write restored within a frame; paths the cases do not take; instructions the listing does not know, which are not watched |

Test programs are written for the tests, never taken from the game, so they can be committed. Every new test file gets a label in `games/forest-dumb/smoke.toml`; Ghidra and oracle runs over a few seconds go in full.

---

## 5. Environment — read this before running anything

| | |
|---|---|
| OS | macOS, arm64 |
| Language / runtime | Rust 1.99.0 pinned (`games/forest-dumb/engine/rust-toolchain.toml`); Python 3.15 in the repo's venv |
| Ghidra | 12.0 at `/opt/homebrew/Cellar/ghidra/12.0` (`ghidraRun` on the PATH), JDK from `~/Library/ghidra/ghidra_12.0_PUBLIC/java_home.save`, `ghidra-amiga` and `GhidraMCP` extensions installed (checked 2026-11-07) |
| Oracle | vAmiga at the pinned commit in `~/.cache/amiga-toolkit/vamiga/`, Kickstart 1.3 at `~/amiga-roms/kick1.3.rom`, the disk zips in the main checkout's `games/forest-dumb/` |
| **Deliberately absent** | No 68000 assembler; test programs are written as bytes. No paid service: the translator calls no model |

**The test command.** The block's lines come from `.pir/settings.json` and are not re-derived; the plan adds no install step. `tools/smoke.sh` runs the smoke set within 900 s (`tools/smoke.py`); recent passing runs took 503 to 904 s (`~/.cache/superfrog-re/smoke-times.tsv`), so read that file before adding anything slow. While working: `cd games/forest-dumb/engine && cargo test -q -p <crate>`, and `venv/bin/python -m unittest discover -q -s games/forest-dumb/tools -p 'test_pcode*.py'`; verbose by dropping `-q`. Nothing here forces colour (`env` checked 2026-11-07: only `COLORTERM`, which the test tools ignore); failures print in full. `games/forest-dumb/test.sh` discovers Python tests in `tools/` only, which is why the translator's Python lives there.

**Measured at plan time (2026-11-07).** Ghidra headless imports and analyses `FOREST_DUMB_FOREVER` and a post-script reads P-code per instruction, flags included (registers `0x44` to `0x47`). Left to its own analysis Ghidra disassembles only 3 837 instructions of the program, so the export must seed entries from `re/symbols.toml` and `re/translate.toml`. Ghidra loads hunk 0 at `$21f000`; the oracle at `$c077d8`.

**Dependencies.** None new. `pcode-rt` has no dependencies; `pcode_rust.py` uses the standard library only. Anything else asks the owner first.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| Go or no-go on the trial (T08) | The owner decides whether the rest of the translator is built |

### 5.2 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| Ghidra headless timeout | 1 800 s (`ghidra_headless.py`) | An import never runs unbounded |
| Judge cases | 2 000 a routine | Each judge run is bounded |
| Judge wall-clock limit | 120 s a run (`translate.py`'s `JUDGE_SECONDS`), raised by T06 if a routine with no `Hang` stop needs longer, the value in `FINDINGS.md` | A translated tree that loops on many states fails that routine as `timeout` instead of running for hours |
| Translated-code step cap | 10 000 000 blocks a call | A translated loop that never ends stops as `Stop::Hang`, as the original's hang is counted |
| Oracle frame cap | Each case names its own | No emulator run is unbounded |
| `judge.lock` | always | One judge build at a time across processes |

Never run the unbounded version of any of these to find something out.

### 5.3 Outside the code — who acts

| Action | Command (exact, wrapped) | Bin | Why this bin | Way back | Expected cost | Login check |
|---|---|---|---|---|---|---|
| Bring level 1's branch into the plan branch (T00) | `git merge --no-edit pir/forest-dumb-level-1` | `worker` | Local history only; the owner approved it at planning (2026-11-07) | `git reset --hard ORIG_HEAD` on the task branch before it is merged | none | none |
| Fetch a crates.io dependency already on the policy | `cargo build` | `worker` | Public read-only fetch | Revert the lockfile | none | none |
| Run Ghidra headless on the program | `python3 -I games/forest-dumb/tools/pcode_export.py` | `worker` | Local; writes only `build/translator/` | Delete `build/translator/` | none | none |

Nothing here touches a cloud account, a paid service, a domain or a device; no credential is needed. The owner confirmed all three rows in `worker` at plan review (2026-11-07).

---

## 6. Recovery

Nothing here can lock anyone out. A bad Ghidra project: delete `games/forest-dumb/build/translator/ghidra/` and rerun the export. A broken judge build: unset `FD_TRANSLATED` and the stub builds.

---

## 7. Decisions and rationale

All by the owner on 2026-11-07 unless marked.

- The trial is its own plan, and stages 1 to 6 are a second plan written after the go and after level 1 merges, so the later stages are planned on what the trial measured rather than on guesses.
- The trial runs now rather than after level 1. It needs level 1's judge, generators and inventory, so T00 brings level 1's branch in as it stands; this branch is merged into `main` only after level 1 has merged, because it carries level 1's unfinished work. Copying the files instead was rejected as a second copy that drifts.
- The gate is the brief's (90%, no writes into code on the level's path); Muse's column is information only.
- A hint counts as a hand edit (§2.3).
- Unjudgeable routines are left out of the gate's count and listed (§2.4).
- On a no-go the work still merges (§2.7).
- Each follow-up session of T07's fixing needs the owner's own yes, not the coordinator agent's, shown with the pass rate so far (owner at plan review, 2026-11-07), because the number of failure kinds is unknown and the cost should not grow unseen.
- Planner: translator in Python, runtime in Rust, export in Java. The export has to be a Ghidra script; the translator is text generation with no Rust dependency to approve (reading JSON in Rust would need `serde_json`, outside the policy); the runtime is Rust because the generated code is.
- Planner: the judge is extended with a mode that builds the whole translated tree once, rather than one build per routine through `FD_PORT`, because 87 builds of the judge would dominate a run and the tree is one program.
- Planner: generated code addresses memory through hunk-relative references resolved at run time (§3.4), because Ghidra's load addresses are not the oracle's and the later app needs the same independence.
- Relocations come from the program file's `HUNK_RELOC32` entries, not Ghidra's relocation table, which is empty for this loader (owner at plan review, 2026-11-07). Loading at the oracle's bases was rejected because it ties the code to those addresses; treating any in-range value as an address was rejected as a guess.
- Planner: no prototype, since nothing here has a surface.

---

## 8. Explicitly out of scope

- Stages 1 to 6 of the brief: the full instruction census against Moira, finding all code (vectors, `level3_handler`, hunk 4), the Amiga runtime, whole-program runs, the seam, the app. The second plan, after the go.
- Cycle counting, because the trial's question is whether translation is right, and timing is judged in stage 1.
- Who may run a translated app, what is rewritten by hand, Superfrog as a proving ground: the second plan or later.
- Any change to level 1's run or its plan files.
- The boot and crack programs, Superfrog, map editing (brief).
