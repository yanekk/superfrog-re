# T26 — os-answers

**Phase:** 4 · **Depends on:** T01, T22 · **Weight:** heavy

## Goal

Answer every OS call and structure in `re/os-calls.toml` (T01) on the machine, so the program runs from its entry with no Kickstart code: `OpenLibrary`/`CloseLibrary`, the `dos.library` file calls read from the disk images by `amiga-disk`'s OFS reader and finished at once, whatever `system_on`/`system_off` and the title rely on from the OS (its interrupts, its keyboard handler), and the memory structures the program reads.

## Design sections this implements

DESIGN §2.8 (the OS), §2.14.

## Files

- `crates/pcode-machine/src/os/{mod,exec,dos,input}.rs` (new). The library bases and vector tables are made-up addresses in the machine's ROM space; a call through a vector is answered in Rust and returns as `rts` would.

## Interface

```rust
pub struct Disks { pub df0: Vec<u8> /* adf */ }  // disk 2, which holds every file the program opens
impl Machine { pub fn insert(&mut self, d: Disks); }
// an untranslated address inside the OS's space that is not a known vector stops: "os: unknown <lib> LVO <n> at <site>"
```

Every answered call is listed in the crate README with what it returns and why the program needs it, from `docs/15-os-surface.md`.

## Tests

- [ ] Made-up program: `OpenLibrary("dos.library")`, `Open`, `Read`, `Close` on a made-up OFS image return its bytes and a handle; a missing file returns 0 as AmigaDOS does.
- [ ] An unknown LVO stops naming it.
- [ ] Full: `load_file` (`h0+$8ec4`) run on the machine loads `_NAPIS` and the result equals the oracle's memory after the same load.

## Done when

- [ ] Every row of `re/os-calls.toml` is answered or explicitly a stop with its reason; the README table matches it.
- [ ] Full: power-on to the title's first frame runs with no stop.
