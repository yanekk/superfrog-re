# rust-engine — run stats

Build of `pir/rust-engine` in superfrog-re, 2026-10-11 11:46 → 2026-10-19 16:50 UTC, finished. Written by pir at the end of the run.

|  | Total | Detail |
| --- | ---: | --- |
| wall-clock | 197h04m | 191h24m running, 5h39m stopped |
| workers busy | 204h32m | 1.1× parallel, peak 4 of 4 |
| tests | 17h21m |  |
|     agent-driven | 16h48m | 8% of worker time, 1126 runs |
|     pir-driven | 32m | 32m end gate |
| waited on you | 69h22m | 27h39m with nothing else moving |
| pir's own steps | 491h25m | 6m of it holding the run |
| tokens | 5643.6M | 5541.3M cache read |

## By task

| Task | Implement | Review | Agent-driven tests | Tokens | Waited on you |
| --- | ---: | ---: | ---: | ---: | ---: |
| T00 | 28m | 13m | 16s | 18.4M | 2m |
| T01 | 9m | 4m | 24s | 4.8M | — |
| T02 | 14m | 5m | 0s | 9.2M | — |
| T03 | 6m | 2m | 16s | 4.3M | — |
| T04 | 8m | 2m | 0s | 4.6M | — |
| T05 | 34m | 19m | 2m | 28.3M | — |
| T06 | 10m | 5m | 26s | 7.6M | — |
| T07 | 35m | 13m | 4m | 75.2M | 5m |
| T08 | 32m | 5m | 1m | 34.9M | — |
| T09 | 5m | 2m | 1m | 3.9M | — |
| T10 | 41m | 9m | 0s | 23.2M | — |
| T11 | 39m | 10m | 2m | 29.5M | — |
| T12 | 14m | 5m | 34s | 12.3M | — |
| T13 | 1h07m | 13m | 23s | 131.7M | — |
| T14 | 2h57m | 38m | 6m | 338.8M | 14h25m |
| T15 | 1h07m | 8m | 9m | 130.7M | 11h52m |
| T16 | 2m | 3m | 0s | 4.2M | 3m |
| T17 | 2m | 1m | 0s | 2.4M | 3m |
| T18 | 1h00m | 17m | 23m | 21.1M | — |
| T19 | 1h58m | 4m | 0s | 179.4M | 2m |
| T20 | 2h09m | 43m | 6m | 223.9M | 10h00m |
| T21 | 1h45m | 17m | 15s | 69.6M | 2m |
| T22 | 23m | 6m | 46s | 15.9M | 6h56m |
| T23 | 39m | 11m | 7m | 33.6M | — |
| T24 | 31m | 8m | 3m | 21.4M | — |
| T25 | 30m | 3m | 11m | 12.9M | — |
| T26 | 34m | 23m | 3m | 24.2M | — |
| T27 | 57m | 40m | 3s | 19.5M | — |
| T28 | 30m | 4m | 21m | 13.3M | — |
| T29 | 53m | 13m | 0s | 40.1M | 3m |
| T30 | 1h55m | 12m | 4m | 54.4M | — |
| T31 | 22m | 10m | 1m | 20.7M | 6h55m |
| T32 | 2h59m | 19m | 2m | 191.0M | 15h07m |
| T33 | 4h09m | 5m | 0s | 79.1M | — |
| T34 | 27m | 10m | 1m | 31.0M | — |
| T35 | 46m | 7m | 33s | 54.5M | — |
| T36 | 1h51m | 31m | 1m | 191.6M | 40s |
| T37 | 54m | 15m | 13m | 54.7M | — |
| T38 | 21m | 24m | 2m | 35.7M | — |
| T39 | 43m | 6m | 4m | 44.6M | — |
| T40 | 2h35m | 20m | 8m | 127.2M | — |
| T41 | 2h33m | 10m | 3m | 61.1M | — |
| T42 | 54m | 12m | 8m | 110.3M | — |
| T43 | 1h54m | 27m | 21m | 81.7M | — |
| T44 | 47m | 26m | 18m | 54.2M | — |
| T46 | 1h47m | 17m | 2m | 84.1M | 44s |
| T47 | 1h04m | 30m | 9m | 16.7M | — |
| T48 | 52m | 32m | 20m | 19.7M | — |
| T49 | 7m | 11m | 1m | 3.3M | — |
| T50 | 46m | 27m | 3m | 14.0M | — |
| T51 | 11m | 7m | 0s | 7.3M | — |
| T52 | 21m | 5m | 15s | 12.1M | — |
| T53 | 50m | 13m | 0s | 17.7M | 22s |
| T54 | 31m | 22m | 4m | 21.1M | — |
| T55 | 1h05m | 17m | 5m | 13.3M | — |
| T56 | 4h50m | 44m | 0s | 86.0M | — |
| T70 | 5m | 3m | 0s | 9.0M | — |
| T71 | 18m | 15m | 3m | 20.4M | — |
| T72 | 5m | 1m | 0s | 7.6M | — |
| T73 | 44m | 12m | 11m | 36.6M | — |
| T74 | 1h30m | 21m | 26m | 41.4M | — |
| T75 | 10m | 18m | 15m | 11.7M | — |
| T76 | 10m | 16m | 1m | 9.5M | — |
| T77 | 7m | 3m | 2m | 5.6M | — |
| T78 | 11m | 6m | 3m | 9.8M | — |
| T79 | 29m | 23m | 3m | 28.5M | — |
| T81 | 18m | 7m | 3m | 7.2M | 2h40m |
| T82 | 8m | 3m | 47s | 8.0M | — |
| T83 | 1h14m | 4m | 1m | 12.4M | — |
| T84 | 19m | 22m | 10s | 13.4M | — |
| T85 | 1h32m | 45m | 0s | 18.1M | — |
| T86 | 2h15m | 38m | 0s | 45.1M | — |
| T87 | 1m | 2m | 30s | 3.3M | — |
| T88 | 50s | 2m | 18s | 1.8M | — |
| T89 | 9m | 6m | 2m | 6.4M | — |
| T90 | 3m | 4m | 22s | 2.3M | — |
| T91 | 6m | 2m | 0s | 4.3M | — |
| T92 | 10m | 4m | 0s | 7.8M | 9m |
| T93 | 18m | 15m | 4m | 17.0M | — |
| T94 | 25m | 10m | 1m | 18.2M | — |
| T95 | 9m | 4m | 1m | 8.4M | — |
| T96 | 27m | 17m | 3m | 19.1M | — |
| T97 | 33m | 16m | 7m | 9.8M | — |
| T100 | 1h24m | 46m | 11s | 58.5M | 26m |
| T101 | 27m | 15m | 1m | 18.2M | — |
| T102 | 25m | 10m | 58s | 28.4M | — |
| T103 | 2h19m | 25m | 19m | 153.8M | 0s |
| T104 | 3h49m | 23m | 1m | 119.6M | — |
| T105 | 35m | 9m | 5m | 25.8M | — |
| T106 | 3h17m | 47m | 37m | 40.5M | — |
| T107 | 42m | 6m | 2m | 9.3M | — |
| T108 | 37m | 12m | 2m | 13.6M | — |
| T109 | 14m | 6m | 2m | 7.8M | — |
| T110 | 13m | 4m | 1m | 11.9M | — |
| T111 | 47m | 6m | 2m | 16.8M | — |
| T112 | 1h20m | 8m | 5m | 15.6M | — |
| T113 | 3h29m | 6m | 3m | 19.8M | — |
| T114 | 10m | 6m | 2m | 6.0M | — |
| T115 | 1h55m | 34m | 20m | 99.9M | — |
| T120 | 44m | 29m | 16m | 16.3M | — |
| T121 | 10m | 11m | 0s | 4.3M | — |
| T122 | 1h22m | 6m | 1m | 51.5M | — |
| T123 | 25m | 5m | 1m | 10.1M | — |
| T124 | 15m | 6m | 1m | 6.8M | — |
| T125 | 34m | 8m | 4m | 15.1M | — |
| T126 | 1h05m | 20m | 30m | 18.5M | — |
| T127 | 21m | 10m | 0s | 8.7M | — |
| T128 | 2h02m | 0s | 9m | 0 | — |
| T129 | 1h28m | 23m | 27m | 37.5M | — |
| T130 | 40m | 8m | 1m | 23.7M | 5m |
| T132 | 1h35m | 10m | 11m | 4.0M | — |
| T133 | 2h26m | 6m | 0s | 32.3M | 5m |
| T140 | 23s | 0s | 0s | 261k | 6m |
| T144 | 1h46m | 40m | 31m | 17.7M | — |
| T145 | 40m | 12m | 0s | 5.0M | — |
| T147 | 40m | 16m | 50s | 14.1M | — |
| T148 | 28m | 13m | 8m | 11.7M | — |
| T149 | 37m | 33m | 13m | 9.3M | — |
| T150 | 1h08m | 12m | 58s | 34.2M | — |
| T151 | 1h31m | 11m | 1m | 9.9M | — |
| T154 | 1h05m | 20m | 12m | 27.4M | — |
| T160 | 9m | 29m | 19m | 5.5M | — |
| T162 | 23m | 17m | 24m | 2.7M | — |
| T163 | 7m | 13m | 8m | 4.5M | — |
| T164 | 56m | 12m | 9m | 8.6M | — |
| T166 | 30m | 5m | 4m | 9.0M | — |
| T167 | 1h39m | 17m | 2m | 24.3M | — |
| T168 | 3h30m | 12m | 7m | 42.1M | — |
| T180 | 45m | 26m | 4m | 35.7M | — |
| T181 | 1h22m | 25m | 32m | 51.1M | 5m |
| T182 | 1h26m | 14m | 1m | 44.4M | — |
| T183 | 1h51m | 1h12m | 31s | 76.0M | — |
| T184 | 1h23m | 55m | 30m | 28.7M | — |
| T185 | 30m | 20m | 0s | 8.1M | — |
| T190 | 13m | 36m | 0s | 6.1M | — |
| T191 | 22m | 32m | 10m | 14.5M | — |
| T192 | 22m | 20m | 5m | 6.7M | — |
| T193 | 31m | 10m | 19m | 7.4M | — |
| T194 | 37m | 17m | 17m | 9.8M | — |
| T195 | 54m | 18m | 15m | 14.2M | — |
| T196 | 2h01m | 21m | 36s | 30.3M | — |
| T197 | 40m | 18m | 32s | 10.1M | — |
| T198 | 19m | 25m | 12m | 5.0M | — |
| T199 | 1h30m | 28m | 1m | 51.6M | — |
| T200 | 28m | 14m | 1m | 20.8M | — |
| T201 | 33m | 17m | 22m | 16.1M | — |
| T202 | 28m | 35m | 0s | 10.1M | — |
| T203 | 2h37m | 27m | 20s | 83.3M | — |
| T204 | 41m | 24m | 5m | 34.1M | — |
| T205 | 1h17m | 17m | 0s | 47.8M | — |
| T206 | 47m | 11m | 11s | 63.6M | — |
| T207 | 5h51m | 12m | 7m | 39.4M | — |
| T208 | 1h21m | 8m | 0s | 38.7M | — |
| T209 | 2h21m | 19m | 4m | 77.4M | — |
| T210 | 26m | 30m | 1m | 7.9M | — |
| T211 | 19m | 23m | 3m | 17.5M | — |
| T212 | 1h39m | 34m | 21m | 33.2M | — |
| T213 | 2h34m | 18m | 6m | 77.0M | 0s |
| T214 | 43m | 39m | 28m | 14.3M | — |
| T215 | 26m | 20m | 11m | 7.1M | — |
| T216 | 1h21m | 27m | 18m | 29.3M | — |
| T217 | 3m | 1m | 0s | 2.5M | — |
| tests-fix | 0s | 0s | 0s | 0 | — |

