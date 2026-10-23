# Glossary

The words every chapter uses for the engine's imitation mechanisms, so that chapters written in
parallel name the same thing the same way. Each entry is a definition, says where the code lives
and which chapter documents it; the chapters carry the detail. Use these terms as written; if a
chapter needs one that is not here, it defines it in its own text and says so in its task's
findings.

`h0+$x` is an address in the game executable's hunk 0 as loaded (see the loaded hunk image).

## The kinds

The four kinds a chapter's `mix` divides its code into (plans/engine-architecture DESIGN §2.1).

- **`amiga`, pretend-Amiga.** Code that imitates the Amiga's hardware or its timing: memory at
  original addresses, cycle charging, beam position, interrupts, copper, blitter, Denise, Paula,
  the CIA keyboard, the disk loader's behaviour.
- **`game`, game logic.** Superfrog's own rules: physics, enemies, scoring, flow. It would exist in
  any reimplementation of the game.
- **`data`, copied-in data.** Tables, memory images and assets read from the original's executable
  or disks and used as they are.
- **`shell`, modern shell.** Neither: the macOS app, file handling, test tooling. Amiga imitation
  inside it (PAL pacing, CRT filters) still counts as `amiga`.

## The reuse scale

Judged against another Amiga game by any developer (DESIGN §2.2).

- **`as-is`.** Carries over with no or trivial change.
- **`small-changes`.** Carries over with a few named, local changes: addresses, a table, a config.
- **`big-changes`.** The approach carries; much of the code is rewritten.
- **`superfrog-only`.** Specific to this game; only the method carries.

## The mechanisms

- **Byte-exact state at original addresses.** Every original variable the engine models is kept
  as the original's bytes at the original's address, read and written big-endian by address, so
  the port keeps the 68000 code's overlaid words and pointers into its own tables. `State` in
  `engine/crates/sf-core/src/state.rs`. Chapter: [memory-model](memory-model.md).
- **The layout table.** `LAYOUT` in `engine/crates/sf-core/src/layout.rs`: each modelled variable with its
  address and width, in address order. Adding a variable is one line of the `layout!` list; a
  write to an address no entry covers panics. Chapter: [memory-model](memory-model.md).
- **Transplant and probe.** Copying the original's memory into the engine (`State::transplant`)
  and reading the engine's values back out by original address (`State::probe`), both driven by
  the layout table. It is how `sf-conform` starts the engine mid-game and compares it with the
  original. Chapters: [memory-model](memory-model.md), [verification](verification.md).
- **The loaded hunk image (`Rom`).** The game executable `d1_c03` as its loader leaves it in
  memory, hunk 0 relocated to `H0`, read from the player's disks; the ported code reads the game's
  own tables (frame lists, jump tables) out of it by address. `Rom` in `engine/crates/sf-core/src/mem.rs`.
  Chapter: [copied-in-data](copied-in-data.md).
- **Boot-built tables.** Tables the original's boot builds once in memory (the map's and the
  display ring's row pointers, the frog's frame records, the tile pointers), rebuilt by the engine
  the same way and placed at the same addresses.
  `boot_tables` in `engine/crates/sf-core/src/mem.rs`. Chapter: [copied-in-data](copied-in-data.md).
- **Cycle charging (`spend`).** A ported routine says what its 68000 work costs
  (`LevelCtx::spend`, `LevelCtx::blit_words` for a blit the CPU waits on), so its writes land
  where in the frame the original's do. In `engine/crates/sf-core/src/level/mod.rs` and
  `engine/crates/sf-core/src/level/beam.rs`. Chapter: [cpu-time-and-beam](cpu-time-and-beam.md).
- **Beam position.** Where the display's raster is (line and horizontal position) when a write
  happens. The level's `Clock` turns charged cycles into lines, giving the CPU fewer cycles on
  lines that fetch bitplanes (`engine/crates/sf-core/src/level/beam.rs`); `Timeline` replays work done at once
  across frames on the beam (`engine/crates/sf-core/src/level/timeline.rs`). Chapter:
  [cpu-time-and-beam](cpu-time-and-beam.md).
