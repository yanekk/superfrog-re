# forest-dumb-start — run stats

Build of `pir/forest-dumb-start` in superfrog-re, 2026-10-31 10:13 → 14:58 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 4h44m | 4h44m running, 0s stopped |
| workers busy | 2h18m | 0.5× parallel, peak 3 of 4 |
| tests | 2h57m |  |
|     agent-driven | 29m | 22% of worker time, 65 runs |
|     pir-driven | 2h27m | 2h27m end gate |
| waited on you | 29m | 12m with nothing else moving |
| pir's own steps | 3h20m | 13s of it holding the run |
| tokens | 41.1M | 38.3M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T00 | 12m | 7m | 0s | 5.4M | 16m |
| T01 | 6m | 2m | 1m | 3.5M | 12m |
| T02 | 3m | 2m | 21s | 2.2M | — |
| T03 | 5m | 3m | 14s | 3.3M | — |
| T04 | 5m | 7m | 16s | 3.1M | — |
| T05 | 9m | 5m | 1m | 6.6M | — |
| T06 | 10m | 8m | 4m | 5.6M | — |
| T07 | 6m | 5m | 4m | 4.0M | — |
| T08 | 1m | 4m | 14s | 2.1M | — |
| T09 | 7m | 9m | 6m | 2.6M | — |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 10 | 1h10m | 50m | 19m | 5m | 23.9M |
| reviewers | 10 | 57m | 33m | 24m | 13m | 14.8M |
| base sync helper | 1 | 10m | 50s | 10m | 10m | 835k |
| coordinator agent | 1 | 1m | 46s | 27s | 0s | 1.4M |
| workers total | 21 | 2h18m | 1h24m | 53m | 29m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 1 | 10m | 10m |
| partial | 64 | 19m | 18s |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 65 | 29m | 27s |

22% of worker active time, and part of it. Untimed runs (started in the background): 1.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 2h27m | 1h13m |
| pir-driven total | 2 | 2h27m | 1h13m |
| task setup (not a test run) | 10 | 39s | 3s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.5× average, peak 3, ceiling 4. Ready tasks waiting for a slot: 2s.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 3h18m | 70% |
| 1 | 47m | 17% |
| 2 | 27m | 10% |
| 3 | 12m | 4% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 10 | 2s | 2s | 0 |
| task setup | 10 | 39s | — | 0 |
| waiting for a slot | 8 | 2s | — | 0 |
| report pick-up | 29 | 1s | — | 0 |
| hand-off, wait idle | 20 | 3m | — | 0 |
| merge | 10 | 1s | 1s | 0 |
| worktree remove | 10 | 8s | 8s | 0 |
| base sync | 1 | 48m | — | 0 |
| end gate | 2 | 2h27m | — | 0 |
| report | 1 | 24s | — | 0 |
| session start-up | 22 | 6s | — | 0 |
| pir's own total |  | 3h20m | 13s |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 4h44m | 1 |
| loop pass | 5976 | 1m | 0 |
| waiting on someone | 9 | 29m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 29m | 5 |
| coordinator agent | 19s | 4 |
| nothing else moving, waiting on you | 12m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-haiku-5-5 | 12k | 919 | 0 | 0 |
| claude-opus-5-5 | 816 | 524k | 38.3M | 2.2M |
| total | 13k | 525k | 38.3M | 2.2M |

Limits (machine-wide: includes anything else you ran): 5-hour 4% → 4%, weekly 37% → 37%.
