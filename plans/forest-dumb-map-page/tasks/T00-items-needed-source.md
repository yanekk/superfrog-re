# T00 — items-needed-source

**Phase:** 0 · **Depends on:** — · **Weight:** light

## Goal

The page must show how many items each level's exit needs, and that number is `items_left` after
`level_init`, not the coin count (W1-1: 332 coins, 282 needed). It is set by `load_list_world_N` (docs/10)
but is not a plain `move.w #$0282` (FINDINGS). Find where each world's start value comes from, so T02 can
read it without running the game, and read it once on the oracle for all six worlds. Throwaway code.

## Design sections this implements

DESIGN §2.2.

## Files

Throwaway under `games/forest-dumb/spikes/items-needed/` (deleted at the end, its README kept); the answer
goes in `games/forest-dumb/docs/10-object-system.md` ("Items") and a FINDINGS row.

## What to establish

- The instruction(s) in `load_list_world_1` … `_6` (hunk 4, `re/symbols.toml`) or elsewhere that write
  `items_left` (`h0+$6a88`, BCD word) before play, read with `tools/re_listing.py`.
- Each world's value on the oracle: run `cases/world-N-start.toml`'s way in (`world_index` set at
  `load_world`, docs/14) and read `items_left` at `level_init`'s return; world 1 from power-on.
- Which of three it is, and what T02 does for each: a constant at a fixed place per world (read the program's
  bytes), a value computed from the map (a pure function of `_MapaBobowN`), or a value only the run produces
  (T02 runs the translated program headless to each world's `level_init` when the page is written).

## Done when

- [ ] docs/10 states the source and the six oracle values, dated; a FINDINGS row says which case it is.
- [ ] The spike code is deleted; its README says how to rerun the reading.

## Needs a person

Nothing.
