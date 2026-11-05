# T61 — generate-service

**Phase:** 5 · **Depends on:** T33, T60 · **Weight:** medium

Added by the owner (2026-11-05, mid-run amendment): generators were the largest Opus cost of the translation track, about $44 for 95 routines (checkpoint 2). Muse writes them, judged by machine (DESIGN §4.3).

## Goal
A command that takes a routine and returns a generator written by Muse that reaches every branch it can, judged on the original, so Opus reviews generators instead of writing them. Same shape as T33's service: the builder sees the result line and the generator, never the listing or the retries.
## Design sections this implements
§4.3, §5.2, §5.3.
## Reuses
T33's `translate.py` (listing extraction, OpenRouter call, retries, spend guard, `calls.jsonl`), T60's `Rig::coverage`, T33's `FD_PORT` build-in in `fd-conform/build.rs` as the pattern for building the candidate generator in, T32's `Generator` and `Setup`, the committed generators in `fd-conform/src/gen/` as examples in the prompt (two or three of the same subsystem, chosen by the command, not the whole folder).
## Files
`games/forest-dumb/tools/generate.py`, `games/forest-dumb/tools/test_generate.py`, `fd-conform/src/bin/fd-judge.rs` (a generator mode), `fd-conform/build.rs` and `fd-conform/src/gen/mod.rs` (the candidate built in from an `FD_GEN` path under `build/`, as `FD_PORT`, and registered under its name), `games/forest-dumb/docs/11-translation.md` (how to use it), `smoke.toml`.
## Interface
```
python3 -I games/forest-dumb/tools/generate.py 'h0+$5462' --name <generator> [--tries 3] [--max-usd 0.10] [--out build/generate]
→ build/generate/<name>.rs, and one stdout line:
  {"routine":"h0+$5462","result":"covered|partial|fail|no-oracle|no-key","tries":n,"usd":x,"branches":n,"uncovered":[...],"returns":n,"gen":"build/generate/<name>.rs"}
```
The judge, per answer: it compiles; the original returns on at least 95% of 2 000 states with no hang; its states' pokes land only where the original read on some state (`Coverage::reads`) or on the stack, as T35 to T39's tests check; a `base` is shown to the reviewer, not judged. Then `Rig::coverage`: a `Bcc`/`DBcc` of `translate.py`'s `flow_text` listing missing from its `branches` counts as uncovered too; the uncovered branches go back to Muse with their listing lines, up to `--tries`. `covered` means every branch both ways; `partial` lists what no try reached. Muse is told a branch may be unreachable and may say so with a reason, which is passed through in `uncovered` for the reviewer. The generator is code written from the listing, like the hand-written ones, and the builder commits it into `fd-conform/src/gen/` after reading it.
## Tests
- [ ] Model calls stubbed, no network: a canned generator missing one branch gets that branch's listing lines back; three such answers give `partial` naming it.
- [ ] A canned generator that hangs the original, or writes outside the routine's reads, is a failed try with the reason sent back.
- [ ] `--max-usd` and `no-key` as in T33.
## Done when
- [ ] One live run per subsystem on a routine that already has a hand-written generator (five routines, a §5.3 `worker` action): coverage and cost of Muse's generator against the existing one, in `docs/11-translation.md` and FINDINGS.
- [ ] `docs/11-translation.md` says how a builder calls it, that a `partial` result's unreached branches go to the reviewer, and what it cannot prove.
