# T187 — chain-level-code

**Split from:** T183 (its "Starting from a level code on the options screen enters that level" and
the `chain-level-code` test)

**Phase:** 2 · **Depends on:** T183, T185; blocks T21 · **Weight:** small

## Goal

`chain-level-code`: from power-on, OPTIONS, a W3-1 code typed into LEVEL CODE, the options left,
START GAME: `game_start_at` (ported by T183 in `front/chain.rs`, with `password::check`) jumps to
`top_loop`'s world 3 entry (h0+$2e8), W3's loads (disk 2), its loading picture, the map, W3-1.
Faithful and alone; in `tests/chain.rs`.

Known (T183): the runner serves disks only for `SyncPoint::EndSequence` (`lib.rs`: `disks`); a
power-on case reaching W3 needs `sync::Disks::serve` on every case (it acts only when the original's
wanted disk, h0+$af54, changes). The code's digits come from `password_table` (entry `4*2+0 = 8`);
`sync.rs`'s `drive_to` shows the typing (raw keys 01..0A, 16 frames apart from `TYPE_FROM`).
`world_load_w3` (h0+$9976) has world 1's shape; its effects are `L1FX-2` (`BankId::Worlds36`).

## Done when

- [ ] `chain-level-code` green, faithful and alone; no `unported`.
