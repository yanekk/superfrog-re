"""Throwaway: items_left as load_list_world_N computes it, from the object maps alone.
h0+$7a9e counts bytes $20-$27 over all 38400 bytes at h0+$cb38; the load list subtracts $32 and
h4+$1c910 packs the three decimal digits as BCD."""
import subprocess, sys, pathlib
root = pathlib.Path(__file__).resolve().parents[2]
for n in range(1, 7):
    out = root / f"build/spike-items/_MapaBobow{n}"
    out.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([sys.executable, str(root / "tools/unpack.py"),
                    str(root / f"build/files/disk2/_MapaBobow{n}") if (root / f"build/files/disk2/_MapaBobow{n}").exists()
                    else str(root / f"build/files/disk1/_MapaBobow{n}"), str(out)], check=True, capture_output=True)
    b = out.read_bytes()
    count = sum(1 for x in b[:38400] if 0x20 <= x <= 0x27)
    v = count - 50
    bcd = ((v // 100) << 8) | (((v // 10) % 10) << 4) | (v % 10)
    print(f"W{n}-1 bytes={len(b)} coins={count} needed={v} bcd=${bcd:04x}")
