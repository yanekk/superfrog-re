# T28 — compass-trial

**Split from:** T14 (its context limit, 2026-10-23). Carries T14's "the compass trial's numbers
are in one FINDINGS row with the choice made and why".

**Phase:** 3 · **Depends on:** T14, T27; blocks T15 · **Weight:** heavy

## Goal

T14's Run step 3 (DESIGN §2.2, §2.3): on W1-4, W2-3, W2-4, W6-2, W6-3, W6-4, unguided, run alone,
three seeds each, `sf-plan --compass straight` against `--compass map` and `--compass walk` (D25,
added by the owner 2026-10-24), and coin choice as a pull if built as a compass option here. Keep a
compass only if it completes more of these or plans them in measurably fewer engine frames;
otherwise the default stays straight (owner, 2026-10-22). If more than one beats straight, keep the
one that completes the most, then the fewest engine frames.

## The walk compass (D25, owner 2026-10-24)

Why: T27's unaided failures that never found a first exit (W2-3, W3-3, W3-4, W4-2, W4-4, W5-3,
W6-2, W6-4) mostly had the coins met and stalled 6–88 cells from the exit with 10k–36k cells
archived; guided W6-4 stalled at 13 cells twice. The straight pull points through walls. W6-1
stalls 1–3 coins short under the straight coin pull (`archive::coin_dist`).

Build it in `core` (pure, DESIGN §3.1), beside `MapCompass`:

- **Open cells.** A cell is open when `facts::live` gives a marker below `$3a` and it is not a
  hazard (marker 6). Read from the explorer's start state. A gate is a block, not one cell
  (`sf_core::items::fixtures::switch` rewrites five rows from the row above `Switch::gate`, its
  shape world by world): per `CellKey::switches` set, the cells of each pressed switch's
  `map::in_gate_block(s.gate, cell)` that are ground at the start are open (make the helper
  `pub(crate)`). Test it on W2-1's switch at 143,37.
- **Exit distance.** A breadth-first fill over open cells, 4-neighbour, from every exit cell,
  ignoring gravity and jumps: steps in cells. Filled lazily per switch set and cached, as
  `MapCompass` does; `Explorer::dist` runs single-threaded (`merge`, `seed_from`, `new`), so the
  cache needs no lock. A key's distance is the least over its 2×2 cells (`key.col * 2 + dc`,
  `key.row * 2 + dr`). A key with no open path to an exit falls back to the straight distance.
- **Coin distance.** With `--compass walk` only (straight and map keep `archive::coin_dist`, so the
  trial compares like with like), the last-8-coins pull (`Explorer::weights`, the `k.band <= 8`
  arm) uses the walking distance to the nearest object coin still live. `burst` runs on the shell's
  threads with `&self`, so it cannot fill: it records in `Held` the frog cell and which object
  coins are live (a bitset over `Facts::coins`), and `merge` computes the distance from one fill per
  (object coin, switch set), filled lazily there and cached. The frog's cell (`frog_cell`, its
  middle a row up) may sit in a wall mid-jump: take the least over it and its 4 neighbours, else
  straight. A coin with no open path counts as straight. Tile coins still never lower the count.
- **Memory.** A fill is 25 200 cells × u16 (~50 KB). The fills sit outside the archive's own count
  (D22); log their total once at the end (expected a few MB; tens with many coins and switches).
- **Units.** Distances are cells, as the straight compass, so `pull` and its constants are
  unchanged.
- **Option.** `--compass walk` in `sf-plan` (a third `CompassArg` variant, `Compass::Walk`); no map,
  no cache, nothing on disk. Its build time is measured in the shell around `Explorer::new` (no clock
  in `core`, `tests/purity.rs`) and goes in the log, so the trial can show it is negligible.
- **Determinism.** M8 holds: the fills are functions of the start state and the switch set only.

Tests (`sf-plan/tests/`): a fill on a hand-made grid goes around a wall where straight goes through
it; a gate opens only in its switch's set; a sealed cell falls back to straight; on one real level
the start key's walking distance is finite and at least the least straight distance over the same
2×2 cells; same seed, same route at
1 and 4 threads with `--compass walk`.

## Files

- `engine/crates/sf-plan/src/**`: the walk compass (`core`, `main.rs` option) with its tests, and
  the default if a compass is kept.
- `plans/smart-planner/FINDINGS.md`.

## Run

`--engine-only` is enough to compare planning (engine frames are machine-independent;
`PLAN_STATS` carries them and `rounds`); 30-min budget; the map compass's maps from the cache (built
once a level, its time recorded apart); walk needs none. Run each triple alone, or the three of a
triple side by side so they share the load, each with its share of the cores (`--threads`).
Worst case 6 levels × 3 seeds × 3 compasses × 30 min = 27 h; most runs end sooner.

## Done when

- [ ] `--compass walk` is built as above, its tests pass, and the run is the same at any thread count.
- [ ] The compass trial's numbers (straight, map, walk: completed, engine frames, rounds per level
      and seed) are in one FINDINGS row with the choice made and why.
