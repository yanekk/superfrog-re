# one-clock — run stats

Build of `pir/one-clock` in superfrog-re, 2026-10-27 20:06 → 2026-10-28 17:19 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 21h13m | 21h13m running, 0s stopped |
| workers busy | 2h29m | 0.1× parallel, peak 1 of 4 |
| tests | 4h26m |  |
|     agent-driven | 57m | 39% of worker time, 32 runs |
|     pir-driven | 3h28m | 3h28m end gate |
| waited on you | 2h30m | 2h30m with nothing else moving |
| pir's own steps | 6h20m | 19s of it holding the run |
| tokens | 34.6M | 32.9M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T01 | 19m | 30m | 31s | 13.2M | — |
| T02 | 35m | 15m | 25m | 10.1M | — |
| T03 | 19m | 9m | 16m | 9.3M | 2h30m |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 3 | 1h15m | 21m | 53m | 36m | 23.7M |
| reviewers | 3 | 55m | 9m | 45m | 5m | 9.0M |
| base sync helper | 1 | 6m | 1m | 5m | 4m | 743k |
| test-fix helper | 1 | 12m | 45s | 11m | 11m | 679k |
| coordinator agent | 1 | 37s | 17s | 19s | 0s | 465k |
| workers total | 8 | 2h29m | 32m | 1h56m | 57m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 32 | 57m | 1m |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 32 | 57m | 1m |

39% of worker active time, and part of it. Untimed runs (started in the background): 6.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 3 | 3h28m | 1h09m |
| pir-driven total | 3 | 3h28m | 1h09m |
| task setup (not a test run) | 3 | 23s | 7s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.1× average, peak 1, ceiling 4. Ready tasks waiting for a slot: 0s.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 18h43m | 88% |
| 1 | 2h29m | 12% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 3 | 1s | 1s | 0 |
| task setup | 3 | 23s | — | 0 |
| waiting for a slot | 2 | 0s | — | 0 |
| report pick-up | 8 | 0s | — | 0 |
| hand-off, wait idle | 6 | 51s | — | 0 |
| merge | 3 | 0s | 0s | 0 |
| worktree remove | 3 | 17s | 17s | 0 |
| base sync | 1 | 6m | — | 0 |
| end gate | 3 | 3h28m | — | 0 |
| tests fix | 1 | 2h43m | — | 0 |
| report | 1 | 16s | — | 0 |
| session start-up | 9 | 4s | — | 0 |
| pir's own total |  | 6h20m | 19s |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 21h13m | 1 |
| loop pass | 16474 | 2m | 0 |
| waiting on someone | 1 | 2h30m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 2h30m | 1 |
| coordinator agent | 0s | 0 |
| nothing else moving, waiting on you | 2h30m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 648 | 154k | 32.9M | 1.5M |
| total | 648 | 154k | 32.9M | 1.5M |

Limits (machine-wide: includes anything else you ran): 5-hour 51% → 51%, weekly 64% → 64%.
