# Spike: Muse Spark contributor translating routines (2026-11-04)

No plan. Question: can `meta/muse-spark-1.3-contributor` (OpenRouter) take over translating the
original's 68k routines into Rust, the token-heavy part of the level 1 work, at Opus's quality?
Owner's choices: the contributor version (Meta may keep what is sent, so pieces of the listing went
to Meta), routine translation as the job, Opus as the baseline, about five routines.

## How

- `judge/`: its own cargo workspace. Loads `FOREST_DUMB_FOREVER` at `$20000` with `Cpu68k::load_exe`,
  runs the original routine on Moira and the port (`SPIKE_PORT`, built in by `build.rs`) from 2000
  generated states each, and compares d0-d7, a0-a6 and every byte written (call stack excluded,
  condition codes not compared). Per-routine generators are in `src/main.rs`. Without a port the
  stub fails every routine, the judge's negative check.
- `run.py`: one prompt per routine (its listing lines with callees, the port interface), one system
  prompt for both models. Muse through the OpenRouter API, Opus through `claude -p --tools ''`
  with its own system prompt. A failed answer gets the judge's first failing case (or compiler
  errors) back, up to 3 tries a run, 3 runs a routine. Calls log to `build/spike/muse-spark/calls.jsonl`.
- `summary.py`: the tables below from that log.
- The listing, the prompts and every answer derive from the game's code and stay in `build/`.

Rerun: `fd-tools extract`, `tools/re_listing.py`, then `python3 -I run.py` and `python3 -I summary.py`
here. `OPENROUTER_API_KEY` from the environment or `~/.zshrc`.

## Routines

Picked from `game_frame`'s call tree: no custom-chip access, rising size. Paths is the number of
distinct instruction counts the original ran over the 2000 states, a proxy for branch coverage.

| Routine | Instructions (with callees) | Prompt chars | Paths | What it reads as |
|---|---|---|---|---|
| `h0+$5462` | 34 | 1 895 | 4 | player position to sprite control words in hunk 1 |
| `h0+$9754` | 61 | 3 348 | 2 | camera to the copper's bitplane pointers |
| `h0+$24ce` | 69 | 3 878 | 7 | map cell under the player, trigger codes `$10`-`$13` |
| `h0+$2dd6` | 122 | 6 756 | 142 | five objects leaving the screen written into the map |
| `h0+$2fae` | 493 | 26 772 | 205 | five objects' per-type movement; data words inside the code |

## Results

| Model | Routine | Runs passed | First try | Calls | Tokens in | Tokens out | Cost $ | Secs/call |
|---|---|---|---|---|---|---|---|---|
| muse | 5462 | 3/3 | 3/3 | 3 | 4 185 | 19 237 | 0.0041 | 60 |
| opus | 5462 | 3/3 | 3/3 | 3 | 8 478 | 3 466 | 0.0851 | 10 |
| muse | 9754 | 3/3 | 3/3 | 3 | 5 988 | 24 897 | 0.0054 | 78 |
| opus | 9754 | 3/3 | 3/3 | 3 | 10 569 | 5 983 | 0.1519 | 17 |
| muse | 24ce | 3/3 | 3/3 | 3 | 6 945 | 12 781 | 0.0032 | 37 |
| opus | 24ce | 3/3 | 3/3 | 3 | 11 601 | 2 978 | 0.0947 | 9 |
| muse | 2dd6 | 3/3 | 3/3 | 3 | 10 212 | 40 436 | 0.0091 | 115 |
| opus | 2dd6 | 3/3 | 3/3 | 3 | 15 627 | 8 808 | 0.2226 | 24 |
| muse | 2fae | 3/3 | 0/3 | 9 | 143 464 | 202 020 | 0.0515 | 196 |
| opus | 2fae | 3/3 | 3/3 | 3 | 41 430 | 19 746 | 0.5136 | 55 |

