# games/forest-dumb/tools

How to call each script without reading its source. Commands run from the repo root. `venv/bin/python`
is the repo's venv (git-ignored; a fresh worktree may lack it, the main checkout has it); scripts marked
"stdlib" run under `python3 -I`. Everything derived from the disks or the program (listings, prompts,
ports, packs, pictures) stays under `games/forest-dumb/build/` (git-ignored) and is never committed.

**Most scripts need the extracted disks**: `cargo run -p fd-tools -- extract` in `games/forest-dumb/engine/`
writes `build/files/disk1/FOREST_DUMB_FOREVER` and the rest of `build/files/` (disk location:
`docs/01-disk-location.md`).

## Model services

generate and reshape ask Haiku through the owner's Claude subscription (`model.py`: `claude -p --model
haiku`, no tools; login check `claude auth status`; no `claude` on PATH, or no call answered: result
`no-model`). A run makes at most `--max-calls` calls (default 40; `stopped: "max-calls"` when the cap ends it). translate and
translate_batch still call Muse, a paid model, with their own key and dollar cap (their docstrings);
T18 of `plans/forest-dumb-translator/` replaces them. All need the listing
`build/re/forest_dumb_forever.lst` (re_listing.py) and cargo for the judge. Each prints one JSON line
per routine/unit on stdout; exit 0 on success, 1 on any other result, 2 on a usage error (reason on
stderr). Model calls log to `<out>/calls.jsonl`, a line a call; generate and reshape log each judged
answer to `<out>/tries.jsonl`. Procedure, results table and when to translate by hand:
[docs/11-translation.md](../docs/11-translation.md). Quote the routine: `$5` is a shell parameter.

### model.py, judge_run.py (stdlib, libraries)
`model.ask(system, prompt, log=...)` is one Haiku call: `ok`, `no-model`, `timeout` or `max-calls`;
`model.transcript(messages)` folds earlier tries into one prompt, since `claude -p` keeps no
conversation. `judge_run` holds the routine's listing (`flow_text`), the judge lock
(`build/translate/judge.lock`) and `fd-judge`'s build and run, shared by every tool here.

### translate.py (stdlib)
One routine to a literal Rust port by Muse, judged by `fd-judge` on its generator's 2 000 states.
```
python3 -I games/forest-dumb/tools/translate.py 'h0+$5462' --gen <generator> [--tries 3] [--max-usd 0.10] [--out DIR]
```
Routine is `hN+$off` or a name from `re/symbols.toml`; `--gen` is required (registered in
`fd-conform/src/gen/`). Default out `build/translate/`; port at `<out>/h0-5462/port.rs`. Results:
pass, fail, too-big (over 180 instructions; see call_tree `split_at`), no-oracle, no-key. A split callee
with no passing port yet is a usage error.

### generate.py (stdlib)
Haiku writes one routine's test-state generator, judged by branch coverage on the original.
```
python3 -I games/forest-dumb/tools/generate.py 'h0+$5462' --name <generator> [--tries 3] [--max-calls 40] [--out DIR]
```
Default out `build/generate/`; `gen` is `<out>/<name>.rs`. Results: covered, partial, fail, no-oracle,
no-model; `uncovered` lists branches never taken. The builder reads the generator and commits it into
`fd-conform/src/gen/` itself.

### reshape.py (stdlib)
Haiku turns a literal port into engine code on typed state, judged by the task's rig test, then tidied.
```
python3 -I games/forest-dumb/tools/reshape.py 'h0+$4274' --task T08 [--tries 3] [--tidy 2] [--max-calls 40] [--out DIR]
python3 -I games/forest-dumb/tools/reshape.py --task T08 --all [--only fn_a,fn_b]
python3 -I games/forest-dumb/tools/reshape.py --opus-cost [--conversations DIR]
```
`--task` choices are the tables of `re/reshape.toml`; the routine is `h0+$off` or its rig name. Needs
the unit's literal port in `build/translate`. Default out `build/reshape/`; it works in a scratch copy
there and never changes committed `fd-core`. `quality` lists what the builder still has to tidy.
`--all`'s units share one `--max-calls`. `--opus-cost` makes no model call.

### translate_batch.py (stdlib)
translate.py over every routine of `re/translate.toml`, one unattended run.
```
python3 -I games/forest-dumb/tools/translate_batch.py --max-usd 5 [--parallel 8] [--resume] [--out DIR]
```
`--max-usd` is required and caps the whole run. Writes `<out>/batch.json` and `<out>/table.md`, prints a
one-line summary. `--resume` keeps finished rows. Exit 1 when any routine is `not-attempted` or `error`.

## Listing and inventory

### re_listing.py
Labelled m68k listing of the main program, from `re/symbols.toml`.
```
venv/bin/python games/forest-dumb/tools/re_listing.py [--exe FILE] [--out DIR] [--xrefs] [--symbols FILE]
```
Writes `build/re/forest_dumb_forever.lst`, prints its path. `--xrefs` adds references to every label.
Gotcha: listing addresses are offsets from a load base of 0 (hunks back to back), not the `h0+$x` a note
uses.

### call_tree.py
Routines reachable from `game_frame` and `redraw_edge_a..d`, sized for translation; writes or checks
`re/translate.toml`.
```
venv/bin/python games/forest-dumb/tools/call_tree.py            # rewrite re/translate.toml
venv/bin/python games/forest-dumb/tools/call_tree.py --check    # check only, exit 1 on problems
venv/bin/python games/forest-dumb/tools/call_tree.py --summary  # each task's share; no disks needed
```
Also `--exe FILE`, `--toml FILE`. A rewrite keeps the hand classification (`subsystem`, `gen_task`,
`port_task`, `note`) by address; a new routine is `unassigned`, which `--check` refuses.

### find_file_names.py (stdlib)
Prints AmigaDOS file-name candidates found in a file's bytes, sorted, one a line.
```
python3 -I games/forest-dumb/tools/find_file_names.py <file>
```
Names print as spelled; case-insensitive comparison with the inventory is the caller's job.

## Translator

### pcode_export.py (stdlib)
Ghidra's P-code for every instruction of the whole main program, every address hunk-relative
(`hN+$off`), for the static translator (plan `forest-dumb-translator`, DESIGN §2.1, §2.3, §3.4).
```
python3 -I games/forest-dumb/tools/pcode_export.py [--out build/translator] [--entries re/entries.toml]
                                                   [--whole | --routines re/translate.toml] [--reimport]
