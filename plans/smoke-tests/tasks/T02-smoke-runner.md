# T02 — smoke-runner

**Phase:** 1 · **Depends on:** T01 · **Weight:** medium

## Goal

The smoke command: it gathers the projects, runs the check, then runs each entry under the clock, reports
times, warns at 15 minutes, fails at 20 and keeps the history. It is what pir calls at the end of a build.

## Design sections this implements

DESIGN §2.1, §2.6, §2.8, §3.1, §3.2, §5 (Quiet).

## Files

- `tools/smoke.sh`, `tools/smoke.py` (new)
- `tools/test_smoke_shell.py` (new)

## Interface

```
tools/smoke.sh [--check-only] [--list]
  exit 0: all passed; non-zero: a problem, a failed entry, or the hard limit
  --check-only  runs the check alone, in seconds
  --list        prints the entries, one per line, and runs nothing
env: SMOKE_VERBOSE=1 (drops -q), SMOKE_TIMES_FILE (history path)
```

```python
# tools/smoke.py
WARN_SECONDS = 15 * 60
LIMIT_SECONDS = 20 * 60
def run_entries(entries, *, warn_s, limit_s, now, spawn) -> Result   # limits and clock as parameters, for the tests
```

Output on green: one line per project, then `smoke: ok in N s` and the five slowest entries. Past the warn
limit: a line starting `smoke: WARNING` naming both numbers. At the hard limit: `smoke: FAILED, over the limit`
and every entry that did not run. History row: date, commit, seconds, `ok|failed|limit`, tab separated.

Each entry runs in its own process group; the runner kills the group at the limit and on its own exit. Root
checks run first: `unittest discover -q -s tools -p 'test_*.py'` and `tools/check_tier_names.py`. A game folder
is found the way `tools/test.sh` finds one, so a new game cannot be missed.

## Tests

Using fake entries and limits of a fraction of a second, never real waits of minutes.

- [ ] all pass: exit 0, summary printed, one history row with `ok`
- [ ] a check problem: exit non-zero, no entry runs
- [ ] one entry fails: the rest still run, exit non-zero
- [ ] past the warn limit: `smoke: WARNING` printed, exit 0
- [ ] past the hard limit: the running entry and a child it spawned are dead, the unrun entries named, exit non-zero, row `limit`
- [ ] an unwritable history path prints a line and does not change the exit status
- [ ] the run leaves the worktree clean
- [ ] `--list` runs nothing; `--check-only` runs no entry

## Done when

- [ ] `tools/test_smoke_shell.py` passes, and `tools/smoke.sh --check-only` reports the current lack of labels by name.
- [ ] Limits are only in the two constants and the history never lands inside the repo.
