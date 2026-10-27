# cia-keyboard-pass-counter — Brief

**Brief reviewed:** 2026-10-27 — 5 fixed

## The change

Parts A and B of `plans/amiga-toolkit-prep/DRAFT-3-one-scheduler.md`, as written there (part C is out
of scope). Read that draft, `plans/amiga-toolkit-prep/README.md` and `NORTH-STAR.md` first. A pure
refactor under the faithful bar (rust-engine DESIGN §2.1): zero behaviour change, every check green,
no tolerance touched (`Tolerance::OWNER` fixed). A red check is a bug in the refactor, never a reason
to loosen.

A. One `CiaKeyboard` adapter in sf-core replaces the two `kbd_poll`s (`level/clock.rs::kbd_poll`
over `State::kbd_flag`/`kbd_acked` and `Hw::sdr`; `front/vbl.rs::kbd_poll` over
`front::Keyboard{sdr,new,held}`) and owns the keyboard: change detection (today twice:
`game.rs::step_body`, `Keyboard::feed`), the key-byte encode/decode (today `game.rs::key_byte`,
`game.rs::level_io`'s decode, `Keyboard::from_keys`, `Keyboard::default`; its `!(0x7e << 1 | 1)` is one of measure.sh's 4 encode sites and goes
through the one encoder too), and the poll (every second blank by
`kbd_toggle`, last byte wins, `$ff` once a new byte has been handled). It is the one state carrier,
replacing `State::kbd_flag`/`kbd_acked`, `front::Keyboard` and `Game.sdr`/`Game.keys`. Both handlers
call it; the INTENA/INTREQ `$0008` writes go back to the caller (a small trait or a returned list),
so the level keeps `c.custom` and the front end keeps `wr`, each at the same point in its write order
as today. It still tells the front end whether this blank polled (`polls_keyboard`, used by
`fade_start`), and `front/gameover.rs::level_vbl` uses it instead of its own `$ff` fix-up. The
original's addresses (`kbd_toggle`, `key_down`, `key_up`, h0+$16076..79) stay compared variables
written by the poll.

B. One pass counter and frame boundary: the `vbl_count` ≥ 2 rule (`frame.rs::pass_due` and
`game.rs`'s inline `n_clear(VBL_COUNT - 2)` at the level-loop entry), and `PassClock`, as one piece
used by `level/frame.rs` and `game.rs` (and by sf-conform where it uses `PassClock` today). Each
handler keeps its own ordered step list. `sf-conform/src/sync.rs`'s own `vbl_count` reads of the
oracle's memory (~278 `>= 2`, ~289 `n_clear`) are not part of B and stay as they are.

Risk (measured 2026-10-27): `State` is snapshotted for a late pass (`beam::Pending.start` in
`game.rs` ~597, `Frames::pass_start` / `f.mid` in `level/mod.rs` ~786) and replayed from it, and
today `kbd_flag`/`kbd_acked` ride along inside it. Wherever the one carrier lives, a pass replayed
from a snapshot must see the keyboard as it was at the snapshot.

## Decisions

- 2026-10-27: when one key goes down and another up in the same frame, the up byte is last
  (`game.rs::step_body`'s order: downs ascending, then ups ascending), on every scene. Today the
  front end's `Keyboard::feed` lets the highest changed code win instead. Reason: the level is where
  play and most conformance happen, so its behaviour stays exactly as it is.
- 2026-10-27: level to front end (`Front::after_level`, `level_over`, `enter_slot`) carries the
  one `CiaKeyboard` across as it is, including its handled/new flag, instead of re-deriving it by
  comparing the raw byte with `key_down`/`key_up`. The memory-based derivation stays only where no
  carrier exists: starts from a transplant (`level_io`, the `Front` built from memory at
  front/mod.rs ~817). Reason: one carrier is the draft's point and is how the CIA behaves. If a
  check goes red over this, the builder asks the owner rather than switching back.
- 2026-10-24 (owner, in the draft): no other decisions for A+B.

## Not changing

The story handler (`front/story/vbl.rs`, `intro.rs`'s own `vbl_count` at h0+$1a). The order of game
routines in each handler. Pause and ESC timing. The `Pacing::Late` feed and cut points (part C).
`LAYOUT`, transplant and probe. Save states are build-tagged, so their layout may change.

## Done when

- `measure.sh` reports kbd_poll definitions 1 and key-byte encode sites 1; one pass counter is used by
  the level handler and `Game`.
- The checks under Tests are green. `docs/architecture/README.md` step 3 gets an "A+B done" line.
- `NORTH-STAR.md`'s log gets a column for this step with `sh plans/amiga-toolkit-prep/measure.sh`'s
  values measured on this branch (M1: what the builder ran).

## Tests

- pir's test line, `cd engine && cargo test -q --workspace`, including
  `kbd_poll_matches_the_original` (`sf-conform/tests/level_routines.rs`),
  `the_keyboard_sends_a_byte_per_change_and_reads_ff_once_handled` (moved to the adapter, with a
  case for a key down and another up in one frame: the up byte wins), the four `front/vbl.rs` unit
  tests, `the_pass_clock_counts_passes_ended`, `state_is_matched_by_pass_count` (`faithful.rs`).
- The builder also runs, in `engine/`: `cargo test -p sf-conform --test frog_cases` (`frog_run_w1_1`,
  its 320 late frames asserted by `the_originals_late_frames_are_counted`, `frog_esc_w1_1`, `complete_w4_3`, `coper_tail_outside_a_pass_kept`),
  `--test cases` (`game_over_esc_w1_1`; the draft placed it in frog_cases, it is in `cases.rs`),
  `--test chain` (`p_pauses_and_p_resumes_as_the_original_does`,
  `fire_ends_the_pause_as_the_original_does`), `--test alone`, and
  `the_frog_moves_every_frame_and_objects_every_other` (`sf-oracle/tests/exe_map_claims.rs`).
- New: `kbd_poll_matches_the_original` extended to the front-end path (the poll as the front end
  calls it, its INTENA/INTREQ writes through `wr`), which today only the level copy has.
- The ESC-across-the-boundary cases (`frog_esc_w1_1`, `game_over_esc_w1_1`) cover the carrier
  crossing level to front end, the risky part.

## Changed during the build
