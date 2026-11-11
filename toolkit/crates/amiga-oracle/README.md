# amiga-oracle

The reference the faithful bar is judged against. `Amiga` runs the original headless on vAmiga, one PAL frame per `step`, with the same `FrameInput` an engine takes, and returns what the frame drew and wrote to Paula. `Cpu68k` runs one routine of the original alone on Moira (vAmiga's 68000 core) over flat memory with no chipset. Test tooling only: a shipped game must never link it (each game's purity test enforces it). A game passes in its own disks and cache roots.

## Setup

- Run `toolkit/tools/setup/oracle.sh` once per machine. It fetches vAmiga at `VAMIGA_COMMIT`, applies `patches/*.patch` and builds into `~/.cache/amiga-toolkit/vamiga/<commit>/<patch-set hash>/` (`AMIGA_TOOLKIT_CACHE` replaces `~/.cache/amiga-toolkit`). Without it `build.rs` panics naming the missing path. Both `Amiga` and `Cpu68k` need this build.
- Kickstart: `AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom` (1.3, 34.5). A set variable is the only place looked. `Cpu68k` needs no ROM.
- `BUILD` (`<commit>-<patch hash>-<settings>`) is in every trace and sync-cache key: a changed patch never reuses an old snapshot.

## Start here

Boot and step (from `tests/boot.rs`):

```rust
let rom = kickstart_path()?;
let cfg = OracleConfig::new(rom, disks /* Vec<Vec<u8>>, raw ADFs */, 2_000);
let mut a = Amiga::power_on(&cfg)?;
let cap = a.step(FrameInput::NONE)?;   // cap.picture, cap.paula, cap.taps()
let ram = a.memory();                  // AmigaMemory { chip, slow }
```

Run a routine alone (from `tests/routine.rs`):

```rust
let mut cpu = Cpu68k::new(a.memory());          // or AmigaMemory with zeroed chip/slow
let mut r = Regs::default(); r.a[7] = 0x30_0000; // sr = 0x2700
let got = cpu.call(entry, r, 100_000)?;          // got.regs, got.writes, got.instr, got.cycles
let after = got.bytes_after(&cpu, addr, len);    // memory itself is put back
```

Frame caps: every run is bounded. `step` returns `OracleError::FrameCap` at `max_frames` (`DEFAULT_MAX_FRAMES` = 20 000); a restored snapshot keeps the cap of the instance that restores it. `Cpu68k::call` ends with `CallError::Timeout` after `max_instr`.

## API by job

Set-up (`config.rs`)
- `OracleConfig::new(kickstart: PathBuf, disks: Vec<Vec<u8>>, max_frames: u32)`: one drive, boots disk 0 (none if empty). Fields `kickstart`, `disks`, `boot`, `boot_label`, `max_frames`, `drives`, `cache` are public.
- `with_drives(self, drives: u8)` (1..=4, asserts); `with_boot(self, boot: Option<usize>, label: &str)`: `None` powers on with df0 empty.
- `kickstart_path() -> Result<PathBuf, OracleError>`, `default_cache() -> PathBuf`, `read_only(path: &Path) -> Result<Vec<u8>, OracleError>`.
- `OracleError`: `Setup`, `FrameCap { max_frames }`, `Emulator`, `NoDrive(u8)`, `Cache`.

