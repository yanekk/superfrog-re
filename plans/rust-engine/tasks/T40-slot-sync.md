# T40 — slot-sync

**Phase:** 2 · **Depends on:** T15 · **Weight:** light

Added by T19 (owner, 2026-10-13: T19 stopped, its slot machine cut into T40–T43). T19's worker
built and proved all of this once; the code was discarded uncommitted. What it learned is below
so this task is a rebuild, not research.

## Goal

The oracle can be brought to the slot machine's start, and the engine has the slot's data and
variables, so T41–T43 only port code.

## What to build

- **Sync point** `sync = { slot = "<script>", start = "<script>" }` (`sf-conform` `SyncPoint::Slot
  { script, start: Option<String> }`, parsed in `case.rs`, run as `Start::Resume` in `lib.rs`): W1-1
  reached by `start` (default `start-w1-1`; the snapshot key for the default must not change); fail
  if `attract_mode` (h0+$18) is set when W1-1 is reached (a late start can land in the attract
  demo); `<script>` played in W1-1 to its last line's frame; then `level_state` = 1 written (owner,
  2026-10-13: the final-level trick reused for the slot); in `level_tally`'s wait (PC in
  h0+$467e..$46e2) hold up until h0+$4798 = 1, then fire every 4th frame; the sync point is the end
  of the first frame `main_chain` contains h0+$478e. Measured: slot entered 73 frames after the
  forced exit; the original's load ends (chain has h0+$b7e2) on frame 758 from the sync point.
- **The RNG does not move in W1-1** (frog input or not). The title's eye blinks draw from it:
  scripts `start-w1-1-blink1` (menu fire at 4650/4750: one step) and `start-w1-1-late`
  (4720/4820: three steps; 4750+ reaches the attract demo instead). W1-1's clock gives 2 coins.
- `engine/conform/scripts/slot-none.txt` (`0: -`).
- **Assets**: keep `FRUT`, `FRUC`, `FRUM` raw in `GameData` (`gamedata.rs` keep list, as END1/END2),
  `CACHE_FORMAT` +1. FRUT and FRUC are each 320×256×6 ($f000), FRUM 320×256×1 ($2800).
- **LAYOUT** (all h0 unless noted): `slot_copper` h2+$65e4 ($16c), `password_shown` $24cc ($c),
  `slot_state` $49c6 ($16), `slot_count` $515a ($12), `slot_prize` $5526 (W), `slot_win` $57b2
  (L), `slot_hilo` $5988 (6), `slot_hilo_start` $5a04 (W), `slot_spin` $5dc6 ($1c), `slot_flash`
  $603e (W), `slot_cursor` $6064 (L), `slot_menu` $6152 ($e6), `slot_attract` $62ec (8),
  `slot_lamps` $6842 ($d0), `slot_palette` $6912 ($40, the fade target: fades read it from memory,
  so the engine writes it there from hunk 0 at the machine's start), `text_plane` $706e (W).
- **Moira cycles** (test tooling): `sf_call_status.cycles` = Moira's clock after a call
  (`moira_shim.cpp`, `.h`, `ffi.rs`, `CallResult::cycles`), for T41's calibration.
- `re/symbols.toml`: the slot's routines and tables (docs/13 of T43 lists them; T19's list:
  $4e28 free spin, $4eae payout, $50b6 count, $51c4 award, $5528 collect, $5628 jackpot, $57e2
  hi-lo, $598e hi-lo spin, $5a06 spin, $5d48 reel stop, $5de2 hold menu, $5eb4 menu, $6238/$6258
  lamps off/on, $6276 attract, $62f4 digits, $645e lamps, $6558 reels, $661e init, $6f3a print;
  tables $57b6 ladder, $606a pointer stops, $668a reel frames, $670a lamp rects, $3b1ae/$3b2e8
  attract patterns).

## Tests

- [ ] `sync_points` test: `slot-none` reaches the slot (chain has h0+$478e, `slot_state` coins 2).
- [ ] The two start scripts give RNG steps 1 and 3 at W1-1 and no attract demo.
- [ ] Moira cycles: `$6238` costs 1 650 cycles (measured by T19).

## Done when

- [ ] The sync point, scripts, assets, LAYOUT entries and cycle count committed and tested.
