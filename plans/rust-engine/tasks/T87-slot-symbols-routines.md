# T87 — slot-symbols-routines

**Split from:** T71

**Phase:** 2 · **Depends on:** T71; blocks T72 · **Weight:** light

Carries T71's "Done when" item for the slot's routines in `re/symbols.toml` (T71 stopped at the
context line; its symbols were discarded uncommitted, the list below is what it had written).

## Goal

The slot machine's routines are named in `re/symbols.toml`, so listings and later slot tasks read
`slot_lamps_off` instead of `h0+$6238`.

## What to build

`[[sym]]` entries, `exe = "d1_c03"`, `hunk = 0`, `kind = "func"`, appended under a
`# --- the slot machine (T71; docs/13) ---` heading, with a one-line note each:
$4e28 `slot_free_spin`, $4eae `slot_payout`, $50b6 `slot_count`, $51c4 `slot_award` (awards the
prize `d7` names, lighting the ladder), $5528 `slot_collect`, $5628 `slot_jackpot`, $57e2
`slot_hilo`, $598e `slot_hilo_spin`, $5a06 `slot_spin`, $5d48 `slot_reel_stop`, $5de2
`slot_hold_menu`, $5eb4 `slot_menu` (pointer menu; fire picks the stop), $6238 `slot_lamps_off`
(clears 26 lamp records), $6258 `slot_lamps_on`, $6276 `slot_attract` (one attract frame: reels
0,9,$18, a pattern row into the lamps), $62f4 `slot_digits`, $645e `slot_draw_lamps` (blits each
lamp on the sixth plane), $6558 `slot_draw_reels` (from FRUC), $661e `slot_init` (reels 0,$b,$16,
lamps pointed at their rects), $6f3a `print_text` (also the tally's and the info screen's; sets
`text_plane`). Check each name is unused first (none was on 2026-10-13). `slot_count`, `slot_hilo`,
`slot_menu` and `slot_attract` are T71's LAYOUT variables ($515a, $5988, $6152, $62ec), whose names
are the ones in `re/symbols.toml`: give those four routines other names.

## Files

- `re/symbols.toml`

## Interface

The routine names above, for listings and the slot tasks' doc comments; no code reads them.

## Tests

- [ ] `venv/bin/python -m unittest tools.test_re_listing` green.
- [ ] `tools/re_listing.py d1_c03 --out /tmp/x` runs and labels `slot_lamps_off:` at $6238.

## Done when

- [ ] The twenty routines named and committed, the listing test green.
