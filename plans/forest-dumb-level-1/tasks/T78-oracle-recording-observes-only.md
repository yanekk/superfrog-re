# T78 — oracle-recording-observes-only

**Phase:** 6 · **Depends on:** T74; blocks T75 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T78` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T74's review with the owner's yes (2026-11-09). T74's runner reads where a pass cut `main_loop`'s edge redraws from line samples (`Amiga::set_profile`, `Amiga::line_samples`), whose doc says the run is unchanged. Measured: with creatures live it is not. On `enemy-route` and `enemy-contact` (creatures live since T46) a pass ends a step later from step 336 and 377 with sampling on; on the ten cases freezing `h0_681c` nothing differs. So `fd_conform::records_edge_cuts` limits T74's sampling and handing to those ten cases, and `enemy-route` sets `edge_latch_right` aside for T75. T75 needs the measured cut with creatures live.

## Goal
Find why sampling lines changes the emulated run with creatures live (the end-of-line trap in the oracle's vAmiga patch, `toolkit/crates/amiga-oracle`, against the blitter, sprite collision or DMA that drawn creatures add), and make it observe only: the run with sampling on any set of steps equals the run without it, every frame. Then lift the limit, or say measured why a part of it must stay.

## Design sections this implements
§2.1, §4.1.

## Reuses
T31's identity test (`amiga-oracle/tests/exec_counts.rs`, `watching_does_not_change_the_run`), T74's `Sampler`, `records_edge_cuts` and `fd-conform/tests/edge_latch.rs`'s `sampled_against_alone`, the oracle's pinned vAmiga build (`toolkit/tools/setup/oracle.sh`).

## Files
`toolkit/crates/amiga-oracle/**` (the sampling hook, its doc and tests), the oracle's patch set, `fd-conform/src/lib.rs` (`records_edge_cuts`), `fd-conform/tests/edge_latch.rs`, docs/09, `smoke.toml`.

## Interface
`Amiga::set_profile` unchanged in shape; its doc states what it is proven to leave unchanged.

## Tests
- [ ] An `amiga-oracle` test: a run with line sampling on, toggled on and off between steps, equals the run without it every frame in chip and slow RAM, picture and CPU registers, on a workload with blits and sprites (T31's form), failing before the fix.
- [ ] `sampling_changes_the_original_where_creatures_are_live` turned round: `enemy-contact` and `enemy-route` sampled every step equal the original alone every frame.
- [ ] `sampling_for_the_edge_cut_leaves_the_original_unchanged_every_frame` covers every case the runner samples, live ones included.

## Done when
- [ ] `records_edge_cuts` no longer depends on creature starts being frozen, or docs/09 states, measured and dated, why it must.
- [ ] The oracle's cache keys change if the fix changes emulation, and Superfrog's oracle smoke stays green.
- [ ] `set_profile`'s doc says what the identity test proves.
