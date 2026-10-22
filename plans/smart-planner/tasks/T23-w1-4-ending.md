# T23 — w1-4-ending

**Phase:** 2 · **Depends on:** T09, T10; blocks T14 · **Weight:** medium

Added during T10 (owner, 2026-10-22): T10's Done-when run of W1-4 guided by `complete-w1-4` timed
out at 30 min, and the explorer alone, engine only from the same start, stalls the same way.

## Goal

Find why the explorer, guided by `complete-w1-4`'s trace, never reaches W1-4's exit, and fix it in
general (no per-level option, DESIGN §1 Stance), so W1-4 guided plans in original mode within 30 min.

## What T10 saw

- Original mode, 10 threads, load ~10: 2 repairs, 5 330 frames confirmed, then the third stretch ran
  to the budget. Reason: coins met, least exit distance 15 cells, 17 769 cells.
- Engine only from `oracle::start_at_level(W1-4)` transplanted, guided, seeds 1, 2, 3 (9, 4, 4 min):
  every best partial ends at cell 158,45 with the coins met, distance 15, 25–29 k cells.
- Exits at 172–173, 41–43. The committed route's trace with the coins met: 159,27 → waits at
  158–159,29 for ~56 frames → 163,32 → 170,34 → 176,35 → 178,40 → 176,43 → 173,43. The search
  never follows it from 159,29.
- Suspects, not checked: a wait (a lift, an enemy, a timed object) at 159,29 that bursts seldom
  hold for; the straight exit pull (§2.4) drawing choices to the dead end below 158,45; guide
  regions in band 0 (the owner's last-8-coins band rule, FINDINGS T08) not matching where the
  archive meets the count.

## Design sections

DESIGN §2.4, §2.11, D19, D21.

## Files

Whatever the cause needs in `engine/crates/sf-plan/src/core/` (`explore.rs`, `guide.rs`, `archive.rs`)
and its tests; a constant changed is a general change, recorded in FINDINGS with its effect on a
level that already worked (W1-1, W2-1, W5-2 or W6-1).

## Tests

- [ ] A regression test or a recorded run showing the cause (the stall reproduced, then gone).
- [ ] Existing `sf-plan` explorer, guide and repair tests stay green; M8 holds.

## Done when

- [ ] W1-4 in original mode guided by `complete-w1-4` (`tests/oracle.rs`
      `w1_4_original_mode_guided_passes_the_check`) plans within 30 min and passes DESIGN §4.1's check;
      time, repairs, confirm share and hearts lost in a FINDINGS row (this is T10's unmet Done-when item).
- [ ] W5-2 unguided in original mode still passes (`w5_2_original_mode_unguided_passes_the_check`).
