# Level index & tile addressing

Address any tile as **`<level-id>:<col>,<row>`** (0-indexed, origin top-left, 16px blocks). E.g. `W1-1:94,53`.

Secret passages are boxed and labelled S1.. on each image.

| level id | chunk | world | size | passages |
|----------|-------|------:|------|---------:|
| W1-1 | d1_c21_level | 1 | 300x84 | 2 |
| W1-2 | d1_c22_level | 1 | 300x84 | 11 |
| W1-3 | d1_c23_level | 1 | 300x84 | 2 |
| W1-4 | d1_c24_level | 1 | 300x84 | 7 |
| W2-1 | d1_c31_level | 2 | 300x84 | 5 |
| W2-2 | d1_c32_level | 2 | 300x84 | 13 |
| W2-3 | d1_c33_level | 2 | 300x84 | 18 |
| W2-4 | d1_c34_level | 2 | 300x84 | 20 |
| W3-1 | d2_c12_level | 3 | 300x84 | 2 |
| W3-2 | d2_c13_level | 3 | 300x84 | 4 |
| W3-3 | d2_c14_level | 3 | 300x84 | 2 |
| W3-4 | d2_c15_level | 3 | 300x84 | 2 |
| W4-1 | d2_c22_level | 4 | 300x84 | 11 |
| W4-2 | d2_c23_level | 4 | 300x84 | 10 |
| W4-3 | d2_c24_level | 4 | 300x84 | 5 |
| W4-4 | d2_c25_level | 4 | 300x84 | 3 |
| W5-1 | d2_c32_level | 5 | 300x84 | 1 |
| W5-2 | d2_c33_level | 5 | 300x84 | 5 |
| W5-3 | d2_c34_level | 5 | 300x84 | 1 |
| W5-4 | d2_c35_level | 5 | 300x84 | 2 |
| WB-1 | d2_c42_level | B | 300x84 | 0 |
| W6-1 | d4_c02_level | 6 | 300x84 | 0 |
| W6-2 | d4_c03_level | 6 | 300x84 | 5 |
| W6-3 | d4_c04_level | 6 | 300x84 | 4 |
| W6-4 | d4_c05_level | 6 | 300x84 | 2 |
| WW-1 | d4_c13_level | W | 300x84 | 0 |
| W6-5 | d4_c14_level | 6 | 300x84 | 0 |
| W6-6 | d4_c15_level | 6 | 300x84 | 5 |
| W6-7 | d4_c17_level | 6 | 300x84 | 2 |

## Passages per level


### W1-1 (d1_c21_level)
- S1: cols 94-95, rows 51-62 (24 cells) — id `W1-1:94,51`
- S2: cols 185-199, rows 41-43 (45 cells) — id `W1-1:185,41`

### W1-2 (d1_c22_level)
- S1: cols 17-18, rows 57-67 (22 cells) — id `W1-2:17,57`
- S2: cols 21-22, rows 42-51 (20 cells) — id `W1-2:21,42`
- S3: cols 25-26, rows 26-36 (22 cells) — id `W1-2:25,26`
- S4: cols 36-49, rows 23-25 (42 cells) — id `W1-2:36,23`
- S5: cols 38-59, rows 6-9 (88 cells) — id `W1-2:38,6`
- S6: cols 118-121, rows 40-43 (16 cells) — id `W1-2:118,40`
- S7: cols 138-147, rows 10-12 (30 cells) — id `W1-2:138,10`
- S8: cols 228-233, rows 19-21 (18 cells) — id `W1-2:228,19`
- S9: cols 252-268, rows 6-8 (51 cells) — id `W1-2:252,6`
- S10: cols 255-287, rows 42-44 (99 cells) — id `W1-2:255,42`
- S11: cols 279-290, rows 6-8 (36 cells) — id `W1-2:279,6`

### W1-3 (d1_c23_level)
- S1: cols 41-50, rows 9-11 (30 cells) — id `W1-3:41,9`
- S2: cols 170-172, rows 8-11 (12 cells) — id `W1-3:170,8`

