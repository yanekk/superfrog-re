# T72 — water-contact

**Phase:** 6 · **Depends on:** T25, T70; blocks T47, T27 · **Weight:** small

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T72` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T70's review with the owner's yes (2026-11-06). T70's goal was a life lost where the original loses one "on W1-1's spikes and water". The spikes are shown (`death-pit`, `death-spikes`, `contact-route`); the water is not, because the engine draws no water until `h0_62b6` (the animated cells: items, water, vanishing blocks) is ported, which `re/translate.toml` gives T25, and a frozen case stops the original drawing it (`freeze.rs`, `h0_6882`, T25). T59 measured `$28` water under the feet costing a life (`object_claims.rs` `water_under_the_feet_costs_a_life`, contact passes 107 to 109; docs/10 "Contact").

## Goal
Water costs a life in the engine as it does in the original: T70's collision model (`render::contact`) and `char_hit_check` on the water T25 draws, compared pass by pass and frame by frame at a W1-1 water spot.

## Design sections this implements
§2.4, §4.1.

## Reuses
T70's `render::contact`, `death-pit` and `contact.rs`; T59's water measurement (`object_claims.rs`); T25's port of `h0_62b6` and its freeze lifted.

## Files
`engine/conform/cases/death-water.toml`, `engine/conform/scripts/death-water.txt`, `fd-conform/tests/contact.rs` (or `death.rs`), `smoke.toml`; `fd-core/src/**` (fixes only).

## Interface
None new.

## Tests
- [ ] `death-water`: the character into `$28` water on W1-1, with `h0_6882` no longer frozen (T25): `clxdat_copy`, `hits_taken`, `lives`, `char_dying` and the character on every pass, and the picture on every frame, equal the original's through the life lost and its hold.
- [ ] The water's frames carry colour 29 where the character touches it: a pass with a contact bit set comes from the water's pixels, not a tile's (shown by the case's CLXDAT turning on only where the water is drawn).

## Done when
- [ ] `death-water` passes with no exclusion naming T25 or T72.
- [ ] docs/10 "Contact" states the water spot and passes measured, dated.
- [ ] A smoke label for the new test; the `[[area]]` "contact and a life lost" covers it if it fits the smoke budget.
