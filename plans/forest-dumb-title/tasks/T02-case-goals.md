# T02 — case-goals

**Phase:** 0 · **Depends on:** — · **Weight:** light

## Goal

The title cases end in two ways the runner cannot state: both sides back at the title, and both sides
quitting to AmigaDOS. Add two general goals to `fd-conform`'s case format for cases lined up by loads,
so T03 to T06 write them as data and later plans can reuse them.

## Design sections this implements

DESIGN §2.3.

## Files

- `games/forest-dumb/engine/crates/fd-conform/src/case.rs`, `src/frames.rs` (`run_entry_frames`).
- `games/forest-dumb/engine/conform/README.md` "A case", `goal`.
- `games/forest-dumb/engine/crates/fd-conform/tests/frames.rs` or a new `tests/case_goals.rs`, labelled
  in `games/forest-dumb/smoke.toml`.

## Interface

```toml
goal = { enters = "title_screen", by_frame = 9000 }   # both sides enter the routine, same aligned frame
goal = { exited = true, by_frame = 5400 }             # both sides return from the program's entry
```

```rust
pub enum Goal { …, Enters { routine: String, by_frame: u32 }, Exited { by_frame: u32 } }
```

`enters`: the routine must be a `func` of `re/symbols.toml`; the original's entry is seen with
`Amiga::watch_pcs`, the translated program's with its entry hook (`write_at`'s mechanism). `exited`:
the original reaches the return address the entry was called with, with the stack as the entry found
it; the translated program reports `TranslatedGame::exited`. Both only on `align = "loads"` cases; a
case using one elsewhere is refused at parse.

`exited` must hold on a snapshot start too: `title-koniec` starts from the 4990 snapshot, and
`TranslatedGame`'s `exit_to` is set today only by the power-on start (`fd-translated/src/lib.rs`), so
from a snapshot every return to AmigaDOS would read as a stop. The snapshot start takes the entry's
return address and stack from the original, as the power-on start leaves them.

## Tests

- [ ] Parse: both forms; an unknown routine, a missing `by_frame`, use without `align = "loads"` are errors.
- [ ] On stand-ins (a second copy of the original, `tests/common`): both enter the routine in the same
      frame passes; one side a frame later fails naming both frames; neither by `by_frame` fails.
- [ ] `exited`: a stand-in returning from the entry passes, from power-on and from a snapshot; a jump to 0 that is not the entry's return
      fails as a stop on the translated side.

## Done when

- [ ] The two goals parse, run and are documented in `conform/README.md`.
- [ ] The stand-in tests pass and are labelled; the existing cases' readings are unchanged.
