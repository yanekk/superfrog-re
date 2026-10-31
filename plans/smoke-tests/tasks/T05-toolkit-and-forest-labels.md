# T05 — toolkit-and-forest-labels

**Phase:** 2 · **Depends on:** T02, T03 · **Weight:** medium

## Goal

Label every toolkit and Forest Dumb test file and write their crucial areas, so those two projects pass the
check. Kept apart from Superfrog because they are small and carry none of its budget questions.

## Design sections this implements

DESIGN §2.2, §2.4, §2.5.

## Files

- `toolkit/smoke.toml` (new; no game name, as the name check requires)
- `games/forest-dumb/smoke.toml` (new)

## Interface

The format of DESIGN §2.2. Label from `plans/smoke-tests/timings.tsv`: 10 s or less is `smoke`; slower is `full`
unless an area needs it. Starting areas, which the worker may refine but not drop:

| Area | Candidates |
|---|---|
| toolkit: disk reading | `amiga-disk` `ofs`, `readers` |
| toolkit: sound | `amiga-paula` `unit` |
| toolkit: the original on the emulator | `amiga-oracle` `boot`, `routine` |
| toolkit: comparison judges | `amiga-conform` `faithful` |
| toolkit: recording | `amiga-record` `round_trip` |
| toolkit: the other pieces | `amiga-chips`, `amiga-clock`, `amiga-input`, `amiga-app`, `amiga-e2e`: each `src` (DESIGN §2.5 says each piece) |
| toolkit: rule checks | `toolkit-checks` `purity`, `dry-fit` `deps`, `dry_fit` |
| Forest Dumb: reading its disks | `fd-tools` `real_disks` |
| Forest Dumb: first comparison | `fd-conform` `first_case`, by name `the_idle_title_matches_the_original_on_every_pass` if the whole file is not needed |

## Tests

Labels only; the check is the test.

- [ ] the check reports no problem for these two projects
- [ ] `venv/bin/python tools/check_tier_names.py` passes
- [ ] each slow file in smoke is justified by an area that covers it

## Done when

- [ ] Each entry from these two projects has run alone under `pir-test` and is green, or its failure is in FINDINGS.
- [ ] Every timed-fast file of these projects is `smoke`.
