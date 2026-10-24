"""Writes batches/main.txt: every (level, executor, condition) not yet in results/runs.csv, cheap
executors first, then the determinism repeats. Run from experiments/route-follower."""
import csv
import os

done = set()
if os.path.exists('results/runs.csv'):
    done = {(r['level'], r['exec'], r['cond'], r['label']) for r in csv.DictReader(open('results/runs.csv'))}
levels = "W1-1 W1-4 W2-1 W3-2 W5-2 W6-1 W6-2 W4-3".split()
eng = "P1 P2 P3:1 P3:2 P3:5 P3:20 P4o P4e:200:2 P5:0 P5:2 P6e".split()
orc = ["P0 --verify", "P3o:5 --verify", "P5o:0", "P6 --verify"]
out = "--csv ../experiments/route-follower/results/runs.csv"
lines = []
for group in (["fixed", "A", "B1"], ["B2", "B23"], ["B3"]):
    for lv in levels:
        for ex in group:
            for c in eng + orc:
                if (lv, ex, c.split()[0], '') in done:
                    continue
                lines.append(f"run {lv} --exec {ex} --cond {c} {out} --threads 3")
for lv in ["W1-1", "W2-1", "W5-2"]:
    for ex in ["B2", "B23"]:
        if (lv, ex, 'P1', 'det-repeat') not in done:
            lines.append(f"run {lv} --exec {ex} --cond P1 {out} --threads 3 --label det-repeat")
        if (lv, ex, 'P1', 'det-threads1') not in done:
            lines.append(f"run {lv} --exec {ex} --cond P1 {out} --threads 1 --label det-threads1")
open("batches/main.txt", "w").write("\n".join(lines) + "\n")
print(len(lines), "runs")
