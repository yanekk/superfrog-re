# T64 — listing-odd-bytes

**Phase:** 2 · **Depends on:** T08, T51 · **Weight:** small

Added by T08 with the owner's yes (2026-11-05), by DESIGN §4.2 (a test red on the base branch is raised as a task). A fix of T51's work: commits carry `Escape: forest-dumb-level-1 T51`.

## Goal
Make `fd-tools/tests/exe_map_claims.rs::the_listing_regenerates_and_decodes_hunk_4` green again. Since T51 it fails with "73 lines of hunk 0 start odd, first `00004be5 ...`": one-byte `var` symbols in `re/symbols.toml` (`char_walk_t` at `h0+$4be4`, `char_facing`, `char_probe_1`.. at `h0+$4ece`.., and T08's `char_jumped`, `char_jump_t`, `char_unhurt_t`, `char_rising`.. ) leave the listing's linear sweep on an odd address, and it decodes noise past the next symbol (`char_step_4be6`, the anim tables at `h0+$4edc`, `char_sprite_copy` at `h0+$9b06`).
## Design sections this implements
§4.2.
## Reuses
`games/forest-dumb/tools/re_listing.py`, `toolkit/tools/hunk_listing.py` (shared with Superfrog: its listing must not change), the `re/symbols.toml` header rule "keep it even where code follows".
## Files
`toolkit/tools/hunk_listing.py` or `games/forest-dumb/tools/re_listing.py`, their tests, possibly `fd-tools/tests/exe_map_claims.rs` (the check may exempt the `dc.b` line of a byte `var` at an odd address, never an instruction line).
## Interface
None new. Byte variables keep `size = 1` (`fd-core`'s LAYOUT test requires size = width).
## Tests
- [ ] A listing test: a byte var followed by a func symbol on the next even address resyncs there.
- [ ] Superfrog's listing unchanged (its own listing test, or a byte-for-byte compare before and after).
## Done when
- [ ] `the_listing_regenerates_and_decodes_hunk_4` passes with every byte var in `re/symbols.toml`.
- [ ] `char_step_4be6`, the walk tables and `char_sprite_copy` list as instructions and data again.
