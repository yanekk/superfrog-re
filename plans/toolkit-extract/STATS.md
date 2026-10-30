# toolkit-extract — run stats

Build of `pir/toolkit-extract` in superfrog-re, 2026-10-29 08:00 → 2026-10-30 12:32 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 28h31m | 28h31m running, 0s stopped |
| workers busy | 7h15m | 0.3× parallel, peak 3 of 4 |
| tests | 5h11m |  |
|     agent-driven | 2h36m | 36% of worker time, 146 runs |
|     pir-driven | 2h35m | 2h35m end gate |
| waited on you | 8h45m | 8h45m with nothing else moving |
| pir's own steps | 7h25m | 1m of it holding the run |
| tokens | 155.9M | 151.5M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T01 | 37m | 14m | 12m | 21.4M | — |
| T02 | 20m | 13m | 21m | 6.6M | — |
| T03 | 20m | 14m | 16m | 9.9M | — |
| T04 | 12m | 5m | 3m | 8.8M | — |
| T05 | 28m | 10m | 3m | 22.7M | — |
| T06 | 24m | 20m | 1m | 16.2M | — |
| T07 | 20m | 13m | 17m | 15.2M | — |
| T08 | 27m | 14m | 15m | 9.1M | — |
| T09 | 34m | 7m | 4m | 19.1M | — |
| T10 | 22m | 25m | 31m | 7.6M | — |
| T11 | 14m | 3m | 2m | 17.7M | 8h45m |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 11 | 4h22m | 1h44m | 2h37m | 55m | 110.7M |
| reviewers | 11 | 2h23m | 39m | 1h44m | 1h14m | 44.1M |
| test-fix helper | 1 | 28m | 1m | 27m | 25m | 733k |
| coordinator agent | 1 | 52s | 24s | 28s | 0s | 313k |
| workers total | 23 | 7h15m | 2h25m | 4h49m | 2h36m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 146 | 2h36m | 1m |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 146 | 2h36m | 1m |

36% of worker active time, and part of it. Untimed runs (started in the background): 14.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 2h35m | 1h17m |
| pir-driven total | 2 | 2h35m | 1h17m |
| task setup (not a test run) | 11 | 53s | 4s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.3× average, peak 3, ceiling 4. Ready tasks waiting for a slot: 4s.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 23h32m | 82% |
| 1 | 3h25m | 12% |
| 2 | 52m | 3% |
| 3 | 41m | 2% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 11 | 3s | 3s | 0 |
| task setup | 11 | 53s | — | 0 |
| waiting for a slot | 10 | 4s | — | 0 |
| report pick-up | 23 | 0s | — | 0 |
| hand-off, wait idle | 22 | 3m | — | 0 |
| merge | 11 | 2s | 2s | 0 |
| worktree remove | 11 | 55s | 55s | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 2 | 2h35m | — | 0 |
| tests fix | 1 | 4h43m | — | 0 |
| report | 1 | 21s | — | 0 |
| session start-up | 24 | 7s | — | 0 |
| pir's own total |  | 7h25m | 1m |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 28h31m | 1 |
| loop pass | 25221 | 11m | 0 |
| waiting on someone | 1 | 8h45m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 8h45m | 1 |
| coordinator agent | 0s | 0 |
| nothing else moving, waiting on you | 8h45m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 2k | 715k | 151.5M | 3.6M |
| total | 2k | 715k | 151.5M | 3.6M |

Limits (machine-wide: includes anything else you ran): 5-hour 39% → 39%, weekly 3% → 3%.
