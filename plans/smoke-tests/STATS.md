# smoke-tests — run stats

Build of `pir/smoke-tests` in superfrog-re, 2026-10-31 16:49 → 22:33 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 5h44m | 5h44m running, 0s stopped |
| workers busy | 2h50m | 0.5× parallel, peak 3 of 4 |
| tests | 1h16m |  |
|     agent-driven | 1h02m | 37% of worker time, 44 runs |
|     pir-driven | 14m | 14m end gate |
| waited on you | 0s | 0s with nothing else moving |
| pir's own steps | 21m | 1m of it holding the run |
| tokens | 16.6M | 15.0M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T01 | 9m | 16m | 7m | 2.0M | — |
| T02 | 10m | 11m | 7m | 1.8M | — |
| T03 | 38m | 3m | 35m | 2.0M | — |
| T04 | 3m | 25m | 6m | 2.0M | — |
| T05 | 10m | 3m | 33s | 1.7M | — |
| T06 | 22m | 6m | 4m | 2.4M | — |
| T07 | 1m | 3m | 30s | 2.6M | — |
| T08 | 1m | 1m | 32s | 1.1M | — |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 8 | 1h37m | 21m | 1h15m | 40m | 7.4M |
| reviewers | 8 | 1h12m | 19m | 52m | 21m | 8.4M |
| coordinator agent | 1 | 1m | 25s | 44s | 0s | 720k |
| workers total | 16 | 2h50m | 41m | 2h08m | 1h02m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 44 | 1h02m | 1m |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 44 | 1h02m | 1m |

37% of worker active time, and part of it. Untimed runs (started in the background): 0.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 1 | 14m | 14m |
| pir-driven total | 1 | 14m | 14m |
| task setup (not a test run) | 8 | 1m | 13s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.5× average, peak 3, ceiling 4. Ready tasks waiting for a slot: 12s.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 3h58m | 69% |
| 1 | 44m | 13% |
| 2 | 58m | 17% |
| 3 | 2m | 1% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 8 | 18s | 18s | 0 |
| task setup | 8 | 1m | — | 0 |
| waiting for a slot | 6 | 12s | — | 0 |
| report pick-up | 21 | 0s | — | 0 |
| hand-off, wait idle | 16 | 3m | — | 0 |
| merge | 8 | 1s | 1s | 0 |
| worktree remove | 8 | 42s | 42s | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 1 | 14m | — | 0 |
| report | 1 | 24s | — | 0 |
| session start-up | 17 | 20s | — | 0 |
| pir's own total |  | 21m | 1m |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 5h44m | 1 |
| loop pass | 5588 | 3m | 0 |
| waiting on someone | 2 | 23s | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 0s | 0 |
| coordinator agent | 23s | 2 |
| nothing else moving, waiting on you | 0s |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 394 | 261k | 15.0M | 1.2M |
| total | 394 | 261k | 15.0M | 1.2M |

Limits (machine-wide: includes anything else you ran): 5-hour 16% → 16%, weekly 50% → 50%.
