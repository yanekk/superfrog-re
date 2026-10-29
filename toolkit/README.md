# The Amiga toolkit

The parts of an Amiga game's engine that carry to another game: the machine's chips and timing, its
disk formats, recorded input, a desktop app's game-free parts, and the test tooling that judges an
engine against the original game running on an emulator. No game's code, data or name is in this
tree; a game depends on these crates by path (`../../../toolkit/crates/<name>` from a crate in the
game's own workspace) and keeps everything of its own.

## What is in it

Pure crates read no clock, file, network or random source, so a whole run of behaviour is checkable
in milliseconds:

| Crate | What it holds |
|---|---|
| `amiga-clock` | The beam: PAL frame and line lengths, one `BeamPos`, a beam `Clock` configured by a display's `Timing`, the blitter's bus timing, a `Timeline` that replays work done at once on the beam |
| `amiga-chips` | The blitter's memory effect (`Blit`), the copper run for one frame (`copper`), the A500's RAM (`AmigaMemory`), the picture a frame shows (`Picture`), and `display::draw`, which draws a lores frame from chip memory and the copper's writes |
| `amiga-paula` | Paula: register writes to the four channels' DAC values, mixed and resampled to the host rate |
| `amiga-input` | One frame's joystick and keys (`FrameInput`), their text form for scripts and recordings, the keyboard as CIA-A presents it |
| `amiga-disk` | A zip reader and stored-entry writer, SHA-1, the ADF size, and readers for hunk executables, IFF ILBM and ANIM, plane-major bitmaps and the OCS palette |
| `amiga-record` | The game-free half of a bug recorder: a recording's inputs, seed, build and note, its text form and its zip; the game supplies its own header through `GameRecording` |

Shell crates touch devices, files or the emulator:

| Crate | What it holds |
|---|---|
| `amiga-oracle` | vAmiga, the reference emulator, run headless one frame at a time with the same `FrameInput` an engine takes; any list of disks, or none; cached traces; one routine of the original alone on the Moira CPU |
| `amiga-conform` | The game-free half of a conformance harness: picture and Paula comparisons against the oracle, the faithful bar's `Tolerance`, sound events, diff pictures, the script token lexer |
| `amiga-app` | A desktop app's game-free parts: audio out and the sample clock that paces the game, gamepads, the Amiga key table, the picture on the GPU with CRT filters, run capture, whole-file writes, the single-copy lock |
| `amiga-e2e` | End-to-end tests of a game's built app, driven by script or through the real window system; the game describes its app in an `AppSpec` |

And two that are not for a game to depend on:

| Crate | What it is |
|---|---|
| `toolkit-checks` | The purity test (the pure/shell boundary, each pure crate's dependencies on an allowlist, no game crate in any toolkit crate's tree) and the name check (no game's name in any tracked file here, bar a `learned on:` provenance line) |
| `dry-fit` | One frame of a made-up game from `amiga-chips`, `amiga-clock` and `amiga-paula` alone: a blit into a bitplane, a copper list that sets up the screen and changes colours, the frame drawn to a `Picture`, the clock stepped across the frame and a Paula note. Its tests assert each step and that its dependency tree holds toolkit crates only. `cargo run -p dry-fit` prints what each chip did |

## Testing it

```bash
cd toolkit && cargo test -q --workspace
```

This is a workspace of its own, with its own toolchain pin and lock file, and runs from this folder
alone. The oracle's tests need vAmiga built (`tools/setup/oracle.sh`, under `~/.cache/amiga-toolkit/`,
or `AMIGA_TOOLKIT_CACHE`) and a Kickstart 1.3 ROM (`AMIGA_KICKSTART`, else
`~/amiga-roms/kick1.3.rom`); without the ROM they fail and say where they looked. The oracle's own
tests boot with no disk in the drive, so they need no game.

## Rules

- Nothing from a game's disks or ROM enters this tree: tests build their data from pieces.
- A constant measured on a particular game keeps its value and says so in a line starting
  `learned on:`, the one place a game may be named.
- A pure crate that needs a file, a clock or a dependency off the allowlist is wrong: move the code
  to a shell crate, never relax the test.
- The frame is PAL's 313 lines throughout; parameterising it is the next game's first step.
