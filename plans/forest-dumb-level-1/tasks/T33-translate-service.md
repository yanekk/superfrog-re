# T33 — translate-service

**Phase:** 1 · **Depends on:** T32 · **Weight:** medium

## Goal
A command that takes a routine and a generator and returns a judged Rust port written by Muse, so builders spend no Opus tokens on literal translation (owner, 2026-11-05; `games/forest-dumb/docs/level-1-brief.md`, "Translation by Muse"). The builder sees only the result line and the port, never the listing or the retries.
## Design sections this implements
§4.3, §5.2, §5.3.
## Reuses
`games/forest-dumb/spikes/muse-spark/`: `run.py`'s prompts, `--hw` control-flow extraction with named labels, retry with the first failing case, spend guard; `judge/src/lib.rs`'s `Mem` and hardware model. T32's `Generator` and rig, `tools/re_listing.py`. Nothing else exists. It lives in the game first; `Mem` and the hardware model move to the toolkit when a second game uses them.
## Files
`games/forest-dumb/tools/translate.py`, `games/forest-dumb/tools/test_translate.py`, `games/forest-dumb/engine/crates/fd-conform/src/bin/fd-judge.rs`, `fd-conform/src/judge/{mod.rs,mem.rs}`, `fd-conform/{build.rs,Cargo.toml}` (the port is built in from a path under `build/`, a stub otherwise, as the spike's `build.rs`), `games/forest-dumb/docs/11-translation.md` (how to use it), `smoke.toml`.
## Interface
```
python3 -I games/forest-dumb/tools/translate.py 'h0+$5462' --gen <name> [--tries 3] [--max-usd 0.10] [--out build/translate]
→ build/translate/h0-5462/port.rs, and one stdout line:
  {"routine":"h0+$5462","result":"pass|fail|too-big|no-oracle|no-key","tries":n,"tokens_in":n,"tokens_out":n,"usd":x,"paths":n,"port":"build/translate/h0-5462/port.rs"}
```
Model `meta/muse-spark-1.3-contributor`, `max_tokens` 96 000, key `OPENROUTER_API_KEY` from the environment or `~/.zshrc`. A routine over 180 instructions with its callees, less the callees named in its `split_at`, answers `too-big` without a call. A split routine is judged with its `split_at` callees' passing ports built in, so they are translated first. A routine the original returns from on no generated state (a line-mode or fill blit, a wait on DMA or an interrupt: what hardware mode does not model) answers `no-oracle` without a call. The routine argument is quoted, since `$5` is a shell parameter. Every call appends to `build/translate/calls.jsonl`. Listings, prompts and ports derive from the game's code and stay in `build/`.
## Tests
- [ ] With no port the judge fails every case (the negative check); a hand port of `h0+$774` passes (positive).
- [ ] A canned wrong answer gets its first failing case sent back; three wrong answers give `fail` and no fourth call (model calls stubbed, no network).
- [ ] An answer cut off with no code counts as a try; 429, 502 and 503 are waited out, not counted.
- [ ] `--max-usd` stops before a call that would pass it.
- [ ] A missing key gives `no-key` and exit non-zero, naming where it looked.
- [ ] A routine whose original never returns on the generator's states gives `no-oracle` and no call.
## Done when
- [ ] `docs/11-translation.md` says how a builder calls it and what it cannot prove (generated states only).
- [ ] Nothing derived from the game's code is committed.
- [ ] One live call on `h0+$5462` passed, its log line in FINDINGS (a §5.3 `worker` action, run once, not a test: a test file never calls the network).
