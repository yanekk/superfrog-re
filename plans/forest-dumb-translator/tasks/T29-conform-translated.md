# T29 — conform-translated

**Phase:** 5 · **Depends on:** T28 · **Weight:** medium

## Goal

Run the conformance cases with the translated engine and compare what the frame bar asks (DESIGN §2.7): state, picture and the frame's custom writes, Paula's in order. Then a first reading of all 42 level 1 cases, sorted by first failure, which T30 and T31 work from.

## Design sections this implements

DESIGN §2.7.

## Files

- `fd-conform/src/main.rs`, `lib.rs`, `case.rs`: `--engine hand|translated`, a `writes` comparison channel for Paula, `engine/conform/README.md`. The Paula comparison reuses `amiga-conform`'s (`compare.rs`, `sound_events.rs`, `channels.rs`, as `sf-conform/src/case.rs` uses it), not a new one (plan review, 2026-11-11).

## Interface

```
fd-conform run cases/<case>.toml --engine translated
fd-conform run-all --engine translated --report build/translated-cases.md
```

## Tests

- [ ] A made-up case on a stand-in engine fails when a Paula write differs in order and passes when equal.
- [ ] `--engine hand` results are unchanged on every case (full).

## Done when

- [ ] `build/translated-cases.md` lists every level 1 case with pass or first difference; the summary is in FINDINGS.
- [ ] Failures are grouped for T30 (character, camera, stretch, picture, idle) and T31 (collect, contact, death, enemy, mechanic, level end, routes).
