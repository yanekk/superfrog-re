# Translation by Muse

How a builder gets a literal Rust port of one routine of the main program without reading its
listing (plans/forest-dumb-level-1 T33, DESIGN §4.3). Measured background: `spikes/muse-spark/README.md`.

## Calling it

From the repo root, the routine quoted (`$5` is a shell parameter):

```
python3 -I games/forest-dumb/tools/translate.py 'h0+$5462' --gen char_sprite_position --max-usd 0.10
```

- The routine is `hN+$off` or its name in `re/symbols.toml`.
- `--gen` names a generator registered in `engine/crates/fd-conform/src/gen/` (T32, T35 to T39). No
  generator, no translation: write or ask for one first.
- `--tries` (3), `--max-usd` (0.10, DESIGN §5.2) and `--out` (`games/forest-dumb/build/translate`).
- Needs `OPENROUTER_API_KEY` in the environment or as an `export` line in `~/.zshrc`, the extracted
  program (`cargo run -p fd-tools -- extract` in `engine/`) and its listing (`tools/re_listing.py`).

It prints one line and nothing else on stdout; read that and the port, not the files beside them:

```
{"routine":"h0+$5462","result":"pass","tries":1,"tokens_in":1629,"tokens_out":7897,"usd":0.001742,"paths":4,"port":"games/forest-dumb/build/translate/h0-5462/port.rs"}
```

| `result` | Meaning | Exit | What the builder does |
|---|---|---|---|
| `pass` | `port` matched the original on all 2 000 states | 0 | turns it into engine code on typed state, proves that on the rig |
| `fail` | `--tries` answers were wrong, or `"stopped":"max-usd"` came first | 1 | translates it by hand |
| `too-big` | over 180 instructions with its callees, less its `split_at` callees; no call made | 1 | asks for `split_at` in `re/translate.toml` (T34), or translates by hand |
| `no-oracle` | the original returned on none of the generator's states, or started a line or fill blit on one; no call made | 1 | fixes the generator, or translates by hand |
| `no-key` | no key; stderr says where it looked | 1 | asks the owner for the key |

Exit 2 is a usage error with the reason on stderr: an unknown routine or generator, a missing
listing or disks, or a split callee with no passing port yet.

`paths` is how many distinct instruction counts the original ran over the states: a proxy for the
branches the generator reaches. A `pass` with few paths proves little.

## What it does

1. The listing by control flow from the routine (every branch both ways, calls followed), then the
   named labels it uses: the spike's `run.py --hw` extraction. Its size decides `too-big`.
2. `fd-judge --survey`: the original alone over the generator's states. It is `no-oracle` when it
   returns on none (a wait on DMA or an interrupt never ends in the routine oracle's hardware mode)
   or when any state starts a blit in line or fill mode (`RigReport::unmodelled_blits`): the
   oracle's blitter runs area mode only, so a port would agree with a blit that never happened.
3. Muse (`meta/muse-spark-1.3-contributor`, `max_tokens` 96 000) answers; the last ```` ```rust ````
   block is built into `fd-judge` (`FD_PORT`, `fd-conform/build.rs`) and judged against the original
   over 2 000 states: registers d0-d7 and a0-a6, every byte written off the stack, and every
   custom-chip write in order. A wrong answer gets the first failing case back; one that does not
   compile gets the compiler's errors; one with no code (cut off while reasoning) gets asked again.
   Each counts as a try. A 429, 502 or 503 is waited out and is not a try.
4. Before each call, its worst case (the conversation at three characters a token, plus 96 000
   answer tokens, at OpenRouter's listed price) must fit under `--max-usd` with what is spent.

The port is written against `Cpu { d, a, mem: Mem }` (`fd-conform/src/judge/mem.rs`), the spike's
model of memory and the custom chips. One judge build and run at a time, across processes
(`<out>/judge.lock`), since the port is compiled into the one binary.

### A split routine

A routine T34 splits names its callees in `split_at`. Those are translated first; each must have a
passing port in its own folder. The prompt leaves their listing out and tells Muse to call
`port_hN_off(cpu)` where the original calls them, and the judge builds their ports in as modules.
The caller's `port.rs` is its own code only; its callees' are in their folders.

## What it cannot prove

- Only the generated states. A pass is equality on the generator's 2 000 states, nothing else: a
  branch no state reaches is untested. The reviewer of each generator checks branches against the
  listing (T35 to T39).
- Only what the routine oracle models. Line and fill blits are refused (`no-oracle`); a read of a
  write-only register, DMA and interrupt timing are not modelled on either side, so a port agreeing
  there proves nothing.
- Not the engine port. A literal port over raw memory is the input to a porting task, which proves
  its engine code on the rig (`fd-conform/src/routine.rs`).
- Condition codes and the stack bytes are not compared.

## Files

Under `--out`, never committed (they derive from the game's code): `calls.jsonl`, a line a call
(time, tries, tokens, reasoning, cost, outcome, the feedback's start); per routine `hN-off/` with
`prompt.md`, `answerN.md`, `tryN.rs` (the code), `judgedN.rs` (what was built, callees included),
`port.rs` (the passing answer only) and `result.json` (the result line, its generator and `split_at`).
A new run of a routine clears its folder first.

## Record

- 2026-11-05, T33: `h0+$5462` with `char_sprite_position`: pass on try 1, 1 629 tokens in, 7 897 out
  (7 131 reasoning), $0.0017, 67 s, 4 paths.
