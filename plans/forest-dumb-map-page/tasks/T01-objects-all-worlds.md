# T01 — objects-all-worlds

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal

Every object value in all six levels must have a group the page can draw it by, and an unknown value must
be caught rather than named by guess. W1-1's table covers every value in worlds 1 and 2; worlds 3 to 6 add
only `$34` (the flame, read in docs/10, not measured). Measure `$34` on the oracle and add a game-wide table
for values W1-1 lacks.

## Design sections this implements

DESIGN §2.1, §1 stance.

## Files

- `games/forest-dumb/re/objects-more.toml` (new): the same `[[value]]` shape as `objects-W1-1.toml`
  (`value`, `group`, `name`, where seen, how measured).
- `games/forest-dumb/engine/crates/fd-conform/tests/object_claims.rs`: a placed-`$34` claim.
- `games/forest-dumb/engine/crates/fd-assets/tests/` (new test file): every world's values named.
- `games/forest-dumb/docs/10-object-system.md`: `$34` measured; `games/forest-dumb/smoke.toml`.

## Interface

```toml
[[value]]
value = 0x34
group = "flame"            # its own group; the page draws it on the springs/blocks/exit layer
name = "flame"
seen = "W3-1:c,r"          # an example cell, filled from the map
measured = "placed, object_claims.rs::flame_changes_no_state"
```

## Tests

- [ ] `$34` placed on the docs/10 path (as the other placed claims): no state variable changes on contact or
  in view other than the picture; `_Plomien` drawn (the frame counter T31's way).
- [ ] Every non-zero value in the used columns of `_MapaBobow1` … `6` is in one of the two tables (needs the disks).
- [ ] A made-up map with an unknown value: the test names the value and the cell.
- [ ] Every `re/objects-*.toml` present is read (a per-world one from plan forest-dumb-level-2 included);
  a value two tables give different groups fails, naming both files (DESIGN §2.1).

## Done when

- [ ] `objects-more.toml` holds `$34`, measured, and docs/10 says so, dated.
- [ ] The all-worlds test passes and is labelled (smoke if it is under a few seconds).
