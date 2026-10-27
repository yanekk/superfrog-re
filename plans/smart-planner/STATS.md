# smart-planner — run stats

Build of `pir/smart-planner` in superfrog-re, 2026-10-21 07:56 → 2026-10-26 09:28 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 121h32m | 120h48m running, 43m stopped |
| workers busy | 44h36m | 0.4× parallel, peak 3 of 4 |
| tests | 3h12m |  |
|     agent-driven | 1h46m | 4% of worker time, 102 runs |
|     pir-driven | 1h25m | 1h25m end gate |
| waited on you | 10h55m | 10h53m with nothing else moving |
| pir's own steps | 4h27m | 1m of it holding the run |
| tokens | 654.4M | 631.7M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T00 | 2h04m | 3m | 0s | 21.7M | 8m |
| T01 | 16m | 17m | 0s | 13.8M | — |
| T02 | 7m | 4m | 0s | 9.4M | — |
| T03 | 7m | 2m | 29s | 7.3M | — |
| T04 | 2h18m | 18m | 55s | 65.4M | 2h17m |
| T05 | 5m | 3m | 29s | 4.5M | — |
| T06 | 9m | 3m | 0s | 5.8M | — |
| T07 | 8m | 4m | 1m | 6.3M | — |
| T08 | 2h44m | 6m | 4m | 28.0M | 1h22m |
| T09 | 45m | 14m | 6m | 12.1M | — |
| T10 | 1h12m | 22m | 5m | 13.7M | — |
| T11 | 43m | 6m | 15m | 13.7M | — |
| T12 | 29m | 30m | 17m | 15.6M | 1m |
| T14 | 13h08m | 22m | 0s | 110.0M | 1h21m |
| T15 | 12m | 16m | 1s | 19.9M | 1m |
| T17 | 12m | 3m | 2m | 11.3M | 9m |
| T18 | 5m | 2m | 1m | 4.0M | — |
| T19 | 4m | 9m | 5m | 3.9M | — |
| T20 | 53m | 0s | 0s | 12.6M | 2h46m |
| T21 | 11m | 14m | 3m | 7.3M | — |
| T22 | 45m | 18m | 6m | 9.0M | — |
| T23 | 1h04m | 13m | 0s | 8.9M | — |
| T24 | 31m | 17m | 6s | 35.4M | — |
| T25 | 42m | 14m | 1m | 24.5M | 1h32m |
| T26 | 54m | 3m | 7s | 34.8M | — |
| T27 | 15m | 4m | 1m | 16.7M | 20m |
| T28 | 25m | 14m | 3m | 16.9M | — |
| T29 | 5h21m | 13m | 3m | 30.3M | — |
| T30 | 13m | 5m | 5m | 10.1M | — |
| T31 | 32m | 16m | 0s | 15.4M | — |
| T32 | 36m | 24m | 4m | 20.1M | — |
| T33 | 28m | 17m | 0s | 10.7M | — |
| T34 | 8m | 2m | 0s | 7.3M | 53m |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 34 | 38h04m | 6h15m | 31h48m | 50m | 523.4M |
| reviewers | 32 | 6h15m | 1h58m | 4h16m | 41m | 104.5M |
| test-fix helper | 1 | 16m | 1m | 15m | 14m | 1.2M |
| coordinator agent | 1 | 13m | 12m | 1m | 0s | 25.2M |
| workers total | 67 | 44h36m | 8h16m | 36h20m | 1h46m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 102 | 1h46m | 1m |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 102 | 1h46m | 1m |

4% of worker active time, and part of it. Untimed runs (started in the background): 12.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 3 | 1h25m | 28m |
| pir-driven total | 3 | 1h25m | 28m |
| task setup (not a test run) | 34 | 3m | 6s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

0.4× average, peak 3, ceiling 4. Ready tasks waiting for a slot: 11s.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 81h32m | 67% |
| 1 | 34h14m | 28% |
| 2 | 4h43m | 4% |
| 3 | 18m | 0% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 3 | 0s | 0s | 0 |
| reconcile | 3 | 0s | 0s | 0 |
| worktree create | 34 | 7s | 7s | 0 |
| task setup | 34 | 3m | — | 0 |
| waiting for a slot | 30 | 11s | — | 0 |
| report pick-up | 92 | 3s | — | 0 |
| hand-off, wait idle | 65 | 9m | — | 0 |
| merge | 33 | 6s | 6s | 0 |
| conflict fix | 1 | 52s | — | 0 |
| worktree remove | 32 | 44s | 44s | 0 |
| base sync | 1 | 1s | — | 0 |
| end gate | 3 | 1h25m | — | 0 |
| tests fix | 1 | 2h45m | — | 0 |
| report | 1 | 39s | — | 0 |
| session start-up | 68 | 1m | — | 0 |
| pir's own total |  | 4h27m | 1m |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 3 | 120h48m | 2 |
| loop pass | 101196 | 1h40m | 0 |
| waiting on someone | 42 | 11h02m | 0 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 10h55m | 13 |
| coordinator agent | 6m | 29 |
| nothing else moving, waiting on you | 10h53m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5[1m] | 8k | 2.3M | 631.7M | 20.4M |
| total | 8k | 2.3M | 631.7M | 20.4M |

Limits (machine-wide: includes anything else you ran): 5-hour 15% → ?, weekly 93% → 40%.
