# amiga-chips — run stats

Build of `pir/amiga-chips` in superfrog-re, 2026-10-27 09:27 → 15:51 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 6h23m | 6h23m running, 0s stopped |
| workers busy | 2h13m | 0.3× parallel, peak 1 of 4 |
| tests | 3h40m |  |
|     agent-driven | 1h13m | 55% of worker time, 33 runs |
|     pir-driven | 2h26m | 2h26m end gate |
| waited on you | 0s | 0s with nothing else moving |
| pir's own steps | 5h21m | 17s of it holding the run |
| tokens | 24.1M | 23.4M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T01 | 38m | 35m | 35m | 15.2M | — |
| T02 | 28m | 22m | 37m | 6.6M | — |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 2 | 1h07m | 15m | 51m | 34m | 16.6M |
| reviewers | 2 | 58m | 5m | 53m | 38m | 5.2M |
| test-fix helper | 1 | 7m | 2m | 4m | 46s | 1.7M |
| coordinator agent | 1 | 1m | 37s | 50s | 0s | 498k |
| workers total | 5 | 2h13m | 23m | 1h49m | 1h13m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 33 | 1h13m | 2m |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 33 | 1h13m | 2m |

55% of worker active time, and part of it. Untimed runs (started in the background): 1.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 2h26m | 1h13m |
| pir-driven total | 2 | 2h26m | 1h13m |
| task setup (not a test run) | 2 | 9s | 4s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.3× average, peak 1, ceiling 4. Ready tasks waiting for a slot: 0s.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 4h10m | 65% |
| 1 | 2h13m | 35% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 1 | 0s | 0s | 0 |
| reconcile | 1 | 0s | 0s | 0 |
| worktree create | 2 | 0s | 0s | 0 |
| task setup | 2 | 9s | — | 0 |
| waiting for a slot | 1 | 0s | — | 0 |
| report pick-up | 6 | 0s | — | 0 |
| hand-off, wait idle | 4 | 40s | — | 0 |
| merge | 2 | 0s | 0s | 0 |
| worktree remove | 2 | 16s | 16s | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 2 | 2h26m | — | 0 |
| tests fix | 1 | 2h53m | — | 0 |
| report | 1 | 47s | — | 0 |
| session start-up | 6 | 2s | — | 0 |
| pir's own total |  | 5h21m | 17s |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 1 | 6h23m | 1 |
| loop pass | 5442 | 48s | 0 |
| waiting on someone | 1 | 20s | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 0s | 0 |
| coordinator agent | 20s | 1 |
| nothing else moving, waiting on you | 0s |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5 | 438 | 103k | 23.4M | 617k |
| total | 438 | 103k | 23.4M | 617k |

Limits (machine-wide: includes anything else you ran): 5-hour 2% → 2%, weekly 53% → 53%.
