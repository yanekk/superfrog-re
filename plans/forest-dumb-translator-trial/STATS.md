# forest-dumb-translator-trial — run stats

Build of `pir/forest-dumb-translator-trial` in superfrog-re, 2026-11-07 16:01 → 17:44 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 1h43m | 1h43m running, 0s stopped |
| workers busy | 1h59m | 1.2× parallel, peak 4 of 4 |
| tests | 37m |  |
|     agent-driven | 9m | 8% of worker time, 79 runs |
|     pir-driven | 28m | 28m end gate |
| waited on you | 37s | 37s with nothing else moving |
| pir's own steps | 46m | 12s of it holding the run |
| tokens | 51.3M | 49.1M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T00 | 1m | 1m | 27s | 1.5M | — |
| T01 | 14m | 6m | 2m | 8.0M | — |
| T02 | 5m | 2m | 38s | 2.9M | — |
| T03 | 15m | 9m | 1m | 8.7M | — |
| T04 | 5m | 3m | 1m | 4.7M | — |
| T05 | 10m | 5m | 1m | 4.9M | — |
| T06 | 7m | 3m | 1m | 8.2M | — |
| T07 | 11m | 3m | 24s | 7.2M | — |
| T08 | 3m | 5m | 0s | 2.4M | 37s |
| tests-fix | 0s | 0s | 0s | 0 | — |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 9 | 1h15m | 53m | 21m | 5m | 30.8M |
| reviewers | 9 | 41m | 28m | 13m | 3m | 18.0M |
| test-fix helper | 1 | 2m | 2m | 32s | 12s | 1.3M |
| coordinator agent | 1 | 1m | 58s | 21s | 0s | 1.0M |
| workers total | 19 | 1h59m | 1h24m | 35m | 9m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 79 | 9m | 6s |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 79 | 9m | 6s |

8% of worker active time, and part of it. Untimed runs (started in the background): 0.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 28m | 14m |
| pir-driven total | 2 | 28m | 14m |
| task setup (not a test run) | 9 | 38s | 4s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

1.2× average, peak 4, ceiling 4. Ready tasks waiting for a slot: 4s.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 30m | 30% |
| 1 | 51m | 50% |
| 2 | 4m | 4% |
| 3 | 8m | 8% |
| 4 | 8m | 8% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 9 | 2s | 2s | 0 |
| task setup | 9 | 38s | — | 0 |
| waiting for a slot | 8 | 4s | — | 0 |
| report pick-up | 22 | 0s | — | 0 |
| hand-off, wait idle | 18 | 2m | — | 0 |
| merge | 9 | 1s | 1s | 0 |
| worktree remove | 9 | 8s | 8s | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 2 | 28m | — | 0 |
| tests fix | 1 | 14m | — | 0 |
| report | 1 | 19s | — | 0 |
| session start-up | 20 | 6s | — | 0 |
| pir's own total |  | 46m | 12s |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 1h43m | 1 |
| loop pass | 4310 | 1m | 0 |
| waiting on someone | 4 | 1m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 37s | 1 |
| coordinator agent | 29s | 3 |
| nothing else moving, waiting on you | 37s |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 1k | 504k | 49.1M | 1.6M |
| total | 1k | 504k | 49.1M | 1.6M |

Limits (machine-wide: includes anything else you ran): 5-hour 10% → 10%, weekly 56% → 56%.
