# T00 spike: raw results

Made by `engine/crates/sf-conform/examples/zz_map_spike.rs` (deleted at the task's end; its last
version is in the T00 commits' history). 2026-10-21, Apple M4 10 cores, on a busy machine: other
pir workers ran beside it, load average 10 to 34 (each file's first line or log names it). Timings
are therefore pessimistic; counts are machine-independent.

Start: `Start::Level(id)` on the engine, idled to frame 151 (frog standing), not the oracle's
`AtLevel` state. Nodes are the frog's cell `((X+0x17)/16, (Y-0x10)/16)` plus wings (`frog+$90`);
with `bands`, also speed band (|VX| 0, under half `MAX_SPEED`, at or over) and facing. Moves (36,
plus 12 flight moves with wings): walk 6/12/24, jump 4/10/20 holding the direction, hop, run-up
8/16/32 then jump, jump up 4/20, down, stop, and flight (a jump, then fire pressed 4/8/16 times).
After its inputs a move holds its tail until landed (cap 150 frames); cells-only maps then idle until
VX is 0 (the node's state is the frog stopped). A move losing a heart, dying or never landing is not
an edge. `--waits` (W6-3-waits.txt only) adds ride 25-200 and wait 12-200 then jump/walk.

Every map build except `q3-build-drawn` ran with a local, uncommitted sf-core patch that skips
`render::level_as_drawn` in `Scene::step` (see q3-profile.txt); the game state is identical.

| File | Question |
|---|---|
| `*-maps.txt` | the live, walked map: size, build time, coins touched, exit |
| `*-q1-seeding.txt` | Q1, frog placed by writing X/Y vs walked; false edges from real states |
| `*-q2-freeze.txt` | Q2, enemy markers cleared before entry vs live |
| `q3-profile.txt`, `*-q3-build-*.txt` | Q3, where `step_unseen`'s time goes; drawn vs skipped |
| `*-q4-stall.txt` | Q4, every stall coin: the map's verdict and a replay from the start |
| `*-q5-enemies.txt` | Q5, object classification; enemies in a slot parked vs walked |
| `*-q6-bands.txt`, `*-bands.log` | Q6, cells only vs cell + band + facing |
| `*-q7-live-failure.txt` | Q7, 500 edges played in a random walk from real states, enemies live |
| `*-pokereach.txt` | the placed-frog graph's reach from the start |
| `W6-3-explore.txt` | random bursts from every W6-3 node: cells the map lacks and their inputs |
| `W6-3-route.txt`, `W6-3-waits.txt` | the committed route on the engine (dies at 1785); the wait moves |
| `*.log` | the build logs |
