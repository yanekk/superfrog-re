# T18 — bosses

**Phase:** 2 · **Depends on:** T16, T17, T180, T182 · **Weight:** medium

> **Rewritten** by the owner's amendment of 2026-10-15. The original asked for "every world's boss
> level"; docs/09 "Bosses" found there is none per world: each world ends with its `LnET` picture
> (T183), and the one boss is the final level's witch. The re-split of 2026-10-11 already said
> "T18 becomes the final level and the witch". "Green" means DESIGN §2.1's faithful checks.

## Goal

The final level (its `LW*` files, `final_level_flag` h0+$1e) and its boss: W6/final-level marker 1,
template `$221f6`, init `$259ae`, update `$25a16`, touch `touch_boss_hit` (`$236b4`), 128 hit
points, spawned 8 rows above its marker cell. Its attack patterns and phases, hit counting, the
palette flash on defeat (`level/fade.rs` mode 1), `level_state = $64`, and the hand-over to the end
sequence (T35). Any other object type the final level has that no world task ported.

## Design sections this implements

DESIGN §2.1, docs/09.

## Files

- `engine/crates/sf-core/src/bosses/{mod.rs,witch.rs}` (names from `symbols.toml`)
- `engine/crates/sf-core/tests/bosses.rs`, `engine/conform/{scripts,cases}/final-*`

## Interface

One `ObjectKind` for the witch, registered into T14's dispatch; its outcome reuses `LevelOutcome`.

## Tests

- [ ] routine conformance for the witch's update and touch routines.
- [ ] `final-win`: the final level from its start, the witch beaten, into the end sequence; faithful
      and alone.
- [ ] `final-lose`: the frog dies to the witch; faithful.
- [ ] `final-idle`: 3 000 frames in the arena without hitting her, every attack pattern seen; faithful.

## Done when

- [ ] The three cases green in `sf-conform -- all`, one short one in the test block.
- [ ] Boss variables in `LAYOUT`, names in `symbols.toml`.
- [ ] No case file excludes anything the final level does.
