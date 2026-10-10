# T05 — oracle-harness

**Phase:** 1 · **Depends on:** T00, T01 · **Weight:** heavy

## Goal

The permanent full-system reference: the original game running in the vAmiga core, headless, under
Rust control, one PAL frame at a time, with the same `FrameInput` the engine takes, recording memory,
picture and Paula writes per frame into a cache outside git. Everything that claims frame-exactness
is judged by this harness, so it must be deterministic and boring.

## Design sections this implements

DESIGN §2.1, §3.2 (`sf-oracle`), §3.5 (tool caches), §5, §5.2.

## Files

- `tools/setup/oracle.sh` — fetches vAmiga at the commit pinned in `engine/crates/sf-oracle/VAMIGA_COMMIT`,
  applies `engine/crates/sf-oracle/patches/*.patch` (T00's findings), builds the core and shim into
  `~/.cache/superfrog-re/vamiga/<commit>/` under a lock so parallel worktrees build once. No-op when present.
- `engine/crates/sf-oracle/shim/{shim.h,shim.cpp}` — C API over the core.
- `engine/crates/sf-oracle/{build.rs,src/lib.rs,src/amiga.rs,src/trace.rs}`
- `engine/crates/sf-core/src/input.rs` — `FrameInput` (the type both sides take; pure).
- `engine/crates/sf-paula/src/write.rs` — `PaulaWrite` (the chip's own event type; pure, so `sf-core`, `sf-paula` and `sf-oracle` share it).
- `engine/crates/sf-oracle/tests/determinism.rs`

## Interface

```rust
// sf-core::input
pub struct FrameInput { pub joy: Joy /* up,down,left,right,fire bits */, pub keys: KeySet /* the original's key codes held */ }

// sf-oracle
pub struct OracleConfig { pub kickstart: PathBuf, pub disks: [PathBuf; 4], pub max_frames: u32 }
pub struct Amiga;   // owns one emulator instance
impl Amiga {
    pub fn power_on(cfg: &OracleConfig) -> Result<Amiga, OracleError>;
    pub fn step(&mut self, input: FrameInput) -> FrameCapture;   // exactly one PAL frame
    pub fn memory(&self) -> AmigaMemory;                          // chip + slow RAM copy
    pub fn snapshot(&self) -> Snapshot; pub fn restore(&mut self, s: &Snapshot);
    pub fn insert_disk(&mut self, drive: u8, slot: DiskSlot);    // for disk-swap prompts
    pub fn frame(&self) -> u32;
}
pub struct FrameCapture { pub picture: Rgb12Frame /* display window only */, pub paula: Vec<PaulaWrite>, pub audio: Vec<i16> /* per-channel DAC taps if T00 found them */ }
// sf-paula::write
pub struct PaulaWrite { pub reg: u16, pub value: u16, pub hpos: u16, pub vpos: u16 }
pub fn cached_trace(cfg: &OracleConfig, start: &SyncPoint, script: &InputScript, frames: u32) -> Trace;  // memoised in ~/.cache/superfrog-re/traces/
```

`hpos`/`vpos` are recorded for diagnosis only. Conformance compares the ordered `(reg, value)` list,
because the engine is not cycle-timed and cannot know the beam position of a write (DESIGN §2.1).

The game swaps disks; the harness answers disk-change requests the way a player would, and the
engine's flow (T21) models the same prompts. How the original detects a swap is recorded in T07.

## Tests

- [ ] power-on to the title twice: identical captures on every frame (picture, Paula, memory hash).
- [ ] snapshot at frame 1 000, run 500, restore, run 500 again: identical.
- [ ] input reaches the game: holding fire / a key on the title changes state as the original does
      (observed in memory), within the frame T00 recorded.
- [ ] `max_frames` is enforced; a run past it errors instead of continuing.
- [ ] trace cache: second call is a hit; changing script, disks or commit misses; a corrupt cache file is rebuilt.
- [ ] the harness opens disks and ROM read-only (checked by file mode after run).

## Done when

- [ ] `tools/setup/oracle.sh` builds from nothing in a fresh worktree and is a no-op the second time.
- [ ] The tests above are in the test block and take under 60 s together.
- [ ] `sf-app` cannot depend on `sf-oracle` (purity test from T01 covers it, still green).

## Outside actions

- Install brew build deps — `worker`
- Fetch and build vAmiga at the pinned commit — `worker`
