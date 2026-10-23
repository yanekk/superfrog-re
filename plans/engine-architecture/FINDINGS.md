# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**What goes here:** what the next session would otherwise rediscover · what you noticed and left
alone under the scope rule (an `owns` path that fits another chapter better, DESIGN §2.7) ·
answers from the user, dated.

**Whoever appends, compacts** (over 60 rows or 15 KB). Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-23 | 📌 | T11: `LevelCtx` in `level/mod.rs` holds about 800 lines of beam machinery (frame ends mid-pass, `sfx_play`'s raster wait, copper interrupt masking), counted as level's `amiga` share. It is cpu-time-and-beam's mechanism; T18 may weigh a move. |
| 2026-10-23 | 📌 | T11: `panel_bitmap`, the display buffers and the scroller's blit queue are presentation pieces (`layout::presentation_piece`), left out of check 1. The panel has no routine test; only the picture check holds it. |
| 2026-10-23 | 📌 | T11: docs/07 and the project `CLAUDE.md` state collision as `flag & 0b11000 == 0b11000`; the engine's `collision.rs` uses a marker ladder instead (`$3a`+ ground, `$3e`/`$3f` landing, `$3d` step-up and walls). Docs left alone. |
| 2026-10-23 | 📌 | T07: the app's mixer and `channels::engine_levels` never call `Paula::set_frame_lines` (`Pacer::set_frame_lines` only in its test), so every frame mixes as 313 lines though the game keeps stretches of short frames. Unmeasured whether audible. |
| 2026-10-23 | 📌 | T07: `sf-paula`'s ADKCON attach modes and audio interrupts are unchecked against vAmiga; Superfrog's driver writes no ADKCON. Left alone. |
| 2026-10-23 | 📌 | T06: the blitter is modelled three times (`Blit::run` in `front/blit.rs`, ascending-only `blit` in `level/objects.rs`, `blit` in `level/scroll.rs`), and `sprite_attach` in `level/bobs.rs` repeats `front/sprite.rs` `attach` (h0+$1b208). A rewrite can merge each. |
| 2026-10-23 | 📌 | T06: `DisplayData::new` (`render/mod.rs`) builds every subsystem's disk data (`rom`, `sound`, `front`, `story`, `levels`), so those lines count as display-hardware; T18 may weigh a move. `render/panel.rs` fetches every bitplane line, not only the panel's. |
| 2026-10-23 | 📌 | T04: `Pacing` and `PassClock` (`engine/crates/sf-core/src/game.rs`, owned by game-flow) are the beam mechanism's interface; `cpu-time-and-beam` describes them without owning them. Four separate beam models exist: level `Clock`, front `Cx`, `slot/draw.rs`, story `sim.rs`. For T18. |
| 2026-10-23 | 📌 | T04: T213 is ✅ in `plans/rust-engine/PROGRESS.md`, yet the project `CLAUDE.md` and the T04 task doc call it open. The chapter states it as ✅ and names the story `sfx_play` remainder. |
| 2026-10-23 | 📌 | T02 review: `layout.rs`'s `Level1Vector` doc names only `enemies::w3::swing` as writing `$64`; `enemies::w2::mace` does the same through a null link slot. Code comment, left alone. |
| 2026-10-23 | 📌 | T02: `mem.rs` is half copied-in data (`loaded_hunk0`, `boot_tables`, `tile_heights`, `Rom::new`) but owned by `memory-model`, which counts it as `data` 15. No check names the null-pointer reads at 0/4 or the `$64` write; T18 may weigh both. |
| 2026-10-23 | 📌 | T05: the keyboard poll is ported twice (`kbd_poll` in `level/clock.rs` and `front/vbl.rs`, state in `State::kbd_flag`/`kbd_acked` and `front::Keyboard`); a rewrite can merge them. No check moves the mouse (`JOY0DAT` always 0). |
| 2026-10-23 | 📌 | T03: `sf-core` never reads `sf_assets::Tables`. Templates, the marker jump table and the slot tables are decoded there, but the engine reads them by address from relocated hunk 0; only `catalogue` and tests use `Tables`. |
| 2026-10-23 | 📌 | T03: hunk 0 is held four times (`ExeImage`, `Rom::h0`, `FrontData::h0`, `SoundData`) by three relocators (`mem.rs` `loaded`, `front/data.rs` `relocate`, `story/data.rs` `data_hunk`); `sound.rs` repeats `H0`, `H3` and its own `file_table` reader. |
| 2026-10-23 | 📌 | T03: the engine reads original code as data: `Chain::run` steps through `top_loop`, `password::table` reads handlers' `move.l` immediates, `table_entries` scans for `lea`. A native rewrite replaces these before dropping hunk 0. |
| 2026-10-23 | 📌 | `tools/architecture.py totals` counts 111k code lines today (verification 47.8k, app 10.4k), against DESIGN §1's ~75k on `pir/rust-engine`. The T01 ownership table covered every tracked file; nothing was reassigned. |
| 2026-10-22 | 📌 | The stored `.pir/settings.json` test line runs only the Python suites (34 tests, ~36 s on a fresh worktree); the Rust suites are not in it. This plan changes no Rust, so the drift check joins the Python suites. |
| 2026-10-22 | 📌 | The Amiga imitation is woven through `sf-core`, not layered: byte-exact `State` at original addresses, `spend` cycle charging, cycle-run copper, and the story's own bus simulator (`front/story/sim.rs`, `cpu.rs`, `mem.rs`). Hence mechanism chapters. |
