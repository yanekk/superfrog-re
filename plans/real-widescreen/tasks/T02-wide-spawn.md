# T02 — wide-spawn

**Phase:** 1 · **Depends on:** T01 · **Weight:** medium

## Goal

In widescreen, objects wake as their cell reaches the wide view's edge and are freed after they leave
it, by the original's margins measured from the wide edges, in normal levels and in the bonus level's
auto-scroll. This is the core of "the strips are real game".

## Design sections this implements

DESIGN §2.2, §2.3.

## Files

- `engine/crates/sf-core/src/level/spawn.rs` (`scan_origin`, `scan_edges`, `scan_screen`,
  `scan_right_column`)
- `engine/crates/sf-core/src/level/objects.rs` (`obj_despawn_if_offscreen`, `obj_free_if_left_behind`,
  `find_free`'s off-screen test, the coin force-out's visible test)
- `engine/crates/sf-core/src/level/screen.rs` (the methods these use)

## Interface

Unchanged signatures; each reads `c.screen`. In widescreen: origin = cell at (cx − $20 − 5 cells,
cy − $20), clamped at 0; edge column at origin + $2e + 2·5 cells' bytes (or origin when moving left);
edge row 25 + 10 cells wide; `scan_screen` 24 + 10 cells wide; bonus right column at the same moved
offset; despawn right at $170 + 65, left at $30 + 65 + width; `obj_free_if_left_behind` at cx − 65 −
width. Vertical margins untouched. Beam points (`c.point(n)`) stay where they are.

## Tests

- [ ] spawn: an object marker at the cell just inside the wide right edge spawns in widescreen and not in
  4:3 (moving right); same at the left edge moving left; a row above/below spans the wider width.
- [ ] despawn: an object 1 px inside the wide right threshold survives in widescreen and is freed in 4:3;
  at the threshold it is freed in both; left side with its frame width.
- [ ] bonus: the right column and the despawn move together, so a spawned enemy is not freed in the
  pass it spawned (regression for DESIGN §2.3's warning).
- [ ] `find_free` treats an object in the strip as on screen in widescreen (not evicted), off screen in
  4:3.
- [ ] at the level's left edge the widened origin clamps at 0 and does not wrap.
- [ ] grep test or review note: no literal $170/$30 horizontal test left outside `screen.rs` in these files.

## Done when

- [ ] Tests green; with the mode off every touched function computes what it did (existing tests).
- [ ] `cd engine && cargo test -q --workspace` run once, recorded in PROGRESS.
