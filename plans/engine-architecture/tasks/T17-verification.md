# T17 — verification

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Write the chapter for the verification machinery: `sf-oracle` (vAmiga headless and the Moira routine runner, with its patches and shim), `sf-conform` (cases, scripts, sync points, comparison, reports, bug replay) with the cases under `engine/conform/`, and `sf-e2e`. It is most of the reusable code, and it is what a native rewrite must keep passing, so it gets more than a light treatment on two points: reuse, and the address coupling (DESIGN §2.5) from the checks' side.

## Design sections this implements

DESIGN §2.3, §2.5. Sources: `plans/rust-engine/DESIGN.md` §2.1, §4; `docs/RE-WORKBENCH.md`; `docs/BUG-FIX.md`; `engine/conform/README.md`.

## Files

- `docs/architecture/verification.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- What each check compares (state by address, picture, sound per channel, milestones, plays-alone) and what it needs from the engine (`probe`, `Start::Transplant`, `FrameOut`, `SceneKind`, the pacing feed).
- For each, what a native rewrite must still provide, and which checks would need an adapter.
- Reuse: generic (oracle harness, routine runner, comparison and reporting) versus Superfrog-specific (cases, scripts, sync points, route planners), with sizes.
- The vAmiga patches: what each hook exposes and why.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Rate generic and specific parts in the text; the front matter carries the combined one.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/verification.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