| Model | Runs passed | Calls | Tokens in | Tokens out | Cost $ | Cost per passed run $ |
|---|---|---|---|---|---|---|
| muse | 15/15 | 21 | 170 794 | 299 371 | 0.0733 | 0.0049 |
| opus | 15/15 | 15 | 87 705 | 40 981 | 1.0681 | 0.0712 |

Opus's cost is what `claude -p` reports (`total_cost_usd`), its input tokens including cache reads.
Muse's out tokens are mostly reasoning.

On `2fae` Muse's first tries: two hit the 32 000-token answer limit while still reasoning (no code),
one differed on 304 of 2000 states. Its second tries differed on 12 to 294; all three runs passed on
the third. Opus passed each first time.

## What it shows

- Quality: equal on these five; both passed every run under the same judge. Up to ~120 instructions
  Muse is right first time; at ~500 it needs the judge's feedback and sometimes runs out of room.
- Cost: Muse is ~15x cheaper per passed run ($0.005 against $0.071), ~33x on the small routines.
- Speed: Muse is 4-6x slower per call (37-196 s against 9-55 s); parallel runs hide it.
- The judge is what makes it usable: Muse's wrong answers were caught only because every answer was
  run against the original. A translation with no machine check should not come from Muse.

## Limits

- A passing port matched the original on 2000 generated states, not on every state.
- The ports are literal translations over raw memory and registers. The engine's ports work on typed
  state (`fd-core`), so a session would still turn a literal port into engine code, or the prompt
  would have to ask for that shape and the judge would have to compare through a state transplant.
- The judge's generators were written by hand for each routine; that work stays with Claude.
- Leaf routines without custom-chip access only. Routines that blit or touch Paula need the
  judge's hardware mode (`Cpu68k::set_hardware`).
- Opus ran through `claude -p` with its default thinking; its cost is notional on a subscription.

## Round 2: routines that drive the custom chips (2026-11-04)

The judge now runs in the routine oracle's hardware mode and compares the custom-chip writes, in
order, as well as registers and memory. The port's `Mem` reproduces the shim's hardware mode
(`judge/src/lib.rs`): custom writes recorded, a BLTSIZE write runs the blit (the shim's `blit`,
line for line), VHPOSR bytes step a counter, other reads return what the case set. VHPOSR's counter
runs on across calls, so a two-instruction probe reads it before each case and the port starts from
there. The program is loaded at the oracle's hunk bases, not packed from `$20000`, so the chip hunks
sit where the blitter reaches them; round 1's numbers were taken under the old loader. Positive check:
a hand translation of `h0+$774` passes 2000 of 2000. Not modelled: a CPU read of a write-only register,
which the shim answers with the data bus's last word (none of these routines does it).

`run.py --hw` adds a hardware paragraph to the system prompt and extracts the listing by control flow
(every branch, backward ones too, which the music player uses), followed by the named labels it uses.

| Routine | Kind | Instructions | Paths | What it does |
|---|---|---|---|---|
| `h0+$3adc` | Paula | 12 | 1 | raster wait, then starts a sample on channel 2 |
| `h0+$774` | blitter | 20 | 3 | waits for raster and blitter, blits one tile |
| `redraw_edge_a` `h0+$844` | blitter | 131 | 7 | redraws one map edge as the screen scrolls |
| `h0+$3b2a` | Paula, CLXDAT | 183 | 69 | collision register, objects against the player, a sound |
| `redraw_edge_b` `h0+$b20` | blitter | 298 | 7 | the opposite edge |
| `h4+$18870` | Paula | 461 | 86 | the music player's per-channel effects tick |

Same rules as round 1 (3 runs, up to 3 tries, Muse `max_tokens` 32 000):

