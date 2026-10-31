# 01 — Disk location

Where the tools and tests look for the game's disks (`fd-tools`' `locate`), in this order.

1. `FOREST_DUMB_DISKS`, the environment variable, used as given.
2. `<repo>/games/forest-dumb`, this game's folder in the main checkout.
3. `<repo>`, the root of the main checkout.

`<repo>` is the main checkout, the parent of the git common dir, so the lookup works from any
worktree. Disks are copyrighted: they are never committed, nor is anything derived from them.

The named default is the owner's copy, outside the repo: `~/Documents/Amiga`. Point
`FOREST_DUMB_DISKS` there. A session cannot list that folder (macOS privacy protection), so the
names below come from the north-star's record of 2026-10-27 (J1), not from a check by this session.

The disk set itself: two zips, each holding one 901 120-byte OFS `.adf`, volume `FOREVER`:

- `Forest Dumb Forever!!! (1996)(L.K. Avalon)(PL)(Disk 1 of 2)[cr WT].zip`
- `Forest Dumb Forever!!! (1996)(L.K. Avalon)(PL)(Disk 2 of 2)[cr WT].zip`

`[cr WT]` marks a cracked release. Disk 1 boots a `startup-sequence` of `Show -o Kwach4`,
`BillClintonToDupek` (the crack's picture and intro) and `forest_dumb_forever`. Disk 2 holds the data
and the `!EAGLE_SOFT!` marker file. The executable and all 48 data files are byte-identical to the
WHDLoad package's (its executable after RNC unpacking). Whether the crack also changed the executable
cannot be told without an uncracked image.

Decided 2026-10-30: the cracked `[cr WT]` set is game two's source. The faithful bar's measuring stick
is this cracked release, whose executable may differ from the original's. An uncracked copy, if one
turns up, is the thing to compare it against.
