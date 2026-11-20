# T32 — routine-rig

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal
Give every routine port a judge before any is written: the routine rig that compares a port with the 68k original, the generator API it runs on, and the port register. Split out of T06 (owner, 2026-11-05) so the translation service (T33) and the generator batches can start before the level case runner exists, and so T06's builder holds one subsystem.
## Design sections this implements
§4, §4.1, §4.3.
## Reuses
`toolkit/docs/porting-routines.md` §3 and §4; `amiga-oracle/src/routine.rs` (hardware mode, `Cpu68k::set_hardware`), `sf-conform/src/routine.rs` as pattern, its `last_regs` included; the spike's `Gen` and `Setup` (`games/forest-dumb/spikes/muse-spark/judge/src/main.rs`), which this task makes the one generator type. `Gen` is extracted to the toolkit if it ends up generic, since this is its second user.
## Files
`games/forest-dumb/engine/crates/fd-conform/src/{routine.rs,gen/mod.rs,gen/spike.rs}` (the spike's generators, carried over and named as in `re/symbols.toml`), `fd-core/src/ported.rs`, `fd-conform/tests/{ported.rs,rig.rs}`, `toolkit/crates/amiga-oracle/src/routine.rs` and `sf-conform/src/routine.rs` if `Gen` moves, `engine/conform/README.md`, `smoke.toml`.
## Interface
```
pub struct Gen(u64);  pub struct Setup { /* registers, memory writes, custom-chip reads */ }
pub struct Generator { pub base: Option<fn(&mut [u8])>, pub case: fn(&mut Gen) -> Setup }   // fd-conform/src/gen/, one per routine, by name; base is the spike's fill_base (screens, map) laid once before all cases
pub fn generator(name: &str) -> Option<Generator>;
pub struct Ported { name: &str, hunk: u8, offset: u32, port: &str }  pub const PORTED: &[Ported];   // fd-core/src/ported.rs
pub struct Effects { pub regs: Regs, pub writes: Vec<(u32, u8)>, pub hw_writes: Vec<(u16, u16)> }   // what the port did, as the original's CallResult
pub struct Rig; impl Rig { pub fn check(&mut self, name: &str, cases: usize, entry: u32, gen: &Generator, port: impl FnMut(&Setup) -> Effects) -> Result<RigReport> }
pub struct RigReport { pub passed: usize, pub paths: usize, pub hangs: usize }   // paths: distinct instruction counts the original took
```
One generator serves the translation service's judge (T33) and the engine port's rig test, so a builder writes it once. A porting task's `port` closure loads the `Setup` into its typed state, runs the engine function and reads back the addresses it owns, as `sf-conform/src/routine.rs`' `check` does.
## Tests
- [ ] `Rig::check` compares returned registers, memory and custom-chip writes in order; a port returning a wrong `d0` fails, and one writing BLTSIZE out of order fails.
- [ ] A routine hang is a bounded count, not a silent pass.
- [ ] `RigReport::paths` counts the original's distinct paths; a generator that never takes a branch reports fewer.
- [ ] Two `PORTED` entries for one address fail; each entry's hunk and offset equal a `kind = "func"` entry of the same name in `re/symbols.toml`, which is committed and needs no disk. A check against the listing, which needs the disks, is optional.
- [ ] A hand port of `h0+$774` passes, as the spike's positive check did.
## Done when
- [ ] `engine/conform/README.md` names the rig, generators and port register, and points porting tasks at `toolkit/docs/porting-routines.md`.
- [ ] Purity test covers everything added to `fd-core`.
## Shared files
`fd-core/src/ported.rs` is created here; every porting task appends to it.
