# T59 — oracle-sprite-collision

**Phase:** 5 · **Depends on:** T21 · **Weight:** medium

Added by T21 with the owner's yes (2026-11-05). T24, T41 to T47 wait for it (`blocks`).

## Goal
The oracle reports sprite-to-playfield collision as the hardware does, so the original's contact rule can be measured and compared. `char_hit_check` (`h0+$3b2a`) reads CLXDAT; CLXCON is `$37dd` (play copper list `h1+$118`), so a playfield pixel of colour 21, 23, 29 or 31 under the character's sprites is contact, and three contact passes in a row cost a life. vAmiga leaves `DENISE_CLX_SPR_PLF` off by default and `amiga-oracle`'s shim never sets it, so CLXDAT reads `$8000` on every pass (T21, `object_claims.rs` `the_oracle_reports_no_sprite_collision_until_t59`): no creature, tile or item ever touches the character. The colours are in 134 of W1-1's 328 tiles, in coins, water and vanishing blocks, so movement cases may change too.
## Design sections this implements
§2.1, §2.2, §4.1.
## Reuses
`toolkit/crates/amiga-oracle/shim/shim.cpp` (where the other options are set), T31's cache-key handling (`docs/09`, FINDINGS: a changed oracle build rebuilds every sync cache once), T02's Superfrog route pins, T21's `object_claims.rs` slide harness.
## Files
`toolkit/crates/amiga-oracle/shim/shim.cpp` and whatever keys the oracle build or sync caches, a toolkit test showing CLXDAT reports a sprite over a matching playfield colour, `games/forest-dumb/engine/crates/fd-conform/tests/object_claims.rs` (the CLXDAT test and contact claims), `games/forest-dumb/docs/10-object-system.md` ("Contact" and the mechanics table), `toolkit/docs/architecture/` (the chapter naming what the oracle models), `smoke.toml` labels.
## Interface
None new. Turn on `DENISE_CLX_SPR_PLF` (and `CLX_SPR_SPR`, `CLX_PLF_PLF` if the bar needs them; say which and why). A snapshot or sync cache taken with collision off must not be reused with it on.
## Tests
- [ ] A toolkit test: a sprite over a playfield pixel matching CLXCON sets the CLXDAT bit; without the match it does not.
- [ ] In W1-1, a creature placed against the character makes `char_hit_check` see contact; stomped while `h0+$4ed0` is set it becomes kind 7 and scores 250; on the ground `hits_taken` counts to 3 and `lives` drops by one.
- [ ] Whether `$28` water and tiles of colour 21, 23, 29, 31 hurt, measured at a W1-1 spot each.
## Done when
- [ ] The check is on in the oracle.
- [ ] T02's Superfrog route pins and Superfrog's checks are unchanged. If switching it on changes any Superfrog result, stop and raise it with the owner; do not adjust Superfrog.
- [ ] Creature contact is re-measured and `docs/10-object-system.md` says what contact does, measured.
