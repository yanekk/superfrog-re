# The route on the original (T48's tool, 2026-11-10)

Throwaway, built by no crate. `onoriginal.rs` planned `engine/conform/scripts/route-W1-1.txt`: T27's
route (`fd-plan/tests/routes/W1-1.txt`) played on the original a step at a time with the case's
engine beside it, as `fd_conform::run_inputs` does, variables only.

- Each step is fed the plan's input for the pass that reads its joystick next (`PassInputs::next_read`),
  so a pass reading a step late still gets its own input.
- After every pass the engine's state is held to the plan's forecast (character, lives, contact, items,
  creature slots). Where the original leaves it, and every 200 steps, the rest of the plan is forecast
  on the engine alone (`Game::own_cuts`) 1 200 steps ahead from the original's state. A life lost or a
  mark missed there is repaired by doubling or dropping one to three inputs in the 150 steps before it,
  else by an explorer leg (`amiga_plan::Goal::Reach`) to a later mark of the plan.
- It stops at the first pass where the engine and the original differ and prints the original's values
  over the last 14 passes, the measured cuts and the last steps' joystick reads. That is how T48's
  three differences were found.

To run it: copy it to `games/forest-dumb/engine/crates/fd-plan/tests/zz_onoriginal.rs`, add
`amiga-oracle = { path = "../../../../../toolkit/crates/amiga-oracle" }` under `[dev-dependencies]` of
fd-plan while it is there, then from `engine/`:

```
REF=tests/routes/W1-1.txt OUT=/tmp/t48/route cargo test --release -p fd-plan --test zz_onoriginal -- --nocapture
```

`REF` is the plan to start from, an input a pass. It writes `OUT.txt` (the route, an input a step),
`OUT.wip.txt` and `OUT.wip.plan` every 200 steps (`RESUME=1` goes on from them), `OUT.differs.txt` at a
difference. `MAXSTEPS=n` stops early; `DUMP=addr,count,stride,col` (hex) prints memory at a difference.
The whole level took 77 s: 28 passes left the plan, 6 repairs, none by a leg, no step taken back.
