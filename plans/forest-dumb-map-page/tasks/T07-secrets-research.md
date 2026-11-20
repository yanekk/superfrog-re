# T07 — secrets-research

**Phase:** 1 · **Depends on:** — · **Weight:** heavy

## Goal

The owner wants the map to show the game's secrets, confirmed and never guessed. Find out what is hidden:
cells drawn as wall that the character passes through, cells drawn empty that are solid, and anything else
the code hides (a warp, a hidden room, a value that does something only when touched). Confirm each kind on
the original, write the rule that lists its cells from the level files, and code that rule.

## Design sections this implements

DESIGN §2.4.

## Files

- `games/forest-dumb/docs/17-secrets.md` (new): what was looked for, where in the code, the rule, each kind's
  oracle evidence, and the count per world; a line in `games/forest-dumb/CLAUDE.md` Notes.
- `games/forest-dumb/engine/crates/fd-assets/src/secrets.rs` (new, pure), `src/lib.rs`.
- `games/forest-dumb/engine/crates/fd-conform/tests/secret_claims.rs` (new, oracle).
- `games/forest-dumb/smoke.toml`.

## What to establish

- How collision is decided: `collision_map` from the shapes (`_GenN-1.Mapa`), vanishing blocks writing it
  (docs/10), and whether a tile's picture says anything to the code at all.
- A rule for "drawn as wall" that a person would agree with: for example a tile whose shape is solid in most
  of its cells across the level, met here with an empty shape. The first count is in FINDINGS (44 of 328 tiles
  in W1-1 have more than one shape); slopes, edges and decoration must fall out of the rule, not be listed.
- Whether any routine teleports the character, changes `world_index` or the camera mid-level, or reacts to an
  object value no map's visible picture shows.

## Interface

```rust
pub enum SecretKind { HiddenPassage, InvisibleFloor /* , others T07 finds */ }
pub struct Secret { pub c: u16, pub r: u16, pub kind: SecretKind }
pub fn secrets(level: &Level) -> Vec<Secret>;   // only kinds confirmed on the oracle
```

## Tests

- [ ] Made-up level: the rule's edges (a solid tile met once empty, a slope that must not count).
- [ ] Oracle (full), per confirmed kind and at least one cell in each world that has one: the character placed
  beside it and moved into it passes (passage) or stands (floor), as the rule says.
- [ ] A kind with no oracle confirmation is not in `SecretKind`.

## Done when

- [ ] docs/17 states each kind with its evidence and per-world counts, dated; or that a kind was looked for and
  not found, and why.
- [ ] `secrets()` and its tests pass; the oracle tests are labelled full.

## Needs a person

Only if a found kind has two defensible readings (for example a passage that is also a shortcut the designers
may have meant as a normal path): ask the owner whether it counts as a secret, with the cell and a screenshot.
