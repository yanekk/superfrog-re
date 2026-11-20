# Readiness call — 2026-10-28

Stage 3 of the readiness plan (`README.md`), on `main` at `9ecba76` with amiga-chips, the
cia-keyboard-pass-counter single run and one-clock merged. The NORTH-STAR columns of the last two
were each measured before the other merged in; these are `main`'s own values, from
`sh plans/amiga-toolkit-prep/measure.sh` on an export of `main`.

| Condition (NORTH-STAR "Ready") | On `main` | Met |
|---|---|---|
| 1. M1 green | one-clock: `sf-conform all` 216 cases green and sf-e2e `whole_game.rs` green, both before the keyboard run merged in; the test block green after the post-sync fix (that fix not freshly reviewed); `channels`' whole-game test fails only its pre-existing `T216` label assert, verdict byte-identical to base (one-clock REPORT) | mostly: no full `sf-conform all` on the combined state |
| 2. M2 toolkit rows at target | kbd_poll 1, key-byte encode 1, blitter memory effect 1, BLTSIZE decodes 3, chip-memory containers 2, `is_paula` 1, frame/line constants 2, beam-position hits 6 all `BeamPos` clients (one-clock column); hunk-0 loads 4 (wanted, not required) | yes |
| 3. M4 ≥ 3 | `PASS` sf-paula (838), amiga-chips (763), amiga-clock (824): M5 2 425 lines | yes |
| 4. J1 | chosen (Forest Dumb Forever); disks at hand, a cracked set; whether a cracked set is acceptable as the reference is decided in draft 6's game-two task (owner, 2026-10-27) | yes, with that decision deferred |
| 5. J2 | owner 2026-10-27: extract after the keyboard and the clock, now both merged | extract now |
| 6. Dry fit | by draft 5's plan it is phase A's last task, on the toolkit as extracted | at the end of draft 5 phase A |

**Verdict:** ready to extract (draft 5 phase A, then draft 6). Carried into draft 5 as known items:

- `amiga-clock` still assumes Superfrog's 313-line frame in its end-of-frame wait and the blitter
  timing's wrap (one-clock REPORT); parameterise before game two.
- The test block took 1 h 45 min of a 120-minute budget (one-clock REPORT); draft 6's per-game
  test selector and the `faster-tests` plan bear on it.
- `channels`' `T216` label assert awaits the owner's confirmation of the accepted whole-game sound
  result; separate from the toolkit work.
- A full `sf-conform all` on the combined `main` has not run since the keyboard run merged in;
  draft 5's T1 runs it first, as M1's new baseline.
