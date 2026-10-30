# monorepo-and-knowledge — run stats

Build of `pir/monorepo-and-knowledge` in superfrog-re, 2026-10-30 17:52 → 20:25 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 2h33m | 2h33m running, 0s stopped |
| workers busy | 50m | 0.3× parallel, peak 2 of 4 |
| tests | 1h23m |  |
|     agent-driven | 8m | 18% of worker time, 31 runs |
|     pir-driven | 1h14m | 1h14m end gate |
| waited on you | 17m | 15m with nothing else moving |
| pir's own steps | 1h16m | 15s of it holding the run |
| tokens | 17.9M | 16.4M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T01 | 7m | 3m | 1m | 2.6M | — |
| T02 | 13m | 7m | 5m | 7.7M | — |
| T03 | 3m | 2m | 1m | 1.5M | — |
| T04 | 1m | 2m | 20s | 1.6M | 17m |
| T05 | 6m | 3m | 16s | 3.3M | — |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 5 | 32m | 27m | 5m | 4m | 10.8M |
| reviewers | 5 | 18m | 14m | 4m | 4m | 6.1M |
| coordinator agent | 1 | 1m | 59s | 21s | 0s | 1.0M |
| workers total | 10 | 50m | 41m | 9m | 8m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 30 | 8m | 17s |
| full, ran without pir-test | 1 | 3s | 3s |
| agent-driven total | 31 | 8m | 17s |

18% of worker active time, and part of it. Untimed runs (started in the background): 0.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 1 | 1h14m | 1h14m |
| pir-driven total | 1 | 1h14m | 1h14m |
| task setup (not a test run) | 5 | 20s | 4s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.3× average, peak 2, ceiling 4. Ready tasks waiting for a slot: 1s.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 1h49m | 72% |
| 1 | 36m | 24% |
| 2 | 7m | 5% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 5 | 1s | 1s | 0 |
| task setup | 5 | 20s | — | 0 |
| waiting for a slot | 4 | 1s | — | 0 |
| report pick-up | 13 | 0s | — | 0 |
| hand-off, wait idle | 10 | 1m | — | 0 |
| merge | 5 | 1s | 1s | 0 |
| worktree remove | 5 | 13s | 13s | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 1 | 1h14m | — | 0 |
| report | 1 | 19s | — | 0 |
| session start-up | 11 | 3s | — | 0 |
| pir's own total |  | 1h16m | 15s |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 2h33m | 1 |
| loop pass | 2975 | 32s | 0 |
| waiting on someone | 4 | 18m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 17m | 1 |
| coordinator agent | 44s | 3 |
| nothing else moving, waiting on you | 15m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 308 | 274k | 16.4M | 1.2M |
| total | 308 | 274k | 16.4M | 1.2M |

Limits (machine-wide: includes anything else you ran): 5-hour 35% → 35%, weekly 33% → 33%.
