"""First frame two input scripts differ: python3 firstdiff.py a.txt b.txt"""
import sys


def load(p):
    ch = []
    for l in open(p):
        l = l.strip()
        if not l or l.startswith('#'):
            continue
        f, v = l.split(':', 1)
        ch.append((int(f), v.strip()))
    out = []
    for i, (f, v) in enumerate(ch):
        nf = ch[i + 1][0] if i + 1 < len(ch) else f
        out += [v] * (nf - f)
    return out


a, b = load(sys.argv[1]), load(sys.argv[2])
for k in range(min(len(a), len(b))):
    if a[k] != b[k]:
        print("first diff at", k)
        print(" a", a[max(0, k - 4):k + 6])
        print(" b", b[max(0, k - 4):k + 6])
        break
else:
    print("same over", min(len(a), len(b)), "frames")