- **The vertical-blank and copper interrupts.** The original's per-frame interrupt
  (`vbl_handler`, h0+$11bf8) and the copper interrupt the scroller requests from it, imitated in
  their order each frame: `engine/crates/sf-core/src/level/frame.rs` in a level, `engine/crates/sf-core/src/front/vbl.rs`
  outside one. Chapter: [frame-interrupts-input](frame-interrupts-input.md).
- **The 25 Hz main-loop pass.** The original's main loop (`level_frame_loop`, h0+$aa6) runs every
  second vertical blank; the engine runs the same pass on the same frames.
  `engine/crates/sf-core/src/level/frame.rs`, its routines in `engine/crates/sf-core/src/level/pass.rs`. Chapters:
  [frame-interrupts-input](frame-interrupts-input.md), [level](level.md).
- **The copper run.** The level's copper list run cycle by cycle on the original's bus model, so
  each register write lands on the pixel it does on the Amiga. `engine/crates/sf-core/src/render/copper.rs`.
  Chapter: [display-hardware](display-hardware.md).
- **Bitplane fetch.** The renderer draws each line as Denise would, fetching its bitplanes from
  the panel or the shown display buffer, which are kept as state the way the original's blits left
  them. `engine/crates/sf-core/src/render/playfield.rs`, `engine/crates/sf-core/src/render/panel.rs`. Chapter:
  [display-hardware](display-hardware.md).
- **Hardware sprites.** The Amiga's eight sprites, put on screen by rewriting the sprite pointer
  moves of a copper list (`engine/crates/sf-core/src/front/sprite.rs`) and drawn over the play field in the
  original's priority order (`engine/crates/sf-core/src/render/objects.rs`). Chapter:
  [display-hardware](display-hardware.md).
- **The blitter.** The Amiga's block-copy chip: what one blit leaves in memory
  (`engine/crates/sf-core/src/front/blit.rs`, the bob set-up in `engine/crates/sf-core/src/level/bobs.rs`) and, in a level,
  what it costs on the beam. Chapter: [display-hardware](display-hardware.md).
- **Paula writes.** Register writes to the Amiga's sound chip, each at its beam position
  (`PaulaWrite` in `engine/crates/sf-paula/src/write.rs`), which `sf-paula` turns into the four
  channels' output and mixes. The game's driver that makes them is in `engine/crates/sf-core/src/audio/`.
  Chapters: [sound-chip](sound-chip.md), [sound-driver](sound-driver.md).
- **The CIA keyboard.** The keyboard read as the original reads it: the byte in CIA-A's serial
  register, polled every second vertical blank and acknowledged by hand (`kbd_poll` in
  `engine/crates/sf-core/src/level/clock.rs`). Chapter: [frame-interrupts-input](frame-interrupts-input.md).
- **Disk prompts and swaps.** The original asks for a disk it finds in no drive
  (`disk_request`, h0+$ade4); the engine reports it as `DiskPrompt` in `FrameOut`
  (`engine/crates/sf-core/src/game.rs`) and the app answers it. Disks are read by `engine/crates/sf-disk/`.
  Chapters: [game-flow](game-flow.md), [disks-and-assets](disks-and-assets.md), [app](app.md).
- **PAL pacing.** The app steps the game once per PAL frame of a clock (the audio device's
  samples played, or the wall clock), never a fraction of one: `Pacer` in
  `engine/crates/sf-app/src/pacing.rs`. Chapter: [app](app.md). The `Pacing` the conformance runs
  feed the engine is a different thing (`engine/crates/sf-core/src/feed.rs`, chapter
  [cpu-time-and-beam](cpu-time-and-beam.md)).
- **The story's bus simulator.** The story intro is a program of its own: the engine keeps the
  whole machine's RAM for it (`engine/crates/sf-core/src/front/story/mem.rs`) and times its draws by running
  the CPU's and the blitter's bus cycles as the reference emulator does
  (`engine/crates/sf-core/src/front/story/cpu.rs`, `engine/crates/sf-core/src/front/story/sim.rs`).
  Chapter: [story-and-ending](story-and-ending.md).