The machine, `Amiga` (`amiga.rs`): an A500, OCS, 512K chip + 512K slow, PAL, deterministic.
- `power_on(cfg: &OracleConfig) -> Result<Amiga, OracleError>`; `step(&mut self, input: FrameInput) -> Result<FrameCapture, OracleError>`; `frame()`, `max_frames()`.
- Picture: `FrameCapture::picture` is an `Rgb12Frame` (`left`, `top`, `width`, `height`, `pixels: Vec<u16>` as `0x0RGB`, `hires_detail`; `pixel(x, y)`), lores, inside the DIW the registers hold at frame end. `picture_in(&self, diwstrt: u16, diwstop: u16) -> Rgb12Frame` re-cuts the last frame through another window, for a game that reloads DIW mid-frame.
- Sound: `FrameCapture::paula: Vec<PaulaWrite>`; `FrameCapture::taps()` yields `AudioTap { channel, vpos, hpos, value }` (DAC changes); `mixed_audio() -> Vec<[f32; 2]>` at `MIX_RATE`, fading in over the first 10 000 samples.
- Memory: `memory() -> AmigaMemory`, `memory_hash() -> u64`, `hash_memory(&AmigaMemory) -> u64`, `peek(addr: u32, len: usize) -> Vec<u8>` (0 outside RAM). `SLOW_BASE` is `0xC00000`.
- Writing RAM between frames: `poke(addr, bytes)` drops bytes outside RAM; `write_memory(addr, bytes)` panics on them. Neither is player input.
- CPU: `cpu() -> CpuRegs` (`d`, `a`, `pc`, `sr`, `usp`, `ssp`; `ipl()`, `supervisor()`).
- State: `snapshot() -> Snapshot` (`frame`, `input`, `bytes`), `restore(&mut self, s: &Snapshot)` (panics on a foreign snapshot).
- Disks: `insert_disk(drive: usize, disk: usize)` and `eject_disk(drive: usize)` panic on an unconnected drive; `df0() -> Option<usize>`.
- Timing evidence from the last step: `frame_lines()` (313 or 312), `frames_run()` (normally 1; a blit can carry a step into the next frame), `cpu_writes() -> Vec<CpuWrite>`, `irqs() -> Vec<Irq>`, `blit_events() -> Vec<BlitEvent>` (`BlitEventKind::{Begin, Free, End}`), `blit_starts() -> Vec<(u16, u16, u16)>`.
- Watches: `watch_write(Option<(u16, u16)>)` then `watched_memory() -> Option<AmigaMemory>` (RAM at the CPU's first write of a value to a register); `watch_pcs(&[u32])` then `pc_hits() -> &[(u32, u32)]` (execution counts, run unchanged).
- Threads: machines may be set up and stepped on several threads at once, each machine on one thread at a time (core patch 0007, `tests/machines_apart.rs`; `das_table_whole()` is that test's look at the core's shared table).
- Profiling: `set_profile(bool)`, `set_dma_profile()`, `line_samples() -> Vec<LineSample>`. Line sampling leaves the run unchanged, switched between any steps (`tests/line_samples.rs`); DMA profiling draws over the picture: never use it in a comparison.

Cached runs (`trace.rs`)
- `SyncPoint::{PowerOn, Snapshot(Snapshot)}`; `InputScript::new(Vec<(u32, FrameInput)>)`, `idle()`, `input_at(frame)`, `canonical()`.
- `cached_trace(cfg, start, script, frames) -> Result<Trace, OracleError>`, stored under `cfg.cache`; `cached_trace_in(dir, ...)` also returns `CacheOutcome::{Hit, Miss, Rebuilt}`. `Trace { key, start_memory, frames: Vec<TraceFrame> }`, `memories()`; `TraceFrame { capture, memory_hash, memory_delta }`. `trace_key(...)` is the SHA-1.

The routine runner, `Cpu68k` (`routine.rs`): 16 MB flat memory (`MEM_SIZE`), chip RAM at 0 and slow at `$C00000`.
- `new(mem: AmigaMemory)`, `memory()`, `memory_mut()`, `read(addr, len)`, `write(addr, bytes)` (panic past 16 MB).
- `call(&mut self, entry: u32, regs: Regs, max_instr: u64) -> Result<CallResult, CallError>`: pushes `RETURN_SENTINEL` (`0xF00000`) and runs to it. Memory is undone on return whatever the outcome; `apply(&result, keep: impl Fn(u32) -> bool)` puts writes back for chained calls.
- `CallResult { regs, writes: Vec<(u32, u8, u8)>, instr, cycles, hw_writes }`; `cycles` is Moira's clock with no bus contention. `CallError::{Timeout, CustomChipAccess { pc, addr }, IllegalInstruction { pc }, Exception { pc, vector }, Halted { pc }}`.
- Hardware mode, `set_hardware(true)`: custom-chip writes go to `hw_writes` instead of failing with `CustomChipAccess`, BLTSIZE runs an area-mode blit (no line mode or fill), VHPOSR steps on every read, other hardware reads 0 unless set by `set_hardware_read(addr, value)`. No DMA or interrupts: a routine waiting on them times out.
- `load_exe(&mut self, exe: &[u8], at: u32) -> Result<HunkMap, HunkError>` loads a hunk executable as a trackloader does; `HunkMap::addr(hunk, offset)` turns `hN+$off` into an address.
- Coverage: `set_trace(true)` then `last_trace() -> CallTrace { branches: Vec<BranchWays>, reads }`; `conditional_branches(entry) -> Vec<u32>` is a static walk; `insn_len(addr)`.
- `Gen::new(seed)`, `next`, `below`, `pick`, `one_in`: the shared xorshift for routine test states. Never change its sequence.

## Where the details are

`src/amiga.rs`, `src/routine.rs`, `src/trace.rs`, `src/config.rs`; `shim/` and `patches/` (C++ side). Examples: `tests/boot.rs`, `tests/routine.rs`, `tests/routine_branches.rs`, `tests/exec_counts.rs`, `tests/picture_in.rs`. `toolkit/docs/architecture/verification.md` (watch and trace costs), `toolkit/docs/porting-routines.md` (proving a port), `toolkit/PLAYBOOK.md`. Cached sync-point snapshots: `amiga-conform::sync_cache`.
