**Split from:** T36 — carries no "Done when" item: the scaffolding T36 left.

# T42 — beam-cleanup

**Phase:** 2 · **Depends on:** T36; blocks T37 · **Weight:** light

Added by T36 (owner, 2026-10-13: "stop and split the rest"). T36 left its working scaffolding
committed; this task turns it into product code and records the design.

## Goal

1. Remove T36's debug hooks: `sf_core::level::DEBUG` and every `eprintln!` it gates (scene.rs LOG
   dump, mod.rs boundary print, render/mod.rs FETCH print); in `sf-conform/src/lib.rs` the
   `T36_STATS`, `T36_STRICT`, `T36_PIC`, `T36_SHOW`, `T36_DUMP`, `T36_MIDVARS`, `T36_NORESYNC`
   blocks.
2. Scratch examples in `engine/crates/sf-conform/examples/` (`beam`, `calib`, `copper`, `fit`,
   `profile`, `reach`, `speed`, `coverage`, `plan`, `items_map`, `dma`, `common/`): keep `profile`,
   `fit`, `coverage`, `plan` and `items_map` (they write item cases: T44–T47) tidied, with a module
   doc each and no `/tmp/t36` paths (the engine-symbol table they read goes to a function); delete
   the rest. `fit` must print `Cost::cycles` match arms, not an array (purity test).
3. Docs: DESIGN §2.1 lag paragraph and docs/08 "Lag" say what `Pacing::Late { writes, line, hpos,
   phase, step }` carries, that conformance anchors the beam at the original's Paula writes and
   blit starts (`Game::anchor_writes`, `Game::anchor_blits`; owner, 2026-10-13) and that normal play
   uses the cost model alone (`level::beam`). README of `engine/conform` "Pacing" section likewise.
4. `sf-oracle`: document `set_profile`, `set_dma_profile` (the DMA debugger's per-slot bus owners,
   reached through the explicit-instantiation access trick in `shim.cpp`), `blit_starts`.

## Done when

- [ ] No `T36_` env read and no `DEBUG` static left; `cargo clippy` clean on the touched crates.
- [ ] `frog-*` cases, `items-run-w1-1`, `items-fruit-w2-2` and `level_entry` still green.