## Work by role

| Role | Sessions | Active | Model | Tools | Tests | Tokens |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| implementers | 174 | 159h08m | 48h47m | 110h26m | 9h22m | 4866.1M |
| reviewers | 167 | 44h44m | 10h54m | 33h50m | 7h16m | 670.6M |
| test-fix helper | 1 | 39m | 16m | 22m | 10m | 8.6M |
| coordinator agent | 1 | 50m | 30m | 20m | 0s | 98.2M |
| workers total | 342 | 204h32m | 59h58m | 144h39m | 16h48m |  |

Workers are the implementers, reviewers and the two helpers. The coordinator agent and the finisher count in tokens only, never in worker time or parallelism.

## Tests

### agent-driven

Workers running `pir-test` in their own sessions.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| full | 0 | 0s | — |
| partial | 1126 | 16h48m | 53s |
| full, ran without pir-test | 0 | 0s | — |
| agent-driven total | 1126 | 16h48m | 53s |

8% of worker active time, and part of it. Untimed runs (started in the background): 25.

### pir-driven

pir running the suite itself, one suite at a time on the machine.

| Kind | Runs | Time | Average |
| --- | ---: | ---: | ---: |
| end gate | 2 | 32m | 16m |
| pir-driven total | 2 | 32m | 16m |
| task setup (not a test run) | 174 | 21m | 7s |

