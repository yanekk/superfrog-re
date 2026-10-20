# T03 — level-facts

**Phase:** 1 · **Depends on:** T00 · **Weight:** light

## Goal

One shared, tested source for the level facts every planner needs, replacing the four hand-copied
versions in the old examples, and the `sf-plan` library with its core/shell boundary and the purity
test that guards it.

## Design sections this implements

DESIGN §3.1, §3.2 (`core::facts`), §2.3 (coin sources), §2.2 (standable cells, switches/gates).

## Files

- `engine/crates/sf-plan/Cargo.toml` (new crate, library only; deps sf-core, sf-assets, serde; the
  workspace's `crates/*` glob picks it up)
- `engine/crates/sf-plan/src/lib.rs`, `engine/crates/sf-plan/src/core/{mod,facts,graph}.rs`
- `engine/crates/sf-plan/tests/purity.rs`, `engine/crates/sf-plan/tests/facts.rs`

## Interface

```rust
pub type Cell = (u16, u16);                                  // (col, row), 300×84
pub struct Facts {
    pub level: LevelId,
    pub standable: Vec<Cell>,
    pub coins: Vec<Coin>,                                     // Coin { cell, kind: Object(slot) | Tile }
    pub switches: Vec<Switch>,                                // Switch { cells: [Cell; 2], gate: Cell, marker: u8 }
    pub exits: Vec<Cell>,
    pub coins_needed: u16,
}
pub fn facts(game: &Game, t7mp: &T7mp) -> Facts;             // from the live map in memory + static T7MP
pub fn frog_cell(st: &State) -> Cell;                        // ((X+0x17)/16, (Y-0x10)/16)
pub fn standing(st: &State) -> bool;
// graph.rs: the shared graph type T04 fills and T06/T08 read
pub struct MoveMap { pub cells: Vec<Cell>, pub edges: Vec<Edge>, pub out: Vec<Range<u32>> }
pub struct Edge { pub from: u32, pub to: u32, pub inputs: Vec<(u8 /*Joy bits*/, u16 /*frames*/)>,
                  pub frames: u32, pub needs_switches: u32 /*bitset*/, pub needs_wings: bool }
pub fn shortest(map: &MoveMap, from: u32, switches: u32, wings: bool) -> Vec<u32>;   // Dijkstra on frames
```

Seeding and world-holding fields follow T00's recommendation as recorded in DESIGN §2.2.

## Tests

- [ ] Purity test: `src/core/` uses none of `std::time`, `std::fs`, `std::env`, `std::process`,
      `std::net`, `Instant`, `sf_oracle`, `sf_conform` (DESIGN §3.1: the T7MP arrives as an argument); the
      test names the rule and says to move code, never relax the test.
- [ ] `facts` on W1-1 from its start: coins_needed and coin count match the level (90 coins, T106).
- [ ] Every level W1-1…W6-4: at least one exit, `standable` non-empty, every switch has a gate.
- [ ] `frog_cell` matches the old planners' formula on a fixture state.
- [ ] `shortest` on a hand-built graph honours `needs_switches` and `needs_wings`.

## Done when

- [ ] `cargo test -p sf-plan` green in the test block.
- [ ] No old example is edited (they are retired in T18).
