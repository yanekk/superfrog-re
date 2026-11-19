# T58 — mount-character-timing

**Split from:** T56 (owner, 2026-11-19: exact paths, more tasks).

**Phase:** 6 · **Depends on:** T56; blocks T40, T42 · **Weight:** heavy

## Goal

Time level 1's two character routines in the frame mount by their path, as T56 times the rest: `char_hit_check` (`h0+$3b2a`, with `h0_69d6`, `h0_3dd8`, `h0_2a6e`, `h0_3adc`; 183 instructions) and `char_update` (`h0+$3e0c`, with `char_probe` `h0+$4f84`/`h0+$5500`, `char_probe_flags` `h0_4f90`, `h0_3e30` and the rest of its tree; 729 instructions). Each gets a rule in `fd-translated/src/seam/rules.rs` giving the [`Decision`]s of the original's path from the level it finds, walked through the generated shapes (`seam::walk`), in place of its `COSTS` lump; CLXDAT's read and each sound's writes and line waits are then made by the original's instructions in the walk.

Owner's rules (2026-11-19, carried from T56): each rule is general, derived from the state the routine reads, with no per-case or per-address table beyond the routine itself; each is held to the translated routine's counted cycles on the routine judge's states; no translated frame runs beside the hand engine; keep the whole-frame mount, loosen no comparison.

Why (T56, 2026-11-19): a routine's time on the machine hangs on each instruction's place on the bus, not on its total, so only the path walked instruction by instruction matches the translated program; measured, exact totals per routine passed 2 of the 8 cases T56 carried. `char_update`'s time runs from 4 374 to 17 160 cycles and sets where the pass ends, and so `main_loop`'s redraw cuts.

## Design sections this implements

DESIGN §2.9 (the seam; the mount's time).

## Files

- `crates/fd-translated/src/seam/rules.rs`: the two rules; `RULED` gains `0x3b2a` and `0x3e0c`.
- `crates/fd-translated/src/seam/frame.rs`: `COSTS` loses the two rows (and goes, if nothing is left in it).
- `crates/fd-conform/tests/mount_timing.rs`: both in `CHECKED` with their generators.
- `crates/fd-conform/tests/mount_level_1.rs`: the cases T56 leaves failing move to `PASS_MOUNTED`; `CLX_LINES_OFF` to 0.

## Interface

```
cargo test --release -p fd-conform --test mount_timing -- --ignored     # each rule against the translated routine
cargo test --release -p fd-conform --test mount_level_1 -- --ignored
cargo run --release -p fd-conform --example game_frame_costs -- [--passes [--mounted]] CASE ..
```

`rules::path(offset, l)` answers `0x3b2a` and `0x3e0c`; `frame.rs` walks them as every ruled routine.

## Tests

- [ ] Full: `mount_timing.rs` passes with both rules, 2 000 states each.
- [ ] Full: every case T56 leaves in its `T56` list that passes translated alone passes mounted.
- [ ] Full: `the_mounts_clxdat_read_lands_on_the_translated_programs_line` with `CLX_LINES_OFF` 0.
- [ ] Full: `fd-judge --seam 'h0+$1fec'` still passes 2 000 of 2 000.

## Done when

- [ ] The tests hold; DESIGN §2.9 names the two rules, dated.
- [ ] FINDINGS has which cases remain, each with its reason.
