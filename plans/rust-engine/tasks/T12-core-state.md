# T12 — core-state

**Phase:** 2 · **Depends on:** T05, T06, T07 · **Weight:** medium

## Goal

The skeleton every mechanic hangs on: the `Game` type and its step function (DESIGN §3.3), the
`layout` table pairing each modelled variable with the original's address, transplant from and probe
back to the original's memory, the input-script format, and the conformance runner that compares the
engine with the oracle frame by frame and names the first divergence. After this task a mechanic task
is "port the routine, map its variables, write scripts, make the runner green".

## Design sections this implements

DESIGN §2.1, §3.3, §3.4, §4.

## Files

- `engine/crates/sf-core/src/{lib.rs,game.rs,state.rs,layout.rs,rng.rs,scene.rs}`
- `engine/crates/sf-conform/src/{lib.rs,script.rs,sync.rs,compare.rs,report.rs,main.rs}`
- `engine/conform/scripts/` (committed, inputs only) and `engine/conform/README.md`
- `engine/crates/sf-conform/tests/{runner.rs,script.rs}`

## Interface

```rust
// sf-core (signatures from DESIGN §3.3, plus:)
pub struct VarDef { pub id: VarId, pub name: &'static str, pub addr: u32, pub width: Width /* B, W, L, or Bytes(n) */ }
pub static LAYOUT: &[VarDef];                       // one entry per modelled original variable
pub enum Scene { /* filled by T14, T19, T20, T21; T12 ships the enum with a Placeholder that renders black */ }

// sf-conform
pub struct InputScript { pub lines: Vec<(u32 /*frame*/, FrameInput)> }   // text: "120: R F" = right+fire held from frame 120
pub enum SyncPoint { PowerOn, AtFrame { script: String, frame: u32 } , AtLevel(LevelId) }  // AtLevel reached via the original's own level-select/password path
pub struct Divergence { pub frame: u32, pub first_var: Option<(VarDef, u32, u32)>, pub picture_diff: Option<PathBuf>, pub paula_diff: Option<String> }
pub fn run(case: &Case) -> Result<(), Divergence>    // Case = sync point + script + frame count + what to compare
// CLI: sf-conform run <case.toml> | sf-conform all | sf-conform record <case.toml>   (record = show the oracle's state per frame for script writing)
```

A variable is either in `LAYOUT` (transplanted in and probed out) or not modelled; there is no third
state, so a half-mapped variable cannot hide a divergence.

## Tests

- [ ] script parser: holds, releases, comments, out-of-order frames rejected, a script with only frame 0.
- [ ] transplant then probe with no step returns exactly the oracle's values for every `LAYOUT` entry.
- [ ] the runner on a deliberately wrong one-variable mutation reports that variable and that frame.
- [ ] picture divergence writes a diff PNG to a temp dir (never the repo) and names the first pixel.
- [ ] Paula divergence shows the first differing write with its index.
- [ ] RNG: the original's routine (from T07) on Moira and `rng.rs` agree for 100 000 calls from random states.

## Done when

- [ ] `sf-conform run` works on a case that compares only `LAYOUT` variables on the title screen, green.
- [ ] `engine/conform/README.md` explains how to write a script and a case.
- [ ] The lag/RNG decision from T07, if one was needed, is reflected in `Game::step` and §2.1.