### W1-4 (d1_c24_level)
- S1: cols 134-145, rows 1-7 (48 cells) — id `W1-4:134,1`
- S2: cols 148-149, rows 5-6 (4 cells) — id `W1-4:148,5`
- S3: cols 148-149, rows 8-9 (4 cells) — id `W1-4:148,8`
- S4: cols 148-149, rows 11-12 (4 cells) — id `W1-4:148,11`
- S5: cols 148-149, rows 14-15 (4 cells) — id `W1-4:148,14`
- S6: cols 148-149, rows 17-18 (4 cells) — id `W1-4:148,17`
- S7: cols 148-152, rows 20-22 (15 cells) — id `W1-4:148,20`

### W2-1 (d1_c31_level)
- S1: cols 52-79, rows 16-19 (112 cells) — id `W2-1:52,16`
- S2: cols 52-59, rows 22-24 (24 cells) — id `W2-1:52,22`
- S3: cols 71-72, rows 31-33 (6 cells) — id `W2-1:71,31`
- S4: cols 78-79, rows 29-31 (6 cells) — id `W2-1:78,29`
- S5: cols 85-88, rows 20-23 (16 cells) — id `W2-1:85,20`

### W2-2 (d1_c32_level)
- S1: cols 24-67, rows 61-63 (132 cells) — id `W2-2:24,61`
- S2: cols 38-39, rows 12-47 (72 cells) — id `W2-2:38,12`
- S3: cols 47-59, rows 7-9 (39 cells) — id `W2-2:47,7`
- S4: cols 47-67, rows 52-54 (63 cells) — id `W2-2:47,52`
- S5: cols 75-81, rows 74-76 (21 cells) — id `W2-2:75,74`
- S6: cols 92-98, rows 61-65 (27 cells) — id `W2-2:92,61`
- S7: cols 107-119, rows 3-5 (39 cells) — id `W2-2:107,3`
- S8: cols 121-122, rows 8-12 (10 cells) — id `W2-2:121,8`
- S9: cols 232-251, rows 40-42 (60 cells) — id `W2-2:232,40`
- S10: cols 240-251, rows 5-7 (36 cells) — id `W2-2:240,5`
- S11: cols 249-267, rows 57-67 (90 cells) — id `W2-2:249,57`
- S12: cols 261-267, rows 3-6 (28 cells) — id `W2-2:261,3`
- S13: cols 273-287, rows 71-74 (60 cells) — id `W2-2:273,71`

### W2-3 (d1_c33_level)
- S1: cols 10-31, rows 3-7 (78 cells) — id `W2-3:10,3`
- S2: cols 10-24, rows 37-39 (45 cells) — id `W2-3:10,37`
- S3: cols 10-25, rows 73-75 (48 cells) — id `W2-3:10,73`
- S4: cols 32-38, rows 14-34 (75 cells) — id `W2-3:32,14`
- S5: cols 45-92, rows 73-75 (144 cells) — id `W2-3:45,73`
- S6: cols 102-105, rows 4-6 (12 cells) — id `W2-3:102,4`
- S7: cols 102-104, rows 59-61 (9 cells) — id `W2-3:102,59`
- S8: cols 112-115, rows 4-6 (12 cells) — id `W2-3:112,4`
- S9: cols 118-119, rows 11-12 (4 cells) — id `W2-3:118,11`
- S10: cols 148-161, rows 22-26 (48 cells) — id `W2-3:148,22`
- S11: cols 148-161, rows 60-64 (48 cells) — id `W2-3:148,60`
- S12: cols 164-165, rows 50-64 (30 cells) — id `W2-3:164,50`
- S13: cols 190-194, rows 50-52 (15 cells) — id `W2-3:190,50`
- S14: cols 192-193, rows 9-10 (4 cells) — id `W2-3:192,9`
- S15: cols 193-194, rows 43-44 (4 cells) — id `W2-3:193,43`
- S16: cols 194-206, rows 70-73 (52 cells) — id `W2-3:194,70`
- S17: cols 237-247, rows 51-53 (33 cells) — id `W2-3:237,51`
- S18: cols 258-284, rows 16-18 (81 cells) — id `W2-3:258,16`

