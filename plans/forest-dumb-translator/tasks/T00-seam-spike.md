# T00 — seam-spike

**Phase:** 0 · **Depends on:** — · **Weight:** medium

Throwaway. The code is deleted at the end; only the answer stays, in FINDINGS and this doc's Notes.

## Goal

Settle how level 1's hand engine (`fd-core`) plugs into the translated program before T35 builds the seam. The hand engine keeps typed state copied from memory one way (`Transplant`); mounting it needs the way back. Two granularities are possible, and the answer decides T35's interface and T36's work.

## Design sections this implements

DESIGN §2.9.

## The question and what each answer means

Try both on the trial's tree (`build/translator/`, the 87 routines, `fd-judge --translated`), each with the smallest real example:

- **Per routine.** Mount two `PORTED` routines (one leaf, e.g. `char_sprite_position` `h0+$5462`, and one with callees, e.g. `camera_update` `h0+$4d7e`): an adapter reads `LAYOUT` vars from the bus into `fd_core` state, calls the ported function, writes changed vars back. Judge mounted against translated on the routine oracle. If it holds: T35 builds a per-routine adapter, and T36 mounts every `PORTED` routine on level 1's path.
- **Per level frame.** While `world_index` is world 1 and `game_running` is set, `game_frame` (`h0+$1fec`) is replaced by the hand `Game::step` with transplant in and write-back out once a frame. If it holds and per routine does not: T35 builds a frame-level mount, and T36 mounts the frame.

Measure for each: does it pass, what state the hand engine holds that `LAYOUT` does not (collision plane, display, colours, object tables), and the write-back's size. Recommend one in FINDINGS with the numbers.

## Files

Scratch only: a branch-local test under `fd-conform/tests/` and an adapter module, both deleted before the task ends. Nothing in `fd-core` changes.

## Tests

- [ ] Per routine: the two routines mounted, judged on 2 000 states each against translated.
- [ ] Per frame: at least one level 1 case's first 100 frames with the frame mount, against the translated tree and the hand engine alone. If the translated tree cannot run a whole frame yet (no machine), measure write-back completeness instead: transplant, write back, transplant again, and compare state equality.

## Done when

- [ ] FINDINGS has the measured answer for both granularities and a recommendation, dated.
- [ ] The scratch code is deleted; the branch carries only the FINDINGS and PROGRESS rows.
