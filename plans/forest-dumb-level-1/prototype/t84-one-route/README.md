# One route for both sides (T84's tools, 2026-11-10)

Throwaway, built by no crate. `both.rs` found `fd-plan/tests/routes/W1-1.txt`, the route the engine
alone and the original both finish with no life lost (`games/forest-dumb/docs/13-full-route.md`,
"The one route", says how it works). `measure.rs` measured why one script plays differently on the
two sides.

To run either: copy it to `games/forest-dumb/engine/crates/fd-plan/tests/zz_both.rs` (or
`zz_measure.rs`), add `amiga-oracle = { path = "../../../../../toolkit/crates/amiga-oracle" }` under a
`[dev-dependencies]` of fd-plan while it is there, then from `engine/`:

```
REF=$PWD/crates/fd-plan/tests/routes/W1-1-planned.txt OUT=/tmp/t84/route \
  cargo test --release -p fd-plan --test zz_both -- --nocapture > /tmp/t84/run.log 2>&1
```

- `REF` is the plan to start from, an input a step. It writes `OUT.txt` (the route) when both sides
  end the level, `OUT.wip.txt` every 200 steps (`RESUME=1` goes on from it), `OUT.stuck.txt` after
  60 rewinds, `OUT.differs-<pass>.txt` where the engine beside the original differs from it (it
  then goes back 300 steps and passes the place another way). `MAXSTEPS=n` stops early.
- The log's `step N000:` lines show both sides' place and items left; the two must stay level.
  `no way past +f` names a failure no change mended: nearer than 250 steps it rewinds, further it
  plays on and plans again closer.
- The whole level took 2 008 s on 10 threads: 39 passes left the forecast, 49 checks planned again,
  4 rewinds, all at the difference below.

`SCRIPT=<script> [MAXSTEPS=n] [CSV=file] [WATCH=from,to] cargo test --release -p fd-plan --test
zz_measure -- --nocapture` plays a script on both sides and prints the passes that read another
step's joystick, where the character first parts, and with `CSV` a row a pass (lateness, creatures
drawn, cells in the pass's list) from which `both.rs`'s rule `late` was read. `WATCH` prints each
pass's interrupted PC, cut and creature slots on both sides.

`differs-creature-slot.txt`: 1 905 steps on which the engine and the original differ at pass 1 905
(a creature started while a pass cut the scan; docs/13, "A difference it went round"). Left.

What the search tried first and dropped: marks held by step count alone (a joystick held 16 steps
put the engine alone late and every later check failed); accepting a change that moved a failure
only a few steps on (the plan wandered); holding each side to the marks apart (the original missed
an item the engine alone took, unseen for 4 000 steps).