```
Seeds the program entry `h0+$0`, every `func` of `re/symbols.toml`, `re/entries.toml` (the oracle trace)
and the handlers the code installs in an autovector (`$64..$7c`); follows direct flow and the jump tables
Ghidra resolves. `--routines` seeds only the trial's routines and warns on stderr for one whose reach
differs from `translate.toml` by over 5%. `pcode.json` holds `entries` (`{"at", "source"}`, source
program-entry, symbol, vector, trace or routine), `vectors`, per instruction its `hunk`, Ghidra's
`operands` text and its `bytes` (hex, for the census), and under `check`
the unresolved computed jumps by address (`unresolved_computed`) and flows out of every hunk (`outside`,
the OS calls). Needs Ghidra (`toolkit/tools/setup/install-ghidra.sh`); writes the Ghidra project
`<out>/ghidra/` (later runs reuse it; `--reimport` starts over) and prints one JSON summary line
(`instructions`, `entries`, `by_source`, `unresolved_computed`, `hunks`: instructions per hunk). Exit 2,
path on stderr, when Ghidra, its JDK, the program, a symbols or entries file is missing. Relocations come
from the file's `HUNK_RELOC32` entries, since Ghidra's Amiga loader records none. The script is
`ghidra/ExportPcode.java`.

### pcode_rust.py (stdlib)
The translator: `pcode.json` to Rust on the `pcode-rt` runtime, instruction for instruction, no model.
```
python3 -I games/forest-dumb/tools/pcode_rust.py build/translator/pcode.json --out build/translator/tree/
                                                 [--hints re/translator-hints.toml] [--parts N]
