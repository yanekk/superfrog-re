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
