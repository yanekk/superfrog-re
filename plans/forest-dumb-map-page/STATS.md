# forest-dumb-map-page — run stats

Build of `pir/forest-dumb-map-page` in superfrog-re, 2026-11-20 15:14 → 20:19 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 5h05m | 5h05m running, 0s stopped |
| workers busy | 4h39m | 0.9× parallel, peak 4 of 4 |
| tests | 1h52m |  |
|     agent-driven | 1h14m | 26% of worker time, 180 runs |
|     pir-driven | 38m | 38m end gate |
| waited on you | 47m | 33m with nothing else moving |
| pir's own steps | 1h16m | 1m of it holding the run |
| tokens | 104.3M | 100.8M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T00 | 5m | 3m | 0s | 3.1M | — |
| T01 | 4m | 3m | 1m | 4.8M | — |
| T02 | 8m | 4m | 1m | 7.7M | — |
| T03 | 5m | 8m | 5m | 2.8M | — |
| T04 | 6m | 3m | 59s | 4.8M | — |
| T05 | 9m | 4m | 3m | 5.7M | 8m |
| T06 | 10m | 6m | 4m | 8.1M | — |
| T07 | 15m | 5m | 3m | 15.6M | 8m |
| T08 | 16m | 7m | 11m | 6.3M | — |
| T09 | 7m | 6m | 2m | 6.2M | — |
| T10 | 14m | 12m | 6m | 7.5M | — |
| T11 | 6m | 2m | 51s | 6.3M | 29m |
| T12 | 1h26m | 8m | 32m | 23.9M | 43s |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 13 | 3h18m | 1h27m | 1h51m | 49m | 77.7M |
| reviewers | 13 | 1h16m | 34m | 42m | 24m | 25.8M |
| test-fix helper | 1 | 4m | 0s | 4m | 31s | 0 |
| coordinator agent | 1 | 1m | 40s | 22s | 0s | 761k |
| workers total | 27 | 4h39m | 2h01m | 2h37m | 1h14m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 180 | 1h14m | 24s |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 180 | 1h14m | 24s |

26% of worker active time, and part of it. Untimed runs (started in the background): 3.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 38m | 19m |
| pir-driven total | 2 | 38m | 19m |
| task setup (not a test run) | 13 | 1m | 5s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.9× average, peak 4, ceiling 4. Ready tasks waiting for a slot: 8m.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 2h06m | 42% |
| 1 | 1h56m | 38% |
| 2 | 38m | 13% |
| 3 | 6m | 2% |
| 4 | 16m | 6% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 13 | 11s | 11s | 0 |
| task setup | 13 | 1m | — | 0 |
| waiting for a slot | 9 | 8m | — | 0 |
| report pick-up | 29 | 2s | — | 0 |
| hand-off, wait idle | 26 | 4m | — | 0 |
| merge | 13 | 4s | 4s | 0 |
| worktree remove | 13 | 44s | 44s | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 2 | 38m | — | 0 |
| tests fix | 1 | 23m | — | 0 |
| report | 1 | 21s | — | 0 |
| session start-up | 28 | 13s | — | 0 |
| pir's own total |  | 1h16m | 1m |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 5h05m | 1 |
| loop pass | 8306 | 3m | 0 |
| waiting on someone | 8 | 47m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 47m | 6 |
| coordinator agent | 13s | 2 |
| nothing else moving, waiting on you | 33m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 2k | 694k | 100.8M | 2.8M |
| total | 2k | 694k | 100.8M | 2.8M |

Limits (machine-wide: includes anything else you ran): 5-hour 61% → 61%, weekly 20% → 20%.
