# T06 — trial-tool

**Phase:** 2 · **Depends on:** T01, T03, T04 · **Weight:** medium

## Goal

One command runs the whole trial: export, translate, build the judge once, judge every routine of `re/translate.toml`, once without hints and once with them, and write the table the owner decides on. This task builds that command, creates the empty hints file, and produces the first table, so T07 has a list of failures to work through.

## Design sections this implements

DESIGN §2.1 to §2.4, §2.7, §2.8, §3.5.

## Files

- `games/forest-dumb/tools/translator_trial.py` (new), `games/forest-dumb/tools/test_translator_trial.py` (new)
- `games/forest-dumb/re/translator-hints.toml` (new: format header, no hints yet)
- `games/forest-dumb/smoke.toml` (label)

## Interface

```
python3 -I games/forest-dumb/tools/translator_trial.py [--routines <name,…>] [--skip-export] [--cases N]
  → build/translator/trial.json, build/translator/table.md; prints
  {"judgeable":n,"pass":n,"pass_pct":x,"pass_with_hints":n,"unjudgeable":n,"unsupported":n,"untranslated":n,"write_into_code":n}
  exit 0 when the table was written, whatever the verdicts; 1 when a step failed (named); 2 on usage
```

`translator-hints.toml`:

```toml
[[hint]]
at = "h0+$2fae"            # the instruction it is about
kind = "targets"          # targets | data
targets = ["h0+$3010"]    # for targets; for data: len = 12
why = "jump table of 4 words after the jmp; Ghidra does not follow it"
```

Table columns: routine (`hN+$off` and name), subsystem, instructions, verdict without hints (`pass`, `fail`, `unjudgeable: <reason>`), first difference or first stop, untranslated, write-into-code, unsupported, verdict with hints, Muse's result from `build/translate/batch.json`. Below it, the gate line: judgeable count, passes, percentage against 90%.

- Unjudgeable (DESIGN §2.4): no generator registered, or the judge's `--survey` returns on no state (`no-oracle`). Decided per routine by running the survey, never by a list.
- The tool takes `judge.lock` for the whole run and refuses a second run (DESIGN §2.8).
- A generated tree that does not compile ends the run with exit 1, the compiler's first error and the routine it came from.
- `table.md` is written through a temporary file and a rename.

## Tests

- [ ] With a fake judge and fake export (a stub command given by an environment variable, test only): the table's columns, the gate line, the unjudgeable rows, and the percentage leave unjudgeable rows out.
- [ ] Muse's column reads `batch.json`'s results, and shows `none` when the file is absent.
- [ ] A second run while the lock is held refuses and names the lock.
- [ ] A compile failure exits 1 naming the routine.

## Done when

- [ ] The tests pass, labelled in `smoke.toml`.
- [ ] One real run over all 87 routines has finished, and its printed summary line is in `FINDINGS.md`, dated, whatever it shows.
