# Implementation plan

13 tasks in 3 phases. Each has a file in [tasks/](tasks/) with its goal, files, interface and what done
means. Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

## Shape of the build

- The one unknown the level data rests on, where each world's items-needed number comes from, is settled
  first (T00), before the level data is written around it.
- Everything pure (level data, object names, the live reader, the secrets rule) is built and proven before
  the page or the app shows any of it; the page then draws data already known to be right.
- The Chrome rig comes before any page task, so page tasks write tests, not infrastructure.
- The open-ended research (T07, secrets) starts on day one and only the Secrets layer waits on it.

```
Phase 0  ▸  T00                    items-needed source        throwaway
Phase 1  ▸  T01 T02 T03 T06 T07    names, level data, rig, live reader, secrets research
Phase 2  ▸  T04 T05 T08 T09 T10 T11 T12   page, command, app, drill
```

## Phase 0 — Prove the ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-items-needed-source.md) | items-needed-source | — |

T00 gates T02's `items_needed`: a constant per world means reading the program's bytes; a value computed
from the map means a pure function of the map; a value only the run produces means running the translated
program headless to each world's `level_init` when the page is written, a shell step of seconds.

## Phase 1 — The pure parts and the rig

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-objects-all-worlds.md) | objects-all-worlds | — |
| [T02](tasks/T02-map-levels.md) | map-levels | T00, T01 |
| [T03](tasks/T03-chrome-rig.md) | chrome-rig | — |
| [T06](tasks/T06-live-reader.md) | live-reader | — |
| [T07](tasks/T07-secrets-research.md) | secrets-research | — |

At the end: every level's data is right against the files and the original, the live reader is right against
headless runs, Chrome can be driven, and the secret kinds are known.

## Phase 2 — The page, the command, the app

| # | Task | Depends on |
|---|---|---|
| [T04](tasks/T04-page-map-only.md) | page-map-only | T03 |
| [T05](tasks/T05-page-live.md) | page-live | T04 |
| [T08](tasks/T08-map-command.md) | map-command | T02, T05 |
| [T09](tasks/T09-secrets-layer.md) | secrets-layer | T07, T08 |
| [T10](tasks/T10-app-live.md) | app-live | T02, T05, T06 |
| [T11](tasks/T11-app-show-map.md) | app-show-map | T08, T10 |
| [T12](tasks/T12-map-drill.md) | map-drill | T09, T11 |

At the end: the owner opens Game ▸ Show Map and the page follows the game.

## Main path: builder and wirer

| Step | Built by | Wired by |
|---|---|---|
| Command writes the six-level page | T02 (data), T04 (map-only page) | T08 (`fd-routeview/src/main.rs`) |
| Page reads `live.js`, draws the dot | T05 (`page.html`) | T08 (`PageSpec.live` set in `fd_map::spec`) |
| App reads its state | T06 (`fd_core::live`) | T10 (`fd-app/src/translated.rs`, `main.rs`) |
| App writes `live.js` | T05 (`write_live`) | T10 (`fd-app/src/main.rs`) |
| Game ▸ Show Map | T08 (`fd_map::write_site`) | T11 (`fd-app/src/menu.rs`, `main.rs`) |
| Secrets layer | T07 (`fd_assets::secrets`) | T09 (`fd-map`) |

## Critical path

```
T03 → T04 → T05 → T10 → T11 → T12
```

T05 → T08 → T11 is as long. T07 (heavy research) runs beside them and joins at T09. Leaves: T12 only.

## Parallel width

13 tasks · longest dependency chain 6 · up to 5 could run at once (T00, T01, T03, T06, T07).

## Rough sizing

| Weight | Tasks |
|---|---|
| Heavy | T05, T07, T10 |
| Medium | T01, T02, T03, T06, T08, T11, T12 |
| Light | T00, T04, T09 |

T07 may overrun: how much the original hides is unknown. T05 is the most page code (live drawing, follow,
switching). T10 touches the app's program thread, where timing matters.

## Decisions still open

- What T07 finds decides what the Secrets layer holds; it does not block anything but T09.
- T00's answer decides how T02 reads items needed; T02 waits on it.
