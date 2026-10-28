# T05 — amiga-oracle

**Phase:** 2 · **Depends on:** T02, T03, T04 · **Weight:** heavy

## Goal

Move the reference emulator harness (vAmiga with its patches, the shim, the Rust wrapper, traces,
the routine runner) into `amiga-oracle`, taking any list of disks and able to power on with none, and
give the toolkit its game-free tests: Kickstart 1.3 booted with no disk twice gives identical frames,
and a snapshot restored mid-boot continues identically. Superfrog's four disks, their names and its
environment variables stay in `sf-oracle`. Game two's first task boots on this harness.

## Design sections this implements

DESIGN §2.2 row `amiga-oracle`, §2.5, §2.10, §6.

## Files

- New: `toolkit/crates/amiga-oracle/` (`git mv` of `src/{amiga,ffi,routine,trace}.rs`, `build.rs`,
  `shim/`, `patches/`, `VAMIGA_COMMIT`; new `src/config.rs` generic half), `toolkit/tools/setup/oracle.sh`,
  its tests (game-free boot, snapshot, the ~9 game-free Moira tests from `sf-oracle/tests/routine.rs`).
- `engine/crates/sf-oracle/` (keeps `config.rs`'s `locate`, the disk mapping, `lib.rs` re-exports,
  `tests/{determinism,blits,drives,exe_map_claims,routine}.rs` minus the moved Moira tests).
- `tools/setup/oracle.sh` (wrapper), `tools/test_oracle_sh.py`, `sf-conform/src/sync.rs` and
  `trace` callers (cache key text), `docs/architecture/verification.md` `owns`, moved-path table,
  purity lists (`amiga-oracle` is test tooling, not pure).

## Interface

```rust
pub struct OracleConfig {
    pub kickstart: PathBuf,
    pub disks: Vec<PathBuf>,       // or bytes; sf-oracle fills it from DiskSet
    pub boot: Option<usize>,       // None: df0 empty at power-on
    pub max_frames: u32,
    pub cache: PathBuf,            // trace cache root; Superfrog passes ~/.cache/superfrog-re/traces
    // …today's other generic fields
}
impl Amiga {
    pub fn power_on(cfg: &OracleConfig) -> Result<Amiga, OracleError>;
    pub fn insert_disk(&mut self, drive: usize, disk: usize);
    pub fn eject_disk(&mut self, drive: usize);
}
// The trace key's boot label is the caller's string; Superfrog passes "D1" so keys stay byte-equal.
pub fn kickstart_path() -> Result<PathBuf, OracleError>; // AMIGA_KICKSTART, else ~/amiga-roms/kick1.3.rom; fails listing where it looked
```

`Cpu68k::load_exe` reads the file's blocks through `amiga_disk::hunk` (T04) and keeps its own
placement, relocation and the game loader's stopping rules (end at a 0x3ED–0x3F1 block, read on past
the file's end); `HunkError` maps from the reader's error with today's variants and messages.

`build.rs` finds the vAmiga build at `$AMIGA_TOOLKIT_CACHE` or `~/.cache/amiga-toolkit/vamiga/<commit>/<patch hash>`;
`oracle.sh` builds there. Patch comments and C symbols renamed in one commit (DESIGN §2.5).

## Tests

- [ ] Game-free: Kickstart booted with no disk to the insert-disk screen (frame count the worker
      measures and records), twice, identical `Rgb12Frame`s and memory hashes.
- [ ] Game-free: snapshot at mid-boot, restore, step N frames: identical to the uninterrupted run.
- [ ] Missing Kickstart: the error names every path tried (test with a scratch `AMIGA_KICKSTART`).
- [ ] The moved Moira tests pass in the toolkit with no ROM and no disk, `load_exe`'s included
      (same `HunkMap` and memory as the base on their synthetic files).
- [ ] Superfrog's trace and sync cache keys are byte-equal to the base's for a sample of cases
      (compute both, compare strings), apart from the one `BUILD` change the rebuild causes.
- [ ] `tools/test_oracle_sh.py` passes against the moved script; `tools/setup/oracle.sh` still builds.
- [ ] `sf-oracle`'s determinism, blits, drives, exe_map_claims tests pass.

## Done when

- [ ] `amiga-oracle` is in the toolkit with no `sf-*` dependency and no game name; the game-free tests
      pass from `toolkit/`.
- [ ] `tools/setup/oracle.sh` builds the renamed vAmiga; Superfrog's oracle tests and DESIGN §4 runs
      green (record the first run's extra time from the cache refill in FINDINGS).
- [ ] Test block green.
