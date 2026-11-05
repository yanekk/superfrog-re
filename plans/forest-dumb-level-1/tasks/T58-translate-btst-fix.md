# T58 — translate-btst-fix

**Phase:** 1 · **Depends on:** T33 · **Weight:** small

Added by T35 with the owner's yes (2026-11-05). T40 waits for it (`blocks T40`).

## Goal
`tools/translate.py`'s `flow_text` follows only real branches, so a routine's listing and its size for `too-big` are its code and nothing else. Today it treats any opcode starting with `b` whose last operand is a label as a branch, so `btst.b #n,h0+$4ed2` is followed into the data at `h0+$4ed2` and on into `char_probe`. Measured by T35: `h0_3ea8` 371, `h0_4718` 348, `h0_48be` 348, `h0_4b0e` 295 against T34's units of 127, 104, 104 and 51, so T40 would answer `too-big` for all four and the prompt would carry unrelated code.
## Design sections this implements
§4.3.
## Reuses
`tools/call_tree.py`, whose own decoding T34 checked against these sizes; `tools/test_translate.py`'s stubbed model.
## Files
`games/forest-dumb/tools/translate.py`, `games/forest-dumb/tools/test_translate.py`, `games/forest-dumb/docs/11-translation.md` (only if it describes the extraction).
## Interface
Unchanged: `flow_text(hunk, off, stops)` returns `(text, instructions)`.
## Tests
- [ ] A listing with `btst.b #0,<label>` and `bset`/`bclr`/`bchg` to a label: none of them is followed.
- [ ] `Bcc`, `DBcc`, `bra`, `bsr`, `jsr`, `jmp` are still followed as before.
- [ ] On the real listing, the sizes `flow_text` gives with `split_at` equal each routine's `unit` in `re/translate.toml` for the T35 and T53 routines (`h0_3ea8` 127, `h0_4718` 104, `h0_48be` 104, `h0_4b0e` 51), or the difference is named with why.
## Done when
- [ ] The four routines above are no longer `too-big`.
- [ ] The fix commit ends with the trailer `Escape: forest-dumb-level-1 T33`.
- [ ] T33 and T40's task docs are not edited.
