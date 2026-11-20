# T06 — superfrog-labels

**Phase:** 2 · **Depends on:** T02, T03 · **Weight:** heavy

## Goal

Label every Superfrog test file and write its crucial areas with the cheapest tests that really prove each. This
is where the budget is won or lost: almost all of the 70 minutes is here.

## Design sections this implements

DESIGN §2.2, §2.4, §2.5.

## Files

- `games/superfrog/smoke.toml` (new)
- `plans/smoke-tests/FINDINGS.md`, for any area needing a test that does not exist

## Interface

The format of DESIGN §2.2. Fast files from `timings.tsv` are `smoke`, the others `full` unless an area needs
them. Slow files enter by named test where possible. Starting areas and candidates, which the worker may replace
with a cheaper test proving the same thing. `complete_w1_1_to_exit_fade` is fixed by the owner.

| Area | Candidates |
|---|---|
| reading the disks | `sf-disk` `real_disks` |
| game data | `sf-assets` `levels` |
| the original boots and repeats | `sf-oracle` `determinism` |
| power-on to the title | `sf-conform` `front::power_on_to_the_title_matches_the_original` |
| entering a level by its code | `sf-conform` `level_entry::a_game_started_at_a_level_plays_as_the_original` |
| the frog in one level | `sf-conform` `frog_cases::frog_run_w1_1` |
| W1-1 can be finished | `sf-conform` `frog_cases::complete_w1_1_to_exit_fade` |
| a sample of enemies | `sf-conform` `enemies_w1` |
| the picture | `sf-conform` `render_vs_oracle` |
| the sound | `sf-conform` `paula_vs_oracle::channel_taps_are_the_references_at_every_clock_through_the_title_music`, `audio_driver` |
| widescreen | `sf-core` `widescreen_camera`, `sf-conform` `widescreen_switch` |
| save states | `sf-core` `savestate` |
| the bug recorder | `sf-conform` `recorder_replay` |
| the route planner | `sf-plan` `walk`, `reach` |
| the app starts | `sf-e2e` `smoke` |
| its menus work | `sf-e2e` `menus` |
| its window opens | `sf-e2e` `window` |

No story or boss area: the owner dropped both (DESIGN §2.5). A test file may still be `smoke` if fast.

## Tests

Labels only; the check is the test.

- [ ] the check reports no problem for Superfrog
- [ ] each candidate chosen has run alone under `pir-test`, and its time is written beside its area as a comment in `smoke.toml`
- [ ] every named slow test really proves its area; the reviewer reads what each asserts, not just its name

## Done when

- [ ] Every area of DESIGN §2.5 for Superfrog has a live smoke test and every timed-fast file is `smoke`.
- [ ] The sum of area and fast-file times is written in PROGRESS as T07's starting estimate.