| Model | Routine | Runs passed | First try | Calls | Tokens in | Tokens out | Cost $ | Secs/call |
|---|---|---|---|---|---|---|---|---|
| muse | 3adc | 3/3 | 3/3 | 3 | 3 585 | 11 642 | 0.0027 | 40 |
| opus | 3adc | 3/3 | 3/3 | 3 | 8 031 | 2 016 | 0.0723 | 7 |
| muse | 774 | 3/3 | 3/3 | 3 | 4 008 | 20 114 | 0.0044 | 60 |
| opus | 774 | 3/3 | 3/3 | 3 | 8 541 | 2 651 | 0.0864 | 10 |
| muse | 844 | 3/3 | 2/3 | 4 | 14 938 | 90 949 | 0.0190 | 194 |
| opus | 844 | 3/3 | 3/3 | 3 | 16 653 | 15 506 | 0.3662 | 44 |
| muse | 3b2a | 3/3 | 2/3 | 5 | 34 406 | 140 859 | 0.0315 | 262 |
| opus | 3b2a | 3/3 | 3/3 | 3 | 21 327 | 21 520 | 0.4996 | 60 |
| muse | b20 | 1/3 | 0/3 | 9 | 72 916 | 263 843 | 0.0593 | 285 |
| opus | b20 | 3/3 | 3/3 | 3 | 29 151 | 58 592 | 1.2536 | 158 |
| muse | 4-18870 | 0/3 | 0/3 | 9 | 102 654 | 288 177 | 0.0657 | 265 |
| opus | 4-18870 | 3/3 | 3/3 | 3 | 42 441 | 53 428 | 1.1876 | 143 |
| **muse** | all | 13/18 | | 33 | 232 507 | 815 584 | 0.1826 | |
| **opus** | all | 18/18 | | 18 | 126 144 | 153 713 | 3.4656 | |

17 of Muse's 21 failed tries were no code: the answer stopped at 32 000 tokens, all reasoning. Muse
alone was rerun on the two large routines with `--muse-max-tokens 128000` (the model allows ~940 000):

| Model | Routine | Runs passed | First try | Calls | Cost $ | Secs/call | Outcomes |
|---|---|---|---|---|---|---|---|
| muse 128k | b20 | 1/3 | 0/3 | 8 | 0.0703 | 321 | pass 1, fail 2, compile 4, no-code 1 |
| muse 128k | 4-18870 | 1/3 | 0/3 | 9 | 0.0757 | 260 | pass 1, fail 7, compile 1 |

With room it writes code, but large answers broke: unbalanced braces, one 1 600-line general 68000
interpreter instead of a translation (panics "bad size"), and translations off on 10 to 300 states.

### What round 2 shows

- Up to ~180 instructions, hardware makes no difference: Muse passed 12 of 12 runs (10 first try),
  blits, Paula writes and raster waits included, at 1/20 to 1/30 of Opus's cost.
- At ~300 to 460 instructions Muse fails: 2 of 6 runs with room to answer, 1 of 6 without. Opus passed
  all 6 first time, at $1.19 to $1.25 a routine.
- So the line is size, not hardware. Large routines go to Opus, or are split at their callees before
  Muse sees them; the music player's effect handlers are separate routines a port could take one by one.

## Round 3: free and nearly free models (2026-11-04/05)

`run.py --models` now takes any OpenRouter id (`muse` and `opus` stay aliases). It waits out
429/502/503 instead of counting a try, caps each new model at $0.50, lowers `--muse-max-tokens` to the
model's own `max_completion_tokens`, scores a port still running after 120 s as `hang`, and `--resume`
redoes only unfinished runs. `rank.py` prints the tables below (digit: the try a run passed on; x: failed).

Candidates: OpenRouter's model list (453) filtered to text models at most $0.50 per million output
tokens that can write code; 19 answered a test call (`stealth/crouton-alpha`: region-blocked).

**Screen**, one run on `774` (20), `2dd6` (122), `3b2a` (183), `--hw`, at `max_tokens` 32 000. Every
no-code answer in it stopped at 32 000 tokens of reasoning, so it was rerun at 96 000:

