# 01 — Disk location

Where the tools and tests look for the game's disks. Replace `<GAME>` and `<game>` when copying.

1. `<GAME>_DISKS`, the environment variable, used as given.
2. `<repo>/games/<game>`, this game's folder in the main checkout.
3. `<repo>`, the root of the main checkout.

`<repo>` is the main checkout, the parent of the git common dir, so the lookup works from any
worktree. Disks are copyrighted: they are never committed, nor is anything derived from them.

The disk set itself: <file names, how many, which release>.
