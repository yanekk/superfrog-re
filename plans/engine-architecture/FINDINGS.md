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
| 2026-10-23 | 📌 | T06: the blitter is modelled three times (`Blit::run` in `front/blit.rs`, ascending-only `blit` in `level/objects.rs`, `blit` in `level/scroll.rs`), and `sprite_attach` in `level/bobs.rs` repeats `front/sprite.rs` `attach` (h0+$1b208). A rewrite can merge each. |
| 2026-10-23 | 📌 | T06: `DisplayData::new` (`render/mod.rs`) builds every subsystem's disk data (`rom`, `sound`, `front`, `story`, `levels`), so those lines count as display-hardware; T18 may weigh a move. `render/panel.rs` fetches every bitplane line, not only the panel's. |
| 2026-10-23 | 📌 | T05: the keyboard poll is ported twice (`kbd_poll` in `level/clock.rs` and `front/vbl.rs`, state in `State::kbd_flag`/`kbd_acked` and `front::Keyboard`); a rewrite can merge them. No check moves the mouse (`JOY0DAT` always 0). |
| 2026-10-23 | 📌 | `tools/architecture.py totals` counts 111k code lines today (verification 47.8k, app 10.4k), against DESIGN §1's ~75k on `pir/rust-engine`. The T01 ownership table covered every tracked file; nothing was reassigned. |
| 2026-10-22 | 📌 | The stored `.pir/settings.json` test line runs only the Python suites (34 tests, ~36 s on a fresh worktree); the Rust suites are not in it. This plan changes no Rust, so the drift check joins the Python suites. |
| 2026-10-22 | 📌 | The Amiga imitation is woven through `sf-core`, not layered: byte-exact `State` at original addresses, `spend` cycle charging, cycle-run copper, and the story's own bus simulator (`front/story/sim.rs`, `cpu.rs`, `mem.rs`). Hence mechanism chapters. |