```
`--out DIR` writes `mod.rs` (`CODE`, `HUNK_SIZES`, `entry()`, `by_addr()`, `enter()`) and one module per
part, `p00.rs` .. `pNN.rs`, removing parts a larger earlier split left; `--out x.rs` writes one module,
as the trial did. Parts (default 16, measured by translator T04: the whole tree builds optimised in about
11 s, 28 s as one module) are groups of functions, each function with the ones only it calls up to one
part's share, packed largest first. `FD_TRANSLATED=<dir or .rs>` builds the tree into `fd-judge`. Prints
one JSON line (`functions`, `blocks`, `unsupported`, `computed_unresolved`, `parts`); exit 2 on a bad
input, named on stderr. After a translator change, `python3 tools/test_pcode_rust.py --regen` rewrites
the committed fixtures in `pcode-rt/tests/fixtures/`.
`--insn --out x.rs [--sites hN+$off,…|@file]` (translator T06) writes, for the instruction check, one
function per opcode word (its first site, or the sites given) that runs that instruction alone and leaves
the next address in `PC`, and `SITES` (encoding, site, hunk, offset, length, absolute operands, function).
Corrections to Ghidra's P-code live in `pcode_fix/`, one module per instruction family (`bcd`, `compare`,
`arith`, `shift_bit`, `move_control`, `cycles`), by mnemonic and P-code pattern, never by address; its
`__init__.py` says what a module may define. Ghidra 12.0's known defects (abcd, sbcd and nbcd's decimal
adjust, X on compares, `cmpm.b (a7)+`, a `bra.w` marked `CALL_RETURN`, the empty relocation table) are
proven one instruction at a time on Moira by `fd-conform/tests/insn_defects.rs`, on a made-up program
Ghidra exports to `tests/fixtures/defects.json` (`python3 -I games/forest-dumb/tools/test_pcode_export_ghidra.py
--write-defects` rewrites it; `ghidra/SetFlowOverride.java` sets the override first, test only).

### pcode_census.py (stdlib)
Every instruction kind `pcode.json` holds, the instruction check's work list (translator DESIGN §2.6).
```
python3 -I games/forest-dumb/tools/pcode_census.py build/translator/pcode.json --out build/translator/census.json
```
A kind is the mnemonic with its size suffix and each operand's addressing mode (`add.w Dn,(An)+`;
modes `Dn An (An) (An)+ -(An) (d16,An) (d8,An,Xn) abs.w abs.l (d16,PC) (d8,PC,Xn) #imm list label`).
Each row: `kind`, `mnemonic`, `size` (1/2/4 or null), `src`, `dst` (operand order, null when absent),
`count`, `encodings` (distinct opcode words), `sites` (first five); commonest first. Modes Ghidra's text
hides (movem's `-(An)`/`(An)+`, `jsr`/`jmp`/`pea`) come from the opcode word. Prints
`{"kinds","instructions"}`; exit 2 naming an operand it cannot place, or an export without `operands`.

### insn_check.py (stdlib)
The instruction check (translator DESIGN §2.6): every census kind, translated alone (`pcode_rust.py
--insn`), against Moira's one-instruction step on generated states, in columns `regs` (d0-d7, a0-a7, PC,
SR's system byte), `ccr` (X N Z V C), `mem` (bytes and custom-chip writes), `cycles`, and `stop`.
```
python3 -I games/forest-dumb/tools/insn_check.py [--kinds k,…] [--states 500] [--columns regs,ccr,mem,cycles] [--seed S]
```
`--kinds` takes census kinds (`add.w Dn,(An)+`), mnemonics (`add.w`, `add`) and families (`bcd`,
`compare`, `arith`, `shift_bit`, `move_control`, `other`). Needs `build/translator/pcode.json`; builds
`fd-judge` optimised with `FD_INSN=build/translator/insn.rs` and runs `fd-judge --insn-all` in four
shards (the whole census: about 10 s). Writes `build/translator/insn.json` and `insn.md` (by family with
the task owning it, then one row per kind by fail count) and prints the summary line. `cycles` reads
`not-counted` until the generated code counts (T11). A state the original takes an exception on (an odd
address, a division by zero) is skipped and counted. One kind alone: `fd-judge --insn <word> --at
'hN+$off'`. Holds `build/translate/judge.lock`; exit 0 when written, 1 when a step failed, 2 on usage.

### translator_trial.py (stdlib)
The whole trial: export, translate without and with `re/translator-hints.toml`, build `fd-judge` once a
tree, survey and judge every routine of `re/translate.toml` (DESIGN §2.1 to §2.4, §2.7).
```
python3 -I games/forest-dumb/tools/translator_trial.py [--routines name,…] [--skip-export] [--cases N]
```
Writes `build/translator/trial.json` and `table.md` (with Muse's `batch.json` results beside, from this
checkout or the main one), prints the gate summary line. Holds `build/translate/judge.lock` for the run and
refuses a second. Exit 0 when the table was written, 1 when a step failed (named), 2 on usage.

## Assets

### unpack.py
Classifies and unpacks `ESL!` (PowerPacker) and `S404` (StoneCracker 4) files with the original's own
68k routine. Needs `build/files` (the ESL! routine is cut from the main program).
```
venv/bin/python games/forest-dumb/tools/unpack.py --all            # build/files -> build/unpacked
venv/bin/python games/forest-dumb/tools/unpack.py <infile> <outfile>
venv/bin/python games/forest-dumb/tools/unpack.py --table          # the table docs/08-packers.md carries
```

### render_levels.py
Renders every level and sprite set to PNG. Needs `build/unpacked` (`unpack.py --all`) and `build/files`
(palette). Uses numpy and Pillow: run with the venv.
```
venv/bin/python games/forest-dumb/tools/render_levels.py [--out DIR]   # default build/renders, one path printed a picture
venv/bin/python games/forest-dumb/tools/render_levels.py --index       # the table docs/LEVEL-INDEX.md carries
venv/bin/python games/forest-dumb/tools/render_levels.py --spot W1-1:10,20   # what the three maps hold there
```

## Session tooling

### task_pack.py (stdlib)
One file holding what a task's session would otherwise read in its first steps (task doc, PROGRESS,
FINDINGS, DESIGN sections, dependencies' commits, file skeletons, routines and listings).
```
python3 -I games/forest-dumb/tools/task_pack.py T22 [--review] [--no-prep] [--max-kb 150]
```
Writes `build/pack/T22.md` (`T22-review.md` with `--review`, which leaves FINDINGS out and adds the
branch diffstat) and prints one JSON line; `omitted` names sections dropped for size and what to read
instead. Without `--no-prep` it extracts the program and writes the listing if missing.

### pack_check.py (stdlib)
Per session of the run, from its conversation logs: pack built/read step, steps to first edit and context
then, re-reads of what the pack holds, cost.
```
python3 -I games/forest-dumb/tools/pack_check.py [--conversations DIR] [--since T42-implement-2] [--json]
```
Default logs: the main checkout's `plans/forest-dumb-level-1/.parallel/control/conversations/`.

## Tests

`test_<script>.py` (and `test_*_judge.py` for the judge halves) test the script of that name; model calls
are stubbed, no network, no disks needed. Run all with `games/forest-dumb/test.sh`, or one:
```
cd games/forest-dumb && ../../venv/bin/python -m unittest discover -q -s tools -p test_translate.py
```
Each is labelled in `games/forest-dumb/smoke.toml` `[python]`; a new test file needs a label there (see
`tools/README.md` at the repo root).