### W2-4 (d1_c34_level)
- S1: cols 2-3, rows 12-20 (18 cells) — id `W2-4:2,12`
- S2: cols 10-21, rows 8-11 (48 cells) — id `W2-4:10,8`
- S3: cols 47-65, rows 2-4 (57 cells) — id `W2-4:47,2`
- S4: cols 74-75, rows 37-39 (6 cells) — id `W2-4:74,37`
- S5: cols 84-86, rows 17-18 (6 cells) — id `W2-4:84,17`
- S6: cols 106-109, rows 71-73 (12 cells) — id `W2-4:106,71`
- S7: cols 123-126, rows 71-73 (12 cells) — id `W2-4:123,71`
- S8: cols 190-192, rows 3-5 (9 cells) — id `W2-4:190,3`
- S9: cols 203-213, rows 74-76 (33 cells) — id `W2-4:203,74`
- S10: cols 205-207, rows 3-5 (9 cells) — id `W2-4:205,3`
- S11: cols 216-218, rows 3-5 (9 cells) — id `W2-4:216,3`
- S12: cols 220-221, rows 58-69 (24 cells) — id `W2-4:220,58`
- S13: cols 227-229, rows 3-5 (9 cells) — id `W2-4:227,3`
- S14: cols 234-236, rows 70-72 (9 cells) — id `W2-4:234,70`
- S15: cols 253-255, rows 11-14 (12 cells) — id `W2-4:253,11`
- S16: cols 253-255, rows 16-18 (9 cells) — id `W2-4:253,16`
- S17: cols 253-255, rows 20-22 (9 cells) — id `W2-4:253,20`
- S18: cols 253-256, rows 24-26 (12 cells) — id `W2-4:253,24`
- S19: cols 261-263, rows 24-26 (9 cells) — id `W2-4:261,24`
- S20: cols 281-283, rows 24-26 (9 cells) — id `W2-4:281,24`

### W3-1 (d2_c12_level)
- S1: cols 145-146, rows 31-32 (4 cells) — id `W3-1:145,31`
- S2: cols 206-217, rows 23-25 (36 cells) — id `W3-1:206,23`

### W3-2 (d2_c13_level)
- S1: cols 144-154, rows 62-66 (39 cells) — id `W3-2:144,62`
- S2: cols 154-230, rows 69-71 (231 cells) — id `W3-2:154,69`
- S3: cols 155-156, rows 26-32 (14 cells) — id `W3-2:155,26`
- S4: cols 243-256, rows 28-62 (118 cells) — id `W3-2:243,28`

### W3-3 (d2_c14_level)
- S1: cols 37-56, rows 60-62 (60 cells) — id `W3-3:37,60`
- S2: cols 114-116, rows 59-61 (9 cells) — id `W3-3:114,59`

### W3-4 (d2_c15_level)
- S1: cols 110-115, rows 27-30 (24 cells) — id `W3-4:110,27`
- S2: cols 290-291, rows 25-29 (10 cells) — id `W3-4:290,25`

### W4-1 (d2_c22_level)
- S1: cols 36-82, rows 36-38 (141 cells) — id `W4-1:36,36`
- S2: cols 37-71, rows 61-63 (105 cells) — id `W4-1:37,61`
- S3: cols 82-96, rows 61-63 (45 cells) — id `W4-1:82,61`
- S4: cols 93-126, rows 35-44 (116 cells) — id `W4-1:93,35`
- S5: cols 107-118, rows 61-63 (36 cells) — id `W4-1:107,61`
- S6: cols 112-141, rows 18-20 (90 cells) — id `W4-1:112,18`
- S7: cols 129-265, rows 61-63 (411 cells) — id `W4-1:129,61`
- S8: cols 211-265, rows 55-57 (165 cells) — id `W4-1:211,55`
- S9: cols 228-248, rows 31-32 (42 cells) — id `W4-1:228,31`
- S10: cols 269-270, rows 31-54 (48 cells) — id `W4-1:269,31`
- S11: cols 275-285, rows 61-69 (57 cells) — id `W4-1:275,61`

