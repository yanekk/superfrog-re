# T59 — mount-walk-close

**Split from:** T56 (pir's context warning, 2026-11-19). Carries T56's Tests 2 to 4 and both "Done when" items.

**Phase:** 6 · **Depends on:** T56; blocks T40, T42 · **Weight:** medium

## Goal

Close T56's walk: the code is committed (`pcode_rt::shape`, `seam::walk`, `seam::rules` with 18 rules, `mount_timing.rs` passing), the lists and docs are not.

Measured by T56 before the split (2026-11-19, `fd-conform run-all --engine translated --mount level-1`, about 11 min): **47 of 50 pass mounted**. Fail: `route-W1-1` (pass 4161, `edge_at_down_x` 580 against 584), `route-both-W1-1` (pass 4969, `creature_slots+$14`), both passing translated alone end to end (T58's: `char_hit_check` and `char_update` are still lumps); `power-on-level-1` (pass 1045, `h0+$1fe6`), which fails translated alone too (T53).

## Files

- `crates/fd-conform/tests/mount_level_1.rs`: move `camera-bottom-right`, `contact-route`, `death-spikes`, `death-water`, `enemy-contact`, `enemy-kind4`, `mechanic-spring`, `mechanic-vanish` from `T56` to `PASS_MOUNTED`; rename the list of the three left (`T58`, its doc saying why each fails); T57's `HEARS_T56` and its test follow (cases that now pass move to `HEARS`, check `the_effects_each_case_hears_…` with them); module doc names the walk.
- `crates/fd-translated/src/seam/frame.rs`: `COSTS` down to `0x3b2a` and `0x3e0c` (every other row is dead since T56), `FADE_ON`/`FADE_FLAG`, `per_blit` and the unwalked `blit` path removed (a blit outside a walked routine is a stop), module doc's time paragraph rewritten for the walk.
- `crates/fd-conform/tests/mount_level_1.rs` `CLX_LINES_OFF`: measure; 0 if it holds, else say why (CLXDAT's read is in `char_hit_check`, still a lump until T58).
- DESIGN §2.9: one paragraph after T55's, "The mount's path (T56, owner: exact paths)": the measurement that chose it, the shapes, the walk, the rules, the check, 18 walked, two lumps until T58.
- FINDINGS: the three cases left, each with its reason.

## Interface

```
cargo test --release -p fd-conform --test mount_level_1 -- --ignored
cargo test --release -p fd-conform --test seam -- --ignored      # fd-judge --seam 'h0+$1fec' 2 000 of 2 000
cargo test --release -p fd-conform --test mount_timing -- --ignored
```

## Tests

- [ ] Full: `every_case_that_passes_mounted_still_does` with the eight moved.
- [ ] Full: `the_mounts_clxdat_read_lands_on_the_translated_programs_line` (with `CLX_LINES_OFF` as measured).
- [ ] Full: `seam.rs` frame mount 2 000 of 2 000; `mount_timing.rs` passes.

## Done when

- [ ] The tests hold; DESIGN §2.9 states the walk, measured, dated.
- [ ] FINDINGS has which cases remain, each with its reason.
