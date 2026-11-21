# forest-dumb-title — run stats

Build of `pir/forest-dumb-title` in superfrog-re, 2026-11-20 19:15 → 2026-11-21 07:29 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 12h13m | 4h39m running, 7h33m stopped |
| workers busy | 5h40m | 1.2× parallel, peak 4 of 4 |
| tests | 1h57m |  |
|     agent-driven | 1h23m | 25% of worker time, 125 runs |
|     pir-driven | 33m | 33m end gate |
| waited on you | 47m | 47m with nothing else moving |
| pir's own steps | 1h16m | 52s of it holding the run |
| tokens | 124.3M | 120.6M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T01 | 23m | 10m | 6m | 20.4M | — |
| T02 | 18m | 19m | 16m | 16.1M | — |
| T03 | 18m | 11m | 10m | 6.3M | — |
| T04 | 12m | 6m | 3m | 5.3M | — |
| T05 | 10m | 6m | 4m | 3.3M | — |
| T06 | 7m | 10m | 5m | 4.5M | — |
| T07 | 34m | 18m | 9m | 16.8M | — |
| T08 | 12m | 5m | 3m | 6.9M | — |
| T09 | 14m | 5m | 7m | 9.1M | — |
| T10 | 18m | 4m | 2m | 11.2M | — |
| T11 | 53m | 11m | 14m | 22.1M | 47m |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 12 | 3h45m | 1h30m | 2h14m | 37m | 95.1M |
| reviewers | 11 | 1h49m | 31m | 1h17m | 45m | 27.3M |
| base sync helper | 1 | 50s | 34s | 15s | 0s | 196k |
| test-fix helper | 1 | 4m | 0s | 4m | 24s | 0 |
| coordinator agent | 1 | 1m | 1m | 21s | 0s | 1.6M |
| workers total | 25 | 5h40m | 2h03m | 3h37m | 1h23m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 125 | 1h23m | 40s |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 125 | 1h23m | 40s |

25% of worker active time, and part of it. Untimed runs (started in the background): 0.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 33m | 16m |
| pir-driven total | 2 | 33m | 16m |
| task setup (not a test run) | 12 | 1m | 5s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

1.2× average, peak 4, ceiling 4. Ready tasks waiting for a slot: 17m.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 1h22m | 29% |
| 1 | 2h08m | 46% |
| 2 | 12m | 5% |
| 3 | 38m | 14% |
| 4 | 17m | 6% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 2 | 0s | 0s | 0 |
| reconcile | 2 | 0s | 0s | 0 |
| worktree create | 12 | 5s | 5s | 0 |
| task setup | 12 | 1m | — | 0 |
| waiting for a slot | 8 | 17m | — | 0 |
| report pick-up | 29 | 2s | — | 0 |
| hand-off, wait idle | 23 | 3m | — | 0 |
| merge | 12 | 2s | 2s | 0 |
| conflict fix | 1 | 30s | — | 0 |
| worktree remove | 11 | 43s | 43s | 0 |
| base sync | 1 | 46s | — | 0 |
| end gate | 2 | 33m | — | 0 |
| tests fix | 1 | 18m | — | 0 |
| report | 1 | 18s | — | 0 |
| session start-up | 26 | 9s | — | 0 |
| pir's own total |  | 1h16m | 52s |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 2 | 4h39m | 1 |
| loop pass | 8721 | 2m | 0 |
| waiting on someone | 5 | 48m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 47m | 1 |
| coordinator agent | 40s | 4 |
| nothing else moving, waiting on you | 47m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 2k | 727k | 120.6M | 2.9M |
| total | 2k | 727k | 120.6M | 2.9M |

Limits (machine-wide: includes anything else you ran): 5-hour 39% → ?, weekly 24% → 29%.