| Model | $/M out | Passed | First try | 774 | 2dd6 | 3b2a | Cost $ | Secs/call |
|---|---|---|---|---|---|---|---|---|
| muse | 0.20 | 3/3 | 2 | 1 | 1 | 2 | 0.0147 | 156 |
| nvidia/nemotron-3-ultra-550b-a55b:free | 0 | 2/3 | 1 | 1 | 2 | x | 0 | 153 |
| nvidia/nemotron-3.5-super-vl:free | 0 | 2/3 | 1 | 1 | 2 | x | 0 | 166 |
| xiaomi/mimo-v2.6-flash | 0.28 | 2/3 | 1 | 1 | 2 | x | 0.0373 | 198 |
| deepseek/deepseek-v4-flash | 0.18 | 2/3 | 1 | 1 | x | 2 | 0.0679 | 275 |
| z-ai/glm-5.3-flash | 0.10 | 2/3 | 0 | 2 | 3 | x | 0.0963 | 393 |
| minimax/minimax-m3.1-flash | 0.30 | 2/3 | 0 | 3 | 2 | x | 0.1142 | 115 |
| inclusionai/ling-3.1-flash | 0.23 | 2/3 | 0 | 3 | 2 | x | 0.1281 | 683 |
| dots-studio/dots-3-note-preview:free | 0 | 1/3 | 1 | 1 | x | x | 0 | 202 |
| openai/gpt-oss-120b | 0.09 | 1/3 | 1 | 1 | x | x | 0.0247 | 47 |
| cohere/north-mini-code:free | 0 | 1/3 | 0 | 3 | x | x | 0 | 467 |
| qwen/qwen3.7-flash | 0.13 | 1/3 | 0 | 2 | x | x | 0.0180 | 135 |
| stepfun/step-3.5-flash | 0.30 | 1/3 | 0 | 3 | x | x | 0.0992 | 473 |
| google/gemma-4-31b-it:free | 0 | 0/3 | 0 | x | x | x | 0 | rate-limited upstream, every call |
| nvidia/nemotron-3.5-lightning:free | 0 | 0/3 | 0 | x | x | x | 0 | 439 |
| recursive/flint-v1.0:free | 0 | 0/3 | 0 | x | x | x | 0 | 186 |
| xingchen-agi/xing4.0-29b-a4b:free | 0 | 0/3 | 0 | x | x | x | 0 | 532 |
| inception/mercury-2.5 | 0.15 | 0/3 | 0 | x | x | x | 0.0069 | 23 |
| nex-agi/nex-n2.5-mini | 0.10 | 0/3 | 0 | x | x | x | 0.0870 | 443 |

**Final**, 3 runs on the eight routines up to 183 instructions, 96 000 tokens:

| Model | Passed | First try | 3adc | 774 | 5462 | 9754 | 24ce | 2dd6 | 844 | 3b2a | Cost $ | Secs/call |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| muse | 24/24 | 22 | 111 | 111 | 121 | 111 | 111 | 111 | 111 | 211 | 0.0762 | 106 |
| deepseek/deepseek-v4-flash | 20/24 | 8 | 121 | 211 | 123 | 322 | 212 | 13x | 13x | 2xx | 0.3590 | 172 |
| xiaomi/mimo-v2.6-flash | 17/24 | 7 | x13 | 221 | 212 | 211 | 131 | 2xx | xx2 | xx2 | 0.2972 | 205 |
| nemotron-3-ultra, nemotron-3.5-super (free) | not run | | | | | | | | | | | |

The two free finalists got HTTP 429 `free-models-per-day` on every call: the account's limit is 50
free requests a day (1000 with 5 credits bought), and the screens used it.

### What round 3 shows

- No free or cheaper model matches Muse. The paid near-free ones fail more and cost more per run,
  because they need more tries and more tokens: DeepSeek V4 Flash $0.018 a passed run, MiMo $0.017,
  Muse $0.003.
- Errors cluster in the 122 to 183 instruction routines; up to ~70 every finalist passes, given tries.
- The free models screen no better than the cheap paid ones, and the two best were not finalised.

## Next if adopted

Send routines up to ~180 instructions to Muse through this judge; split larger ones at their callees
first, or give them to Opus (a larger `max_tokens` alone did not fix them, round 2). Keep Claude on
the generators, the typed-state shape and review. Untested: whether split pieces of a large routine
pass as reliably as routines that size do.