The end gate's time includes its wait for its turn. Task setup is not in the total.

## Parallelism

1.1× average, peak 4, ceiling 4. Ready tasks waiting for a slot: 485h25m.

| Workers busy | Time | Share of running time |
| ---: | ---: | ---: |
| 0 | 93h20m | 49% |
| 1 | 45h57m | 24% |
| 2 | 15h51m | 8% |
| 3 | 18h08m | 9% |
| 4 | 18h06m | 9% |

## pir's own steps

| Step | Count | Total | Holding the run | Left open |
| --- | ---: | ---: | ---: | ---: |
| start-up | 7 | 12s | 12s | 0 |
| reconcile | 7 | 23s | 23s | 0 |
| worktree create | 174 | 43s | 43s | 0 |
| task setup | 174 | 21m | — | 0 |
| waiting for a slot | 179 | 485h25m | — | 14 |
| report pick-up | 456 | 1m | — | 0 |
| hand-off, wait idle | 336 | 1h00m | — | 0 |
| merge | 178 | 31s | 31s | 0 |
| conflict fix | 13 | 1h28m | — | 0 |
| worktree remove | 160 | 4m | 4m | 0 |
| base sync | 1 | 0s | — | 0 |
| end gate | 2 | 32m | — | 0 |
| tests fix | 1 | 2h28m | — | 0 |
| report | 1 | 36s | — | 0 |
| session start-up | 343 | 8m | — | 0 |
| pir's own total |  | 491h25m | 6m |  |

Session start-up is not in the total; pir's step time overlaps worker time and is never added to it.

Listed apart, since they contain other steps or are not pir's work:

| Step | Count | Total | Left open |
| --- | ---: | ---: | ---: |
| coordinator running | 7 | 191h24m | 2 |
| loop pass | 291023 | 10h31m | 2 |
| waiting on someone | 160 | 69h54m | 2 |

## Waiting

| On | Time | Times |
| --- | ---: | ---: |
| you | 69h22m | 41 |
| coordinator agent | 31m | 119 |
| nothing else moving, waiting on you | 27h39m |  |

## Tokens by model

| Model | Input | Output | Cache read | Cache write |
| --- | ---: | ---: | ---: | ---: |
| claude-opus-5-5[1m] | 57k | 18.3M | 5541.3M | 83.8M |
| total | 57k | 18.3M | 5541.3M | 83.8M |

Limits (machine-wide: includes anything else you ran): 5-hour 51% → 27%, weekly 48% → 7%.
