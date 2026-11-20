# real-widescreen — run stats

Build of `pir/real-widescreen` in superfrog-re, 2026-10-26 20:55 → 2026-10-27 09:07 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 12h11m | 12h10m running, 51s stopped |
| workers busy | 8h30m | 0.7× parallel, peak 4 of 4 |
| tests | 57m |  |
|     agent-driven | 42m | 8% of worker time, 71 runs |
|     pir-driven | 15m | 15m end gate |
| waited on you | 1h36m | 1h36m with nothing else moving |
| pir's own steps | 1h17m | 40s of it holding the run |
| tokens | 206.0M | 201.0M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T00 | 30m | 26m | 0s | 15.0M | — |
| T01 | 4m | 7m | 0s | 5.9M | — |
| T02 | 25m | 7m | 0s | 9.4M | — |
| T03 | 29m | 6m | 0s | 17.5M | — |
| T04 | 18m | 15m | 0s | 12.9M | — |
| T05 | 11m | 8m | 0s | 9.0M | — |
| T06 | 17m | 11m | 50s | 12.7M | — |
| T07 | 9m | 4m | 0s | 6.0M | — |
| T08 | 40m | 30m | 39m | 29.4M | — |
| T09 | 14m | 13m | 0s | 14.0M | — |
| T10 | 27m | 10m | 2m | 14.2M | — |
| T11 | 50m | 6m | 0s | 22.9M | — |
| T12 | 56m | 6m | 17s | 30.6M | 1h36m |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 15 | 5h37m | 2h24m | 3h12m | 21m | 150.1M |
| reviewers | 13 | 2h34m | 55m | 1h39m | 21m | 49.9M |
| test-fix helper | 1 | 19m | 2m | 16m | 0s | 2.1M |
| coordinator agent | 1 | 4m | 4m | 33s | 0s | 3.7M |
| workers total | 29 | 8h30m | 3h22m | 5h07m | 42m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 71 | 42m | 36s |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 71 | 42m | 36s |

8% of worker active time, and part of it. Untimed runs (started in the background): 0.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 15m | 7m |
| pir-driven total | 2 | 15m | 7m |
| task setup (not a test run) | 15 | 1m | 4s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.7× average, peak 4, ceiling 4. Ready tasks waiting for a slot: 19m.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 5h57m | 49% |
| 1 | 4h50m | 40% |
| 2 | 49m | 7% |
| 3 | 12m | 2% |
| 4 | 21m | 3% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 2 | 0s | 0s | 0 |
| reconcile | 2 | 0s | 0s | 0 |
| worktree create | 15 | 3s | 3s | 0 |
| task setup | 15 | 1m | — | 0 |
| waiting for a slot | 11 | 19m | — | 0 |
| report pick-up | 33 | 0s | — | 0 |
| hand-off, wait idle | 26 | 4m | — | 0 |
| merge | 13 | 1s | 1s | 0 |
| worktree remove | 13 | 34s | 34s | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 2 | 15m | — | 0 |
| tests fix | 1 | 36m | — | 0 |
| report | 1 | 26s | — | 0 |
| session start-up | 30 | 17s | — | 0 |
| pir's own total |  | 1h17m | 40s |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 2 | 12h10m | 1 |
| loop pass | 15847 | 6m | 0 |
| waiting on someone | 17 | 1h37m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 1h36m | 12 |
| coordinator agent | 1m | 5 |
| nothing else moving, waiting on you | 1h36m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 2k | 627k | 141.0M | 2.2M |
| claude-opus-5-5[1m] | 1k | 343k | 59.9M | 1.8M |
| total | 3k | 970k | 201.0M | 4.0M |

Limits (machine-wide: includes anything else you ran): 5-hour ? → 2%, weekly 44% → 46%.
