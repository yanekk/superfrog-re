# T196 — pass-over-two-blanks

**Phase:** 2 · **Depends on:** T184, T150; blocks T21 · **Weight:** heavy

Added by T150 with the owner's yes, 2026-10-16. `enemies-w6-4-complete` (T150) reaches W6-4's exit
on the original but fails checks 1–3 and check 4, both where the original's pass runs late over
more than one frame. T146, which owned W6 completion faults, was dropped, and T184 (alone-pass-drift)
did not cover these phases. The owner gave this task the whole family (below).

## Goal

Make the engine take a late pass the original holds over more than one frame as the original does,
so that every case below is green under `sf-conform -- run` (checks 1–4), and put each in the test
block. No threshold is changed (DESIGN §2.1).

The family:

1. **W6-4, frames 150–153 (checks 1–3).** Late frames 150 (`blanks` 1), 151, 152 (`blanks` 1), all
   phase 5, then 153 (phase 3, step 1). At 154 the engine has taken three coins (`coins_left_for_exit`
   `$60`) the original takes a pass later (`$63`; its 96 at 156). With re-sync the case then fails
   only on sound: a channel-3 stop at 155 the original lacks (reported at 158).
2. **W6-4, frames 5228–5229 (check 4).** The original holds one pass over the late frames 5228
   (phase 5, `blanks` 0) and 5229 (phase 5, `blanks` 1); its `vbl_count` is 2 at the end of 5230 and
   its passes begin a frame later from then on. The engine run alone does not: its objects (update
   routines `$27f2e`, `$27fd4` among them) move at 5231 where the original's move at 5232, and stay a
   frame ahead. The frog matches to 5468; at 5469 the engine's is hurt by an object (hearts 4→3) the
   original's misses, and the milestones part at play frame 5517 (a coin, the engine's at 5622).
3. **W2-2 (T166, for the dropped T155).** `w2-2-complete` red under checks 1–3 at 1229 (`hitboxes`
   order; 1227 `blanks: 1`, 1228 cut at phase 1) and alone at 4570 (tile coin 101,49; the engine's
   99,47 at 4700). Check whether it is the same fault.
4. **W6-2, frame 6661 (T194).** `enemies-w6-2-complete`'s channel-3 sound event, a `blanks: 2`
   frame inside a lag stretch: it looks like the same fault. Once this task lands, T194 should close
   on a re-run of its case.

## What T150 measured

- Pacing per frame comes from `sync::pacing` on the original (T150's scratch probe printed it with
  `step_inserting` and `sync::core_memory`; not committed). Frames 154–156, 158, 5227, 5230, 5232
  are on time; 5231 and 5233 are late, phase 5, `blanks` 0.
- In the alone run (`Alone::Only`, `run_with`'s hook gets the engine alone), comparing the 20 object
  slots (h0+$1ba6c, `$7c` each: live `+$40`, update `+4`, x `+8`, y `+$a`) against the original frame
  by frame: they agree to 5230 (bar one-frame blips at 186–187, coins cleared a pass early, and 5053),
  then differ at every later frame by one object step.
- `sf-conform scan` on the case lists only frames 158 and 5517.

## Interface

None new.

## Files

- The engine's pacing or pass-cutting code the fault is in (`sf-core`, `Game::step_paced` and
  `level::beam`), and a test that fails without the fix.
- `engine/crates/sf-conform/tests/enemies_w6_cases.rs`: `enemies-w6-4-complete` added, pinning its exit
  (`level_state` 1 at frame 7307, 0 at 7308) so a script cut short cannot stay green.
- `engine/conform/cases/enemies-w6-4-complete.toml`: its comment updated.
- The W2-2 and W6-2 cases and their tests, if this fixes them.

## Tests

- [ ] `enemies-w6-4-complete` green in `sf-conform -- run` (checks 1–4).
- [ ] A test pinning each W6-4 fault: the coins at 154 and the objects' step at 5231 alone.
- [ ] `w2-2-complete` and `enemies-w6-2-complete` re-run; green ones moved into the test block,
  red ones named in FINDINGS.md with what is left.
- [ ] Every case that was green before stays green (`sf-conform -- all`).

## Done when

- [ ] `enemies-w6-4-complete` is green and in the test block, with no threshold changed.
- [ ] W2-2's and W6-2's faults are fixed by it or named as different, with what was measured.
