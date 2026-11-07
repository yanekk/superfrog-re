# games/forest-dumb/tools

How to call each script without reading its source. Commands run from the repo root. `venv/bin/python`
is the repo's venv (git-ignored; a fresh worktree may lack it, the main checkout has it); scripts marked
"stdlib" run under `python3 -I`. Everything derived from the disks or the program (listings, prompts,
ports, packs, pictures) stays under `games/forest-dumb/build/` (git-ignored) and is never committed.

**Most scripts need the extracted disks**: `cargo run -p fd-tools -- extract` in `games/forest-dumb/engine/`
writes `build/files/disk1/FOREST_DUMB_FOREVER` and the rest of `build/files/` (disk location:
`docs/01-disk-location.md`).

## Muse services (paid, OpenRouter)

translate, generate, reshape and translate_batch call a paid model. They need `OPENROUTER_API_KEY` in the
environment or as an `export OPENROUTER_API_KEY=` line in `~/.zshrc` (no key: result `no-key`), the
listing `build/re/forest_dumb_forever.lst` (re_listing.py), and cargo for the judge. Each prints one JSON
line per routine/unit on stdout; exit 0 on success, 1 on any other result, 2 on a usage error (reason
on stderr). Calls log to `<out>/calls.jsonl`. Procedure, results table and when to translate by hand:
[docs/11-translation.md](../docs/11-translation.md). Quote the routine: `$5` is a shell parameter.

### translate.py (stdlib)
One routine to a literal Rust port, judged by `fd-judge` on its generator's 2 000 states.
```
python3 -I games/forest-dumb/tools/translate.py 'h0+$5462' --gen <generator> [--tries 3] [--max-usd 0.10] [--out DIR]
```
Routine is `hN+$off` or a name from `re/symbols.toml`; `--gen` is required (registered in
`fd-conform/src/gen/`). Default out `build/translate/`; port at `<out>/h0-5462/port.rs`. Results:
pass, fail, too-big (over 180 instructions; see call_tree `split_at`), no-oracle, no-key. A split callee
with no passing port yet is a usage error.

### generate.py (stdlib)
Muse writes one routine's test-state generator, judged by branch coverage on the original.
```
python3 -I games/forest-dumb/tools/generate.py 'h0+$5462' --name <generator> [--tries 3] [--max-usd 0.10] [--out DIR]
```
Default out `build/generate/`; `gen` is `<out>/<name>.rs`. Results: covered, partial, fail, no-oracle,
no-key; `uncovered` lists branches never taken. The builder reads the generator and commits it into
`fd-conform/src/gen/` itself.

### reshape.py (stdlib)
Muse turns a literal port into engine code on typed state, judged by the task's rig test, then tidied.
```
python3 -I games/forest-dumb/tools/reshape.py 'h0+$4274' --task T08 [--tries 3] [--tidy 2] [--max-usd 0.10] [--out DIR]
python3 -I games/forest-dumb/tools/reshape.py --task T08 --all [--only fn_a,fn_b]
python3 -I games/forest-dumb/tools/reshape.py --opus-cost [--conversations DIR]
```
`--task` choices are the tables of `re/reshape.toml`; the routine is `h0+$off` or its rig name. Needs
the unit's literal port in `build/translate`. Default out `build/reshape/`; it works in a scratch copy
there and never changes committed `fd-core`. `quality` lists what the builder still has to tidy.
`--opus-cost` makes no model call.

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
