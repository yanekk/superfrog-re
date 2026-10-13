# T88 — slot-symbols-tables

**Split from:** T71

**Phase:** 2 · **Depends on:** T71; blocks T72 · **Weight:** light

Carries T71's "Done when" item that the slot's tables have their exe offsets documented in
`re/symbols.toml` (T71 reads them through `Tables`, with offsets in the field docs; the symbols were
discarded uncommitted at the context line).

## Goal

The slot machine's six tables are named and sized in `re/symbols.toml`, matching
`sf_assets::Tables`' `slot_*` fields.

## What to build

`[[sym]]` entries, `exe = "d1_c03"`, `hunk = 0`, `kind = "table"`, under the slot heading (T87's,
or add it): `slot_ladder` $57b6 size $2c (ten `slot_lamps` pointers, 0-ended); `slot_pointer_stops`
$606a size $c ($ffff-ended); `slot_reel_frames` $668a size $80 (32 × x,y in FRUC);
`slot_lamp_rects` $670a size $138 (26 × x,y,w,h + 2 words); `slot_attract_later` $3b1ae size $13a
(6 rows of 26 words + $ff80); `slot_attract_first` $3b2e8 size $1a2 (8 rows + $ff80). Sized
symbols may not overlap: `anim_record` ends at $3b1ae, the two patterns abut.

## Files

- `re/symbols.toml`

## Tests

- [ ] `venv/bin/python -m unittest tools.test_re_listing` green.
- [ ] `tools/re_listing.py d1_c03 --out /tmp/x` runs and labels `slot_ladder:` at $57b6.

## Done when

- [ ] The six tables named and sized, committed, the listing test green.
