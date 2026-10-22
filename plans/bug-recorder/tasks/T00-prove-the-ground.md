# T00 — prove-the-ground

**Phase:** 0 · **Depends on:** — · **Weight:** heavy

## Goal

Answer, on this machine against the oracle, the two questions the plan rests on, with throwaway code
deleted afterwards. First, which way of starting a level with the dialog's values gives an engine
state equal to the original's (DESIGN D2), including through a later screen transition. Second,
whether per-step inputs recorded from the unpaced engine stay in step on the lagging original when
each input is fed at the frame the original's matching pass reads it (DESIGN §2.9), across a level's
end, the tally, the slot machine and the next level's load.

## Design sections this implements

DESIGN §2.1 (what the start sets), §2.9, §7 D2.

## Files

Scratch only: `engine/crates/sf-conform/examples/spike_*.rs` (deleted before the task ends, kept in
the task's commit history). Nothing else changes. The answers go in `FINDINGS.md` and as a dated
amendment blockquote at the top of T01, T02, T03 and, if the fallback is needed, T05.

## Questions and what each answer means

1. **Start.** For W1-1, W1-2, W4-3 and WB-1, in normal, easy and hard, build the engine start both
   ways: (a) `Start::Level` + pokes; (b) power-on, options screen set (lives, difficulty), level
   code typed (`Game::type_code`), then pokes at frame 0. Original: power-on, the same options set
   on the options screen, the code typed, the same pokes, `rng_state` written to the engine's. Compare
   at frame 0 (as `level_entry.rs` does) and after 2 000 frames of an existing script.
   - (b) equal and (a) not: (b) is the start; `seed` is the power-on seed. Expected.
   - both equal: prefer (a) only if it is also equal after a level-end transition; else (b).
   - neither: name the variables; the plan stops and the owner is told.
2. **Retiming.** Take the scripts of `chain-w1-1-to-w1-2`, `tally-slot-w1-1` + `slot-gamble`, and
   one level with long lag (W4-3's completion). Play each on the engine unpaced from the chosen start
   to get per-step inputs, as the app would record them. Retime by pass and feed the original; run
   the faithful checks over the whole stretch, W1-1 through the slot machine into W1-2 included.
   - green: §2.9's rule stands.
   - drifts: measure where; try the per-leg fallback (re-sync from the original every N passes) and
     record the N that holds; T05 is amended to the fallback.
3. Record the frog's cell formula check: `((frog_x + 0x17)/16, (frog_y - 0x10)/16)` names the
   tile the owner would call the frog's on three hand-picked spots, by comparing with
   `tools/export_annotated.py`'s grid.

## Tests

None committed (throwaway). The evidence is the runs, quoted in FINDINGS rows with their numbers.

## Done when

- [ ] FINDINGS has dated rows answering questions 1–3 with the measured numbers.
- [ ] T01–T03 (and T05 if needed) carry an amendment blockquote stating the chosen start and timing rule.
- [ ] No spike code remains in the tree; the test command is green.
