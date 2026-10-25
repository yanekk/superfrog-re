# T28 compass trial (2026-10-24/25)

`sf-plan <level> --engine-only --seed N --budget-min 30 --threads 3 --compass C`, binary 5e659de,
unguided; per level and seed the three compasses ran side by side (9 of 10 cores, load 7–26 with
another plan's worker running). Map compass maps prebuilt into the cache, 5 threads: W2-3 180 s,
W3-4 137 s, W5-3 251 s, W6-1 170 s, W6-2 382 s, W6-4 296 s. Walk compass: `Explorer::new` 1.4–16.5 ms.

Cell: ✓ completed / ✗ budget spent; engine frames (M, millions, `PLAN_STATS engine_frames`, shortening
included); rounds; for ✓ route frames and wall-clock; for ✗ coins still needed and the best entry's
exit distance with the coins met (cells for straight and map, walking steps for walk, so not
comparable across compasses). Walk fills: fills held at the end and their bytes, outside the archive.

| Level | Seed | straight | map | walk | walk fills |
|---|---|---|---|---|---|
| W2-3 | 1 | ✗ 21.14 M, 1339 r, coins 0, dist 17 | ✗ 21.29 M, 1344 r, coins 0, dist 17 | ✗ 20.71 M, 1177 r, coins 0, dist 39 | 500, 25.2 MB |
| W2-3 | 2 | ✗ 18.35 M, 1005 r, coins 0, dist 17 | ✗ 18.30 M, 1004 r, coins 0, dist 17 | ✗ 18.56 M, 1097 r, coins 0, dist 39 | 257, 13.0 MB |
| W2-3 | 3 | ✗ 16.06 M, 1039 r, coins 0, dist 17 | ✗ 16.03 M, 1038 r, coins 0, dist 17 | ✗ 16.06 M, 1051 r, coins 0, dist 37 | 253, 12.8 MB |
| W3-4 | 1 | ✗ 27.54 M, 1451 r, coins 43, dist none | ✗ 27.64 M, 1453 r, coins 43, dist none | ✗ 27.64 M, 1453 r, coins 44, dist none | 339, 17.1 MB |
| W3-4 | 2 | ✗ 21.60 M, 1467 r, coins 0, dist 135 | ✗ 21.82 M, 1471 r, coins 0, dist 135 | ✗ 21.22 M, 1450 r, coins 0, dist 91 | 436, 22.0 MB |
| W3-4 | 3 | ✗ 24.30 M, 1381 r, coins 0, dist 13 | ✗ 24.30 M, 1381 r, coins 0, dist 13 | ✗ 23.93 M, 1437 r, coins 0, dist 35 | 441, 22.2 MB |
| W5-3 | 1 | ✗ 24.57 M, 1665 r, coins 0, dist 42 | ✗ 24.66 M, 1712 r, coins 0, dist 42 | ✗ 24.87 M, 1946 r, coins 0, dist 42 | 207, 10.4 MB |
| W5-3 | 2 | ✓ 7.29 M, 708 r, 5782 f, 8.8 min | ✓ 7.29 M, 730 r, 5974 f, 9.2 min | ✓ 7.47 M, 746 r, 5866 f, 9.3 min | 176, 8.9 MB |
| W5-3 | 3 | ✓ 27.86 M, 1924 r, 7753 f, 30.1 min | ✓ 23.89 M, 1931 r, 6785 f, 27.2 min | ✓ 7.84 M, 797 r, 5611 f, 9.7 min | 167, 8.4 MB |
| W6-1 | 1 | ✓ 9.81 M, 948 r, 6207 f, 12.2 min | ✓ 9.81 M, 948 r, 6207 f, 12.1 min | ✓ 8.45 M, 882 r, 6217 f, 11.4 min | 197, 9.9 MB |
| W6-1 | 2 | ✓ 10.03 M, 988 r, 7773 f, 12.9 min | ✓ 10.34 M, 1016 r, 7776 f, 13.0 min | ✓ 9.56 M, 908 r, 8510 f, 11.5 min | 222, 11.2 MB |
| W6-1 | 3 | ✓ 10.21 M, 1001 r, 9937 f, 11.6 min | ✓ 10.17 M, 997 r, 10169 f, 11.6 min | ✓ 6.87 M, 644 r, 8155 f, 8.9 min | 230, 11.6 MB |
| W6-2 | 1 | ✗ 27.27 M, 1615 r, coins 0, dist 6 | ✗ 27.17 M, 1613 r, coins 0, dist 6 | ✗ 28.17 M, 1647 r, coins 0, dist 6 | 391, 19.7 MB |
| W6-2 | 2 | ✗ 26.60 M, 1801 r, coins 0, dist 6 | ✗ 26.65 M, 1803 r, coins 0, dist 6 | ✗ 27.08 M, 1905 r, coins 0, dist 6 | 393, 19.8 MB |
| W6-2 | 3 | ✗ 27.10 M, 1803 r, coins 0, dist 6 | ✗ 27.19 M, 1806 r, coins 0, dist 6 | ✗ 27.34 M, 1677 r, coins 0, dist 6 | 364, 18.3 MB |
| W6-4 | 1 | ✗ 25.70 M, 1669 r, coins 0, dist 93 | ✗ 25.75 M, 1671 r, coins 0, dist 93 | ✗ 25.87 M, 1752 r, coins 0, dist 48 | 768, 38.7 MB |
| W6-4 | 2 | ✗ 26.04 M, 1536 r, coins 0, dist 13 | ✗ 26.04 M, 1536 r, coins 0, dist 13 | ✗ 26.18 M, 1416 r, coins 0, dist 48 | 751, 37.9 MB |
| W6-4 | 3 | ✗ 26.33 M, 2005 r, coins 0, dist 13 | ✗ 26.30 M, 2004 r, coins 0, dist 13 | ✗ 26.05 M, 1620 r, coins 0, dist 48 | 854, 43.0 MB |

Completed: straight 5, map 5, walk 5 (the same five). Engine frames over those five: straight
65.19 M, map 61.50 M, walk 40.19 M (−38%; without W5-3 seed 3, 32.35 M against 37.34 M, −13%).
Walk used fewer frames than straight on 4 of the 5 (W5-3 seed 2: +2.6%). Map ran within 2% of
straight on 4 of the 5 and identically where its map had no path. Choice (T28's rule): walk, the
only compass beating straight; `sf-plan`'s default is now `--compass walk`.

Before the last 8 coins the pull is 1.0 for every compass (`Explorer::weights`), so the three
searches are the same search until then (W3-4 seed 1, 43 coins left: map and walk identical frames).