### W4-2 (d2_c23_level)
- S1: cols 16-34, rows 11-40 (138 cells) — id `W4-2:16,11`
- S2: cols 28-43, rows 50-53 (64 cells) — id `W4-2:28,50`
- S3: cols 42-63, rows 11-13 (66 cells) — id `W4-2:42,11`
- S4: cols 51-61, rows 51-53 (33 cells) — id `W4-2:51,51`
- S5: cols 81-82, rows 56-60 (10 cells) — id `W4-2:81,56`
- S6: cols 103-104, rows 54-56 (6 cells) — id `W4-2:103,54`
- S7: cols 155-156, rows 60-62 (6 cells) — id `W4-2:155,60`
- S8: cols 225-251, rows 12-17 (112 cells) — id `W4-2:225,12`
- S9: cols 244-263, rows 67-70 (80 cells) — id `W4-2:244,67`
- S10: cols 273-288, rows 68-70 (48 cells) — id `W4-2:273,68`

### W4-3 (d2_c24_level)
- S1: cols 45-61, rows 4-6 (51 cells) — id `W4-3:45,4`
- S2: cols 62-63, rows 34-46 (26 cells) — id `W4-3:62,34`
- S3: cols 137-138, rows 37-39 (6 cells) — id `W4-3:137,37`
- S4: cols 190-191, rows 38-39 (4 cells) — id `W4-3:190,38`
- S5: cols 190-261, rows 70-72 (216 cells) — id `W4-3:190,70`

### W4-4 (d2_c25_level)
- S1: cols 158-159, rows 63-68 (12 cells) — id `W4-4:158,63`
- S2: cols 176-205, rows 70-72 (90 cells) — id `W4-4:176,70`
- S3: cols 279-280, rows 65-69 (10 cells) — id `W4-4:279,65`

### W5-1 (d2_c32_level)
- S1: cols 135-147, rows 36-52 (91 cells) — id `W5-1:135,36`

### W5-2 (d2_c33_level)
- S1: cols 41-55, rows 12-15 (60 cells) — id `W5-2:41,12`
- S2: cols 41-59, rows 48-51 (76 cells) — id `W5-2:41,48`
- S3: cols 197-207, rows 27-29 (33 cells) — id `W5-2:197,27`
- S4: cols 240-257, rows 16-18 (54 cells) — id `W5-2:240,16`
- S5: cols 247-257, rows 71-73 (33 cells) — id `W5-2:247,71`

### W5-3 (d2_c34_level)
- S1: cols 60-78, rows 72-75 (76 cells) — id `W5-3:60,72`

### W5-4 (d2_c35_level)
- S1: cols 68-75, rows 23-26 (32 cells) — id `W5-4:68,23`
- S2: cols 97-118, rows 69-71 (66 cells) — id `W5-4:97,69`

### W6-2 (d4_c03_level)
- S1: cols 47-59, rows 63-65 (39 cells) — id `W6-2:47,63`
- S2: cols 48-59, rows 6-8 (36 cells) — id `W6-2:48,6`
- S3: cols 83-84, rows 23-26 (8 cells) — id `W6-2:83,23`
- S4: cols 166-168, rows 3-6 (12 cells) — id `W6-2:166,3`
- S5: cols 240-251, rows 32-35 (48 cells) — id `W6-2:240,32`

### W6-3 (d4_c04_level)
- S1: cols 48-59, rows 24-27 (48 cells) — id `W6-3:48,24`
- S2: cols 119-121, rows 4-6 (9 cells) — id `W6-3:119,4`
- S3: cols 240-250, rows 4-7 (44 cells) — id `W6-3:240,4`
- S4: cols 240-249, rows 58-61 (40 cells) — id `W6-3:240,58`

### W6-4 (d4_c05_level)
- S1: cols 60-63, rows 34-37 (16 cells) — id `W6-4:60,34`
- S2: cols 111-112, rows 25-27 (6 cells) — id `W6-4:111,25`

### W6-6 (d4_c15_level)
- S1: cols 47-59, rows 63-65 (39 cells) — id `W6-6:47,63`
- S2: cols 48-59, rows 6-8 (36 cells) — id `W6-6:48,6`
- S3: cols 83-84, rows 23-26 (8 cells) — id `W6-6:83,23`
- S4: cols 166-168, rows 3-6 (12 cells) — id `W6-6:166,3`
- S5: cols 240-251, rows 32-35 (48 cells) — id `W6-6:240,32`

### W6-7 (d4_c17_level)
- S1: cols 60-63, rows 34-37 (16 cells) — id `W6-7:60,34`
- S2: cols 111-112, rows 25-27 (6 cells) — id `W6-7:111,25`