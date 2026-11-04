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

## Next if adopted

Raise Muse's `max_tokens` above 32 000 for routines over ~300 instructions; split large routines at
their callees; have the level 1 plan's porting tasks send translation to Muse through this judge and
keep Claude on the generators, the typed-state shape and review.
