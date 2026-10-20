# T19 — slot-and-bonus

**Phase:** 2 · **Depends on:** T15 · **Weight:** medium

## Goal

The slot machine played between levels (reels, stopping, payouts, how it reads the player's coins)
and the bonus sub-levels it or the levels lead to, with their own rules and exits. The slot's odds and
reel strips are tables read from the disks (DESIGN §2.3).

## Design sections this implements

DESIGN §2.1, §2.3.

## Files

- `engine/crates/sf-core/src/slot/{mod.rs,reels.rs,payout.rs}`
- `engine/crates/sf-core/src/bonus/{mod.rs}`
- `engine/crates/sf-core/src/scene.rs` — adds `Scene::Slot`, `Scene::Bonus`.
- `engine/crates/sf-core/tests/slot.rs`, `engine/conform/{scripts,cases}/slot-*`, `bonus-*`

## Interface

`Scene::Slot { coins_in }` returning `SlotOutcome { award }`; `Scene::Bonus(LevelId)` returning `LevelOutcome`.

## Tests

- [ ] routine conformance for the reel-stop and payout routines on every reachable reel state.
- [ ] frame conformance: a slot session for each payout kind (scripted stop timings found with `sf-conform record`), and zero coins.
- [ ] frame conformance: entering, completing and failing each bonus level.

## Done when

- [ ] Every payout kind and every bonus level in a green case.
- [ ] Slot and bonus variables in `LAYOUT`.
- [ ] Reel strips and odds come from `Tables` with their exe offsets documented.
