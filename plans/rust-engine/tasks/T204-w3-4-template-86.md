# T204 — w3-4-template-86

**Phase:** 2 · **Depends on:** T201; blocks T203 · **Weight:** medium

Added by T201 with the owner's yes (2026-10-16), the goal widened by the owner.

## Goal

1. Port the object the original spawns at W3-4's start when the frog stands still: at frame 212
   of `{ level = "W3-4" }` with nothing held, template 86 (init h0+$24216, update h0+$24236; vy
   −2, immune `$7D00`, passive, touch `nothing`) goes into slot 6 from map cell 285,63
   (`level_map` +$97DA). Find what spawns it (a marker spawner, or another object's routine) and
   port both, faithful and alone. Today the engine spawns nothing and reports nothing unported.
2. An object template the engine meets and has no kind for is reported in `Game::unported`
   (owner: a missing object that is silent is the worry), never skipped silently.
3. A sweep: every object template any level's map can spawn (through each world's marker
   spawners and through the objects that spawn others), listed against the templates the engine's
   dispatch registers; each one missing named in `FINDINGS.md`, or ported if small.

## Files

`engine/crates/sf-core/src/{enemies,items,level}/`; `engine/conform/{scripts,cases}/`;
`engine/crates/sf-conform/tests/`.

## Interface

None new beyond what `Game::unported` already returns.

## Tests

- [ ] A case from `{ level = "W3-4" }`, nothing held, ~600 frames: green faithful and alone, the
      object spawned on both sides at 212.
- [ ] `code-w3-4` (T201, cut to 2250 frames to stop before 212) run on past it, green.
- [ ] An engine test: a spawn of a template with no kind lands in `unported`.
- [ ] The sweep as a test or an example whose output is recorded in `FINDINGS.md`.

## Done when

- [ ] All green, nothing unported in the W3-4 cases; the sweep's missing templates ported or each
      named in `FINDINGS.md` with its level and spawner.
