# T83 — red-cut-tests

**Phase:** 6 · **Depends on:** —; blocks T29 · **Weight:** light

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T83` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T48 with the owner's yes (2026-11-10), under DESIGN §4.2: two tests of `fd-conform` are red on the feature branch. Both were red at `53638b37`, before T48 changed anything (run there in a clean checkout, release), and both hand a case to a wrapper of `Game`.

## Goal
Both tests pass again, each still failing when the thing it guards is taken away. The fix commit ends with the trailer `Escape: forest-dumb-level-1 T80` (owner, 2026-11-10).

## The two tests, as measured (2026-11-10)
1. `fd-conform/tests/edge_latch.rs`, `the_engine_fails_without_the_measured_cut_or_the_scan_it_stopped` (a smoke `[[area]]`). It expects `mechanic-vanish` with the measured cut dropped (`Hand::Dropped`) to differ on an `edge_latch_` variable before pass 400. It now stops earlier: `picture differs at pass 313 (oracle frame 6615): 216 pixels differ, 3 frames in a row`, first at (273, 68), `103` in the original and `000` in the engine. The case itself passes unwrapped.
2. `fd-conform/tests/contact.rs`, `the_waters_frames_change_contact_in_the_pool_as_the_originals`. `death-water` through the test's `Copies` wrapper: `picture differs at pass 1004 (oracle frame 7306): 421 pixels differ, 3 frames in a row`, first at (24, 0). `Copies` forwards `foreground_cut` and `cut_redraw` and not `creatures_uncut` (T80). The case itself passes unwrapped.

Not measured: which commit turned each red; that the missing `creatures_uncut` is the whole of the second.

## Design sections this implements
§4.1, §4.2.

## Reuses
`fd_conform::Engine` (a wrapper forwards `cut_redraw`, `foreground_cut` and `creatures_uncut`), FINDINGS 2026-11-09 (T74 review: the wrappers that forward neither), `creature_cuts.rs`.

## Files
`fd-conform/tests/edge_latch.rs`, `fd-conform/tests/contact.rs`; any other wrapper of `Game` in `fd-conform/tests/` found red the same way.

## Interface
None.

## Tests
- [ ] Both tests pass, in the debug profile the test sets use.
- [ ] `edge_latch.rs`'s run with the measured cut dropped still ends on an `edge_latch_` difference, not on the picture, and its run with the scan whole on a picture difference.
- [ ] Every wrapper of `Game` in `fd-conform/tests/` is listed with what it forwards; one that meets a creature cut behind the beam forwards `creatures_uncut`.

## Done when
- [ ] The tests above pass; the fix commit carries `Escape: forest-dumb-level-1 T80`.
- [ ] `tools/smoke.sh --check-only` is clean.
