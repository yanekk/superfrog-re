# T18 — translate-command

**Phase:** 3 · **Depends on:** T16, T17 · **Weight:** medium

## Goal

The one command a session runs to get a routine's, or a task's, translation and its judge result (DESIGN §2.12). It replaces Muse's `translate.py` under the same name, and the trial's two-step `pcode_export.py` then `translator_trial.py --skip-export`.

## Design sections this implements

DESIGN §2.12, §2.13.

## Files

- `tools/translate.py` (rewritten), `tools/test_translate.py` (rewritten).
- Deleted: `tools/translate_batch.py`, `tools/test_translate_batch.py`, `tools/test_translate_judge.py` if it tests Muse only, and `translator_trial.py`'s Muse column (it reads `build/translate/batch.json` only if present).
- `tools/README.md`: the command's section first.

## Interface

```
translate.py 'hN+$off' | <symbol> | --task T{nn} [--gen <generator>] [--cases 2000] [--fresh]
translate.py --build   # export and translate the whole program to build/translator/tree/, no judging (what fd-translated includes)
# exports and translates the whole program once (cached on the program's SHA-1 and the tool's own version),
# writes build/translate/<hN-off>/translated.rs (the routine and its callees, readable order) and prints:
{"routine":"h0+$5462","name":"char_sprite_position","result":"pass|fail|no-gen|no-oracle|stop",
 "first":"...","states":2000,"rust":"games/forest-dumb/build/translate/h0-5462/translated.rs"}
# exit 0 pass, 1 otherwise, 2 usage
```

`--task` takes a task's routines from `re/translate.toml` or the task doc's routine list and prints one line each.

## Tests

- [ ] Fake judge and fake export: pass, fail with first difference, no generator, symbol lookup, `--task`, `--build`.
- [ ] Full: `translate.py char_sprite_position` passes on the real program in under 2 minutes with a warm cache.

## Done when

- [ ] One command gives the Rust path and a pass line; README documents it.
- [ ] No tool imports Muse's code; `grep -ri openrouter games/forest-dumb/tools` is empty.
