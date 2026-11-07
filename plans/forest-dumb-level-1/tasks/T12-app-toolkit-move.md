# T12 — app-toolkit-move

**Phase:** 3 · **Depends on:** — · **Weight:** heavy

## Goal
Move the Controls, Start Level, recording bar and save windows, settings and the script parser core from `sf-app` into the toolkit, because this game needs them and is their second user.
## Design sections this implements
§2.7, §3.6, §7.
## Reuses
`sf-app/src/{controls_window,start_level_window,recording_bar,save_window,settings,args,input,script,recorder}.rs`. The models are AppKit-free and need a host trait. `window.rs` and `menu.rs` stay per game.
## Files
`toolkit/crates/amiga-app/src/{controls_window,start_level_window,recording_bar,save_window,settings,script_core}.rs`, `toolkit/crates/amiga-app/Cargo.toml`, `sf-app/src/*`, `games/superfrog/smoke.toml`, `toolkit/smoke.toml`.
## Interface
```
pub trait ControlsHost { fn bindings(&self) -> Bindings; fn set_binding(&mut self, a: Action, k: Key); }
pub struct LevelField { pub label: String, pub kind: FieldKind }  // Dropdown, Number, Checkbox
pub struct StartLevelSpec { pub fields: Vec<LevelField> }   // the game names its fields, so the second game can list one level
pub fn open_start_level(spec: &StartLevelSpec, on_ok: impl FnOnce(Values)) ;
pub trait AppHost { fn class_prefix(&self) -> &str; }      // ObjC class names may not name a game
pub fn parse_script_core<E: ScriptEvent>(text: &str) -> Result<Vec<E>, ScriptError>
```
## Tests
- [ ] `sf-e2e` `controls.rs`, `menus.rs`, `recorder_start_level.rs`, `recorder_bar.rs`, `recorder_save_window.rs`, `persist.rs` pass unchanged.
- [ ] A dry-fit test builds a one-field start-level spec with no game name.
- [ ] `tools/check_tier_names.py` passes.
## Done when
- [ ] Superfrog's app behaves identically; its e2e suite is green.
- [ ] The moved models have unit tests in the toolkit.
- [ ] `toolkit/README.md` lists what moved.
