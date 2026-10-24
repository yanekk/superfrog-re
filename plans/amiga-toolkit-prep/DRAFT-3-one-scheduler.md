# Draft 3 — one frame scheduler, one keyboard

**Status:** draft, 2026-10-23; owner decisions recorded 2026-10-24. Not reviewed. Parts A+B are input for `@repo/single`; part C is a
later `pir plan`, after draft 2.
Source: `docs/architecture/frame-interrupts-input.md` §3–§7; native order step 3.

## The change

Handlers: level `level/frame.rs` (`vbl`, `copper_irq`, `main_program` with 21 inline `spend`
charges), front end `front/vbl.rs` (a measured `cost` table placing writes after the fact), story
`front/story/vbl.rs` (instruction-timed on its own bus simulator), game over borrowing the level's
`vbl`. Boundary plumbing in `level/mod.rs` (`frame_end`, `irq_frame_end`, `vbl_held`).

Keyboard: two `kbd_poll`s with different state — `level/clock.rs` (`State::kbd_flag`/`kbd_acked`,
`hw.sdr`, writes via `c.custom`) and `front/vbl.rs` (`Keyboard{sdr,new,held}`, writes via `wr`) —
sharing `kbd_toggle`/`key_down`/`key_up` at h0+$16076..79. Key-change detection twice
(`game.rs::step_body`, `Keyboard::feed`); key-byte encode/decode three times (`game.rs::key_byte`/
`level_io`, `Keyboard::from_keys`, `Keyboard::default`).

- **A. One `CiaKeyboard` adapter** (single run): last byte wins, poll every second blank, `$ff`
  once handled; owns change detection, encode/decode and the poll; one state carrier replacing
  `kbd_flag`/`kbd_acked`, `Keyboard` and `Game.sdr`. Both handlers call it; the interrupt-register
  writes go back to the caller (a small trait or returned list), so the level keeps `custom` and
  the front end keeps `wr`. It still reports "polled this blank" for the front end's `fade_start`.
  The original's addresses (`kbd_toggle` etc.) stay compared variables.
- **B. One pass counter and frame boundary** (same run): the `vbl_count` ≥ 2 rule, `pass_due`,
  `PassClock` as one piece shared by `frame.rs` and `game.rs`. Each handler keeps its own ordered
  step list.
- **C. Later, after draft 2:** a "where in the frame" value on draft 2's `BeamPos` replacing
  `Cut`'s cycle-charged fields, and `Frames`/`Pending` folded into a generic late-pass runner.
  1 000+ lines across `beam.rs`, `level/mod.rs`, `frame.rs`, `sync.rs`, `feed.rs`; 2 tasks.

## Decisions

None for A+B: nothing a player feels changes if the checks stay green. One for later: whether C is
done at all; the north star decides (if the scheduler is the last piece keeping the toolkit from
being ready). An "update all, then draw" frame is not an option: it breaks the 50/25 Hz split
rust-engine DESIGN §2.1 keeps.

## Not changing

The story handler (its timing stays separate, story-and-ending chapter). The order of game routines
in each handler. Pause and ESC timing. The `Pacing::Late` feed and cut points (until C).

## Done when (A+B)

- One `fn kbd_poll` (north-star M2: today 2); one key-byte encoder (today 4 code lines).
- One pass counter used by the level handler and `Game`.
- Checks below green. `docs/architecture/README.md` step 3 gets an "A+B done" line.

## Tests

- pir's test line runs `cd engine && cargo test -q --workspace` (owner, 2026-10-24)
  (`kbd_poll_matches_the_original` in `level_routines.rs`,
  `the_keyboard_sends_a_byte_per_change_and_reads_ff_once_handled`, the four `front/vbl.rs` unit
  tests, `the_pass_clock_counts_passes_ended`, `state_is_matched_by_pass_count`);
  the builder also runs `cargo test -p sf-conform --test frog_cases` (`frog_run_w1_1` with 320 late frames, `frog_esc_w1_1`,
  `game_over_esc_w1_1`, `complete_w4_3`, `coper_tail_outside_a_pass_kept`), `--test chain`
  (`p_pauses_and_p_resumes_as_the_original_does`, `fire_ends_the_pause_as_the_original_does`),
  `--test alone`, and `the_frog_moves_every_frame_and_objects_every_other` (`exe_map_claims.rs`).
- New: `kbd_poll_matches_the_original` extended to the front-end path, which today only the level
  copy has.

## Risks

- Keyboard state lives in compared memory and must carry from level to front end
  (`Front::after_level`, `level_io` transplant). Merging the two state carriers is the risky part;
  the ESC-across-the-boundary cases cover it.

## Toolkit payoff

Generic: `CiaKeyboard`, `input.rs` (`Joy`, `KeySet`, `FrameInput`, the token format), `Hw`/`joy1dat`,
the pass rule and frame boundary; after C, the late-pass replay machinery. Superfrog-only: what runs
at 50 Hz and its order, the pass's 13 phases, `level_clock_tick`, the front end's `cost` table, the
story handler. Moves north-star M2 (keyboard row), M5.
