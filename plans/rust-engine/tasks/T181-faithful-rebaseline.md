# T181 — faithful-rebaseline

**Phase:** 2 · **Depends on:** T180, T182; blocks T21 · **Weight:** medium

Added by the owner's amendment of 2026-10-15.

## Goal

Re-judge every existing case under the faithful checks and leave the suite honest: what is green
goes in the test block, what is red has an owner. This takes over what the dropped tasks (T128,
T129, T131, T140–T143, T146, T153, T155, T161, T165, T169, T170, T171) were for: the reds they
chased are expected to turn green under §2.1's tolerances; any that do not are real behaviour
differences and are kept.

## Design sections this implements

DESIGN §2.1, §4.

## Files

- `engine/crates/sf-conform/tests/*.rs`: the test block's case lists.
- `engine/conform/cases/*.toml`: drop `lag-limit`/pin notes that name a dropped task, keep the rest.
- `plans/rust-engine/FINDINGS.md`: rows naming a dropped task are compacted to one row.

## Tests

- [ ] `cargo run -q --release -p sf-conform -- all`: the per-case result table (faithful, and alone
      where §2.1 asks), saved in the commit message.

## Done when

- [ ] Every case judged with check 4 too (T182 makes it the default), and the original's late frames
      per level recorded in docs/08 "Lag" (DESIGN §2.1: slowdown accepted, measured).
- [ ] Every case green under the faithful checks is in the test block (the long ones as ignored
      tests named in the README, as today).
- [ ] Every case still red has a FINDINGS row with its first failing check, frame and cause, and
      an owning task: an open one, or a new task the owner or coordinator approves (CLAUDE.md,
      adding a task in a parallel run).
- [ ] T155's goal 3, which was behaviour, not timing: `w2-1-complete` checked through `run_with` to
      meet what the original meets (the mace from 2029, hearts lost at 2061 and 2365, the spikes from
      2305, the crumbling ledge from 1646 breaking under the frog, the bouncer from 1678, the spud
      stun at 795), each confirmed on the original first (`examples/script_seen`).
- [ ] The `complete-w1-1`, `complete-w1-3`, `complete-w4-1..3`, `w2-1-complete`, `enemies-w6-1-complete`
      and `enemies-w6-3-alt-complete` cases (the dropped tasks' targets) are green faithful and alone,
      or owned as above.
