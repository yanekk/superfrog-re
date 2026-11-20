# engine-architecture — run stats

Build of `pir/engine-architecture` in superfrog-re, 2026-10-23 05:48 → 06:48 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 59m | 59m running, 0s stopped |
| workers busy | 2h38m | 2.7× parallel, peak 4 of 4 |
| tests | 11m |  |
|     agent-driven | 10m | 7% of worker time, 68 runs |
|     pir-driven | 45s | 45s end gate |
| waited on you | 0s | 0s with nothing else moving |
| pir's own steps | 3h48m | 21s of it holding the run |
| tokens | 148.5M | 143.4M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T01 | 5m | 1m | 2s | 4.6M | — |
| T02 | 5m | 3m | 5s | 6.6M | — |
| T03 | 7m | 3m | 2s | 11.0M | — |
| T04 | 5m | 3m | 4s | 10.3M | — |
| T05 | 4m | 2m | 52s | 7.6M | — |
| T06 | 4m | 3m | 48s | 7.7M | — |
| T07 | 5m | 2m | 0s | 8.4M | — |
| T08 | 5m | 3m | 41s | 8.0M | — |
| T09 | 6m | 3m | 46s | 9.6M | — |
| T10 | 5m | 2m | 3s | 8.7M | — |
| T11 | 6m | 4m | 43s | 10.1M | — |
| T12 | 3m | 4m | 1m | 5.9M | — |
| T13 | 6m | 6m | 1m | 10.0M | — |
| T14 | 6m | 3m | 57s | 8.2M | — |
| T15 | 4m | 1m | 6s | 5.3M | — |
| T16 | 3m | 3m | 54s | 6.1M | — |
| T17 | 6m | 5m | 1m | 11.1M | — |
| T18 | 6m | 2m | 0s | 7.8M | — |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 18 | 1h38m | 1h33m | 4m | 5s | 97.7M |
| reviewers | 18 | 1h00m | 47m | 13m | 10m | 50.2M |
| coordinator agent | 1 | 36s | 9s | 26s | 0s | 596k |
| workers total | 36 | 2h38m | 2h21m | 17m | 10m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 68 | 10m | 9s |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 68 | 10m | 9s |

7% of worker active time, and part of it. Untimed runs (started in the background): 0.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 1 | 45s | 45s |
| pir-driven total | 1 | 45s | 45s |
| task setup (not a test run) | 18 | 2m | 7s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

2.7× average, peak 4, ceiling 4. Ready tasks waiting for a slot: 3h37m.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 1m | 3% |
| 1 | 22m | 38% |
| 2 | 1m | 3% |
| 3 | 2m | 4% |
| 4 | 31m | 53% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 18 | 6s | 6s | 0 |
| task setup | 18 | 2m | — | 0 |
| waiting for a slot | 17 | 3h37m | — | 0 |
| report pick-up | 39 | 2s | — | 0 |
| hand-off, wait idle | 38 | 5m | — | 0 |
| merge | 20 | 4s | 4s | 0 |
| conflict fix | 2 | 1m | — | 0 |
| worktree remove | 18 | 10s | 10s | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 1 | 45s | — | 0 |
| report | 1 | 24s | — | 0 |
| session start-up | 37 | 36s | — | 0 |
| pir's own total |  | 3h48m | 21s |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 59m | 1 |
| loop pass | 5114 | 1m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 0s | 0 |
| coordinator agent | 0s | 0 |
| nothing else moving, waiting on you | 0s |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5[1m] | 2k | 706k | 143.4M | 4.4M |
| total | 2k | 706k | 143.4M | 4.4M |

Limits (machine-wide: includes anything else you ran): 5-hour 21% → 21%, weekly 12% → 12%.
