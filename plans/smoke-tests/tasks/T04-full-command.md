# T04 — full-command

**Phase:** 1 · **Depends on:** — · **Weight:** light

## Goal

The command the owner's nightly schedule will call: every test in every project, the `#[ignore]`d ones
included except the 32 screen tests (DESIGN §2.1). It is never run for real in this plan.

## Design sections this implements

DESIGN §2.1, §4.

## Files

- `tools/full.sh` (new)
- `toolkit/test.sh`, `games/superfrog/test.sh`, `games/forest-dumb/test.sh`
- `tools/test_full_command.py` (new)

## Interface

```sh
# tools/full.sh
PIR_TEST_SCOPE=all PIR_TEST_INCLUDE_IGNORED=1 exec "$(dirname "$0")/test.sh"
# each project's test.sh: appends `-- --include-ignored` to its cargo line when the variable is 1
# games/superfrog/test.sh also adds a `--skip` per test whose #[ignore] reason names SF_E2E_OS=1, unless that is set
```

`tools/test.sh` without the variable behaves exactly as today, so existing callers do not change.

## Tests

With a fake `cargo` first on the PATH that records its arguments and exits 0, and never the real suites. Run in
a scratch repo with the real `test.sh` files and a stub `venv/bin/python`, as `tools/test_selector_shell.py`
does: in this repo `tools/test.sh`'s root checks would discover this test file and recurse without end.

- [ ] `tools/full.sh` gives each of the three projects' cargo calls `--include-ignored`
- [ ] `tools/test.sh` with `PIR_TEST_SCOPE=all` and no new variable gives none
- [ ] Superfrog's call skips a screen test found from its `#[ignore]` reason, and not an ordinary ignored test; with `SF_E2E_OS=1` it skips nothing
- [ ] the skip list found from the real source has 32 names
- [ ] a failing fake cargo in one project makes `tools/full.sh` exit non-zero while the others still run

## Done when

- [ ] `tools/test_full_command.py` passes without the real suites running.
- [ ] `tools/full.sh` is executable, and PROGRESS says it is proven on fakes only, unrun for real.
