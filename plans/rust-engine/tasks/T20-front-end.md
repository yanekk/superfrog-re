# T20 — front-end

**Phase:** 2 · **Depends on:** T08, T11, T12, T13 · **Weight:** heavy

## Goal

Everything around the levels: the Team 17 logo and intro, the story animation, the title screen and
its attract mode, the options screen, password entry and the password scheme, the world map if the
game shows one, game over, and the end sequence. Each is a scene with its own music and timing, and
each must match the original frame for frame, including how long it waits and what keys skip it.

## Design sections this implements

DESIGN §2.1, §2.6 (keys the original reads), §2.9 (passwords as progress).

## Files

- `engine/crates/sf-core/src/front/{mod.rs,intro.rs,story.rs,title.rs,options.rs,password.rs,worldmap.rs,gameover.rs,ending.rs}`
- `engine/crates/sf-core/src/scene.rs` — adds the front-end scenes.
- `engine/crates/sf-core/tests/front.rs`, `engine/conform/{scripts,cases}/front-*`
- `docs/13-front-end.md` — scene list, timings, password algorithm.

## Interface

```rust
pub fn password_for(progress: &Progress) -> Password;  pub fn decode_password(p: &Password) -> Option<Progress>;
pub struct Progress { pub world: u8 /* plus whatever the password encodes */ }
```

## Tests

- [ ] frame conformance from power-on through the intro and story untouched to the title, and the same with every skip key.
- [ ] title attract mode cycling for 5 000 frames.
- [ ] options: every option changed and its effect visible as the original shows it.
- [ ] password: every valid password from the original's routine (routine conformance, exhaustive if the space allows) and a selection of invalid ones; entry screen frame conformance.
- [ ] game over and the end sequence, entered from transplanted states, to their end.

## Done when

- [ ] All cases green; power-on-to-title in the test block.
- [ ] docs/13 written, including the password algorithm.
- [ ] Front-end variables in `LAYOUT`.
