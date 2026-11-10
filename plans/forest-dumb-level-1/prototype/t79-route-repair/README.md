# Route repair tools (T75's, for T79)

Throwaway. Three test files T75 used to turn inputs made with creatures frozen into inputs that reach
the same goal with creatures live. Not built by any crate: copy one into
`games/forest-dumb/engine/crates/fd-conform/tests/` and run it by name, then remove it.

- `repair.rs`: `CASE=collect-return OUT=/tmp/out.txt [SEED=n] [ITERS=n] [NOLIVES=1] [DUMP=1] cargo test -p fd-conform --test repair -- --nocapture`.
  The case's script must be the frozen one. Engine alone; writes the repaired script body to `OUT`.
- `repair_on_original.rs`: `CASE=... REF=<frozen script name>:<frames> OUT=... cargo test ...`. Plays each
  round's candidate on the original. Unfinished (tasks/T79, "What T75's searches tried").
- `probe.rs`: `SCR=case1,case2 [TRACE=1] [FRAMES=n] [NOPIC=1] [LAG=1] [STRIP=none|zero|fab|fabstart]`. Runs
  cases on the original with the creature slots added to their variables and prints each verdict's
  head and where score, lives and slots change; `STRIP` changes the measured cuts handed to the engine.

## T79's tools (2026-11-10)

T79 gave `death-water`, `mechanic-vanish` and `mechanic-spring` their live inputs with these two; both
play every candidate on the original and take the case's engine there as the original's state.

- `goto.rs` (copy into `fd-plan/tests/`, add `amiga-chips` as a dev-dependency of fd-plan while it is
  there): `CASE=death-water OUT=/tmp/x.txt PROG="G166,71 G152,61 H05x14 H04x9 H00x260" [PREFIX=file]
  [EVERY=n] cargo test --release -p fd-plan --test goto goto -- --nocapture`. `Gc,r` is fd-plan's
  explorer to a cell (`Goal::Reach`) from the state the original is in, planned again from the pass
  where the original's run left the plan and the leg no longer arrives; `Hjjxn` holds joystick byte
  `jj` for `n` steps. It made `death-water` and `mechanic-vanish` outright.
- `follow.rs` (copy into `fd-conform/tests/`): `CASE=... REF=<frozen script>:<frames> OUT=...
  [START=file] [FREE=n] [NOLIVES=1] [TOL=n] [ADV=n] [OFF=0]`. `repair.rs` with the original in the
  loop: the search runs from the engine's state a little before the original's run leaves the frozen
  path, under three timings. `REF` needs `Game::freeze_as_a_case`, which T79 removed: it runs on a
  commit before that. It made `mechanic-spring` from `camera-bottom-right`'s live opening.
- `differs-up-and-down.txt`, `differs-anim-ptr.txt`: two candidates on which the engine and the
  original differ (FINDINGS 2026-11-10), as scripts from the level sync point, `# frames` their length.
