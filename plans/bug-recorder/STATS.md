# bug-recorder — run stats

Build of `pir/bug-recorder` in superfrog-re, 2026-10-20 10:16 → 2026-10-21 07:41 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 21h24m | 21h24m running, 0s stopped |
| workers busy | 12h00m | 0.6× parallel, peak 4 of 4 |
| tests | 1h19m |  |
|     agent-driven | 1h02m | 9% of worker time, 50 runs |
|     pir-driven | 16m | 16m end gate |
| waited on you | 10h15m | 9h31m with nothing else moving |
| pir's own steps | 3h01m | 1m of it holding the run |
| tokens | 242.4M | 234.8M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T00 | 49m | 5m | 0s | 21.2M | 9m |
| T01 | 8m | 6m | 1m | 6.2M | — |
| T02 | 15m | 4m | 1m | 13.2M | — |
| T03 | 18m | 9m | 5m | 8.8M | — |
| T04 | 8m | 8m | 0s | 7.6M | — |
| T05 | 29m | 16m | 11m | 17.0M | — |
| T06 | 31m | 13m | 0s | 20.4M | — |
| T07 | 13m | 13m | 6m | 8.0M | — |
| T08 | 37m | 7m | 19s | 17.2M | 21m |
| T09 | 36m | 21m | 0s | 22.7M | 21m |
| T10 | 18m | 6m | 0s | 16.6M | 22m |
| T11 | 2h07m | 19m | 6m | 30.3M | 1h05m |
| T12 | 21m | 5m | 0s | 8.1M | 22m |
| T13 | 50m | 31m | 2m | 14.6M | 7h32m |
| T14 | 55m | 15m | 26m | 16.9M | — |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 15 | 8h41m | 2h49m | 5h52m | 32m | 177.7M |
| reviewers | 15 | 3h05m | 50m | 2h15m | 29m | 51.7M |
| base sync helper | 1 | 13m | 1m | 11m | 54s | 809k |
| coordinator agent | 1 | 7m | 7m | 38s | 0s | 12.2M |
| workers total | 31 | 12h00m | 3h41m | 8h18m | 1h02m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 50 | 1h02m | 1m |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 50 | 1h02m | 1m |

9% of worker active time, and part of it. Untimed runs (started in the background): 0.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 16m | 8m |
| pir-driven total | 2 | 16m | 8m |
| task setup (not a test run) | 15 | 1m | 6s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.6× average, peak 4, ceiling 4. Ready tasks waiting for a slot: 27m.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 12h27m | 58% |
| 1 | 7h21m | 34% |
| 2 | 36m | 3% |
| 3 | 31m | 2% |
| 4 | 27m | 2% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 15 | 4s | 4s | 0 |
| task setup | 15 | 1m | — | 0 |
| waiting for a slot | 14 | 27m | — | 0 |
| report pick-up | 50 | 1s | — | 0 |
| hand-off, wait idle | 32 | 5m | — | 0 |
| merge | 17 | 3s | 3s | 0 |
| conflict fix | 2 | 1h56m | — | 0 |
| worktree remove | 15 | 52s | 52s | 0 |
| base sync | 1 | 13m | — | 0 |
| end gate | 2 | 16m | — | 0 |
| report | 1 | 30s | — | 0 |
| session start-up | 32 | 34s | — | 0 |
| pir's own total |  | 3h01m | 1m |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 21h24m | 1 |
| loop pass | 23381 | 12m | 1 |
| waiting on someone | 27 | 10h24m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 10h15m | 10 |
| coordinator agent | 8m | 17 |
| nothing else moving, waiting on you | 9h31m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5[1m] | 3k | 1.1M | 234.8M | 6.4M |
| total | 3k | 1.1M | 234.8M | 6.4M |

Limits (machine-wide: includes anything else you ran): 5-hour 2% → 2%, weekly 80% → 80%.
