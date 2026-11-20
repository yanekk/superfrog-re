# T04 — movement-map

**Phase:** 1 · **Depends on:** T03 · **Weight:** heavy

## Goal

The real movement map: for every node (a standable cell, plus speed band and facing if T00 adopted
them, D15), the macro moves the frog can make and where each really lands, measured by stepping the
engine, with the coins each touches on the way and switch and wing preconditions. It is what replaces
the geometry-only distance field and is the reason this plan exists.

## Design sections this implements

DESIGN §2.2 (as amended by T00's review), §3.1, §3.2 `core::map`, D15, D17.

## Files

- `engine/crates/sf-plan/src/core/map.rs`, `engine/crates/sf-plan/src/core/moves.rs`
- `engine/crates/sf-plan/tests/map.rs`

## Interface

```rust
pub struct MoveSet { pub families: Vec<MoveFamily> }        // Walk, Jump, RunJump{run_up}, Drop, Launch, Fly
pub fn default_moves(wings: bool) -> MoveSet;
/// Work split for the shell's threads: each part is a slice of seed cells; results merge in part order.
pub fn parts(facts: &Facts, n: usize) -> Vec<MapPart>;
pub fn build_part(start: &Game, facts: &Facts, moves: &MoveSet, part: &MapPart) -> Vec<Edge>;
pub fn assemble(facts: &Facts, parts: Vec<Vec<Edge>>) -> MoveMap;    // dedup: keep the fewest-frames edge per (from,to,needs)
pub fn build(start: &Game, facts: &Facts, moves: &MoveSet) -> MoveMap; // single-thread convenience
```

Edges end when the frog has landed (`facts::landed`), at whatever speed it carries; no frames are
added to stop it. Each edge records the `Facts::coins` it touches on the way. A move whose path touches
a `Facts::hazards` cell is not recorded.

Switch sets: edges through a gate carry `needs_switches`; cells behind gates are measured from a state
with that switch pressed (written into the state the way the switch handler clears both markers).

## Tests

- [ ] Every edge of the W1-1 map replays on the engine from its seed state to its `to` node, landed,
      alive, touching exactly its recorded coins (test block).
- [ ] A random 200-edge sample per level on W2-4, W4-2, W6-2 replays the same way (test block if under
      30 s total, else `#[ignore]` with its time noted).
- [ ] No edge's path touches a hazard cell (W6-4 spikes at row 10 are crossed only by wing edges).
- [ ] If speed bands are adopted: on a fixture run-up, a full-speed node's jump lands farther than the
      stopped node's (the reason for D15).
- [ ] W4-2's exit is reachable only with the switch at 222,4 pressed (FINDINGS T103).
- [ ] Every level W1-1…W6-4: in the map (no enemy help), the exit is reachable from the start through
      at least `coins_needed` reachable coins (DESIGN §2.1, D13). Any level that fails is listed in a
      FINDINGS row with where the map stops.
- [ ] W1-4 and W6-3 stall spots from T00 are judged as T00 found.
- [ ] `build` equals `assemble(parts(...).map(build_part))` for 1 and 8 parts (determinism).

## Done when

- [ ] Tests green; build time per level for all 24 levels recorded in one FINDINGS row (10 threads).
- [ ] No level's map takes over 5 min to build, or the owner has been asked.
- [ ] Any level failing the reachability test has been brought to the owner (it may need an enemy, a
      stomp reward or a knock; D13), and the answer is recorded, before T08 starts.
