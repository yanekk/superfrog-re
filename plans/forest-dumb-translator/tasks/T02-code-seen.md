# T02 — code-seen

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal

Record every instruction address the original executes on the oracle, so the export can seed every entry and no code the game runs is left untranslated. Ghidra alone finds 3 837 instructions; the trial's 87 entries reached 3 962; level 1 alone is about 8 300 with callees.

## Design sections this implements

DESIGN §2.3.

## Files

- `fd-conform/src/code_seen.rs` and a `fd-conform code-seen` subcommand (new), reusing `code_writes.rs`'s instruction-address watch.
- `games/forest-dumb/re/entries.toml` (new, committed): addresses and sources only.

## Interface

```
fd-conform code-seen [--cases DIR] [--power-on-frames N] [--worlds 2,3,4,5,6]
# re/entries.toml
[[entry]] at = "h0+$1eda"  source = "symbol" | "vector" | "trace" | "program-entry"
```

`trace` entries are the starts of executed runs not reached by fall-through or a direct branch from another executed instruction (call targets, computed targets, interrupt entries), not every executed address, so the file stays small.

## How

Run level 1's 33 cases unfrozen, the power-on run through the title into level 1, and each world 2 to 6 start (poke `world_index`, as T01) for a bounded number of frames. Union the executed addresses; reduce to run starts.

## Tests

- [ ] Unit: a made-up executed set with a computed call and an interrupt entry reduces to exactly those starts.
- [ ] Full: two runs write identical `re/entries.toml`.

## Done when

- [ ] `re/entries.toml` committed with counts per source in the commit message and a FINDINGS row.
- [ ] Every symbol of kind `func` in `re/symbols.toml` that the runs executed appears as `symbol` or `trace`.
