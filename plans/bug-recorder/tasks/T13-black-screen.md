# T13 — black-screen

**Phase:** 3 · **Depends on:** T12 · **Weight:** heavy

## Goal

The plan's acceptance case: the owner's bug (black screen instead of W1-2's intro after W1-1's slot
machine; fire moves on to the world map and W1-2 plays normally; owner, 2026-10-20) recorded by the
owner in the app and taken through `docs/BUG-FIX.md` to a fix with a permanent case, or to a report
that the original does the same.

## Design sections this implements

DESIGN §1 success criteria, §2.10.

## Needs a person

The worker builds the release app first and checks `bugs/` is empty of a previous attempt. Then it
asks the owner:

```
cd engine && cargo run --release -p sf-app
Game ▸ Start Level…: W1-1, defaults, recorder on → play to the exit → at the tally choose the slot
machine → play it → when the black screen shows, press STOP (or ⌘.) → note "black screen after slot"
→ Save → paste me the path
```

Expect: the black screen in place of W1-2's intro, then a zip in `bugs/`.
Tell me: the path, and whether the black screen appeared this time.

If it did not appear, the worker asks once more with the gamble's outcome noted (won or lost). It
does not guess a reproduction itself, because the owner's real play is the thing under test.

## Automated checks (the worker runs these)

```
sf-conform bug replay <zip>          # ends on the screenshot; the frames before STOP show the black screen
sf-conform bug compare <zip>         # same → quirk report; differs → --strict, fix, bug case, fails-before/passes-after
```

## Tests

- [ ] if ours: the new case fails on the commit before the fix and passes after, and is in `tests/cases.rs` (or ignored with its time); the fix commit has `Escape: rust-engine T{nn}` or `Escape: unknown`
- [ ] if the original's: the plain-words report with `original-stop.png` and `engine-stop.png` beside the zip, nothing committed

## Done when

- [ ] the owner's zip went through every step of `docs/BUG-FIX.md`, each step's result in PROGRESS
- [ ] either the fix and its case are in and the replay of the owner's zip on the fixed engine shows W1-2's intro, or the quirk report is delivered
- [ ] the test command is green, and the full workspace suite (`cd engine && CARGO_TERM_COLOR=never cargo test -q --workspace`, DESIGN §5) has been run once with its result in PROGRESS; a dated FINDINGS row records the outcome and the owner's recording
