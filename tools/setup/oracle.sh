#!/bin/sh
# Fetch vAmiga at the pinned commit, apply sf-oracle's core patches and build the core libraries into
# the shared tool cache, so every worktree links the same build (DESIGN §3.5, §5.3 row "Fetch and
# build vAmiga at the pinned commit", bin: worker). Way back: rm -rf ~/.cache/superfrog-re/vamiga
#
# Layout: $CACHE/vamiga/<commit>/<patch-set hash>/{src,build,shim-flags.txt,libs.txt,.done}. The
# patch-set hash is the SHA-1 of engine/crates/sf-oracle/patches/*.patch concatenated in name order
# (sf-oracle's build.rs computes the same one), so changing a patch builds afresh beside the old one.
#
# No-op when the build is there. Parallel worktrees build once: a lock directory holds the builder's
# PID, the others wait for it, and a lock left by a dead process is taken over. Writes nothing in the
# repository. Overrides (for tests): SUPERFROG_CACHE (default ~/.cache/superfrog-re), VAMIGA_URL.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
ORACLE="$ROOT/engine/crates/sf-oracle"
COMMIT=$(tr -d ' \n' < "$ORACLE/VAMIGA_COMMIT")
CACHE=${SUPERFROG_CACHE:-$HOME/.cache/superfrog-re}
URL=${VAMIGA_URL:-https://github.com/dirkwhoffmann/vAmiga.git}

PATCHES=$(ls "$ORACLE"/patches/*.patch 2>/dev/null | LC_ALL=C sort || true)
if [ -n "$PATCHES" ]; then
    # shellcheck disable=SC2086
    PHASH=$(cat $PATCHES | shasum -a 1 | cut -c1-12)
else
    PHASH=$(printf '' | shasum -a 1 | cut -c1-12)
fi
DEST="$CACHE/vamiga/$COMMIT/$PHASH"
LOCK="$DEST.lock"

say() { echo "oracle.sh: $*"; }

if [ -f "$DEST/.done" ]; then
    say "vAmiga ${COMMIT%"${COMMIT#????????}"} (patches $PHASH) already built in $DEST"
    exit 0
fi

mkdir -p "$(dirname "$DEST")"

# Take the lock, or wait for whoever holds it; a lock whose PID is dead is stale and taken over.
waited=0
while ! mkdir "$LOCK" 2>/dev/null; do
    holder=$(cat "$LOCK/pid" 2>/dev/null || true)
    if [ -n "$holder" ] && ! kill -0 "$holder" 2>/dev/null; then
        say "taking over a stale lock left by PID $holder"
        rm -rf "$LOCK"
        continue
    fi
    if [ -f "$DEST/.done" ]; then
        say "built by PID ${holder:-?} while waiting"
        exit 0
    fi
    if [ "$waited" -ge 3600 ]; then
        say "gave up after waiting ${waited}s for PID ${holder:-?} holding $LOCK" >&2
        exit 1
    fi
    [ $((waited % 60)) -eq 0 ] && say "waiting for PID ${holder:-?} to finish building vAmiga"
    sleep 2
    waited=$((waited + 2))
done
echo $$ > "$LOCK/pid"
trap 'rm -rf "$LOCK" "$DEST.tmp"' EXIT INT TERM

if [ -f "$DEST/.done" ]; then
    say "built by another worktree"
    exit 0
fi

command -v cmake >/dev/null 2>&1 || "$ROOT/tools/setup/install-brew-deps.sh"
CMAKE=$(command -v cmake || echo /opt/homebrew/bin/cmake)

rm -rf "$DEST" "$DEST.tmp"
mkdir -p "$DEST.tmp/src"
say "fetching vAmiga $COMMIT"
(
    cd "$DEST.tmp/src"
    git init -q
    git remote add origin "$URL"
    git fetch -q --depth 1 origin "$COMMIT"
    git -c advice.detachedHead=false checkout -q FETCH_HEAD
    for p in $PATCHES; do
        say "applying $(basename "$p")"
        git apply "$p"
    done
)

say "building the core (several minutes)"
"$CMAKE" -S "$DEST.tmp/src/Core" -B "$DEST.tmp/build" -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_EXPORT_COMPILE_COMMANDS=ON >/dev/null
"$CMAKE" --build "$DEST.tmp/build" --target VACore -j "$(sysctl -n hw.ncpu 2>/dev/null || echo 4)" >/dev/null

# What a client compiling against the core needs: VAmiga.cpp's include paths, defines and language
# standard, and every static library the core build made.
python3 - "$DEST.tmp/build/compile_commands.json" "$DEST.tmp/shim-flags.txt" <<'PY'
import json, shlex, sys
cmds = json.load(open(sys.argv[1]))
entry = next(c for c in cmds if c["file"].endswith("/Core/VAmiga.cpp"))
args = entry.get("arguments") or shlex.split(entry["command"])
keep, i = [], 0
while i < len(args):
    a = args[i]
    if a in ("-I", "-isystem", "-D"):
        keep += [a + args[i + 1]] if a != "-isystem" else ["-isystem", args[i + 1]]
        i += 2
        continue
    if a.startswith(("-I", "-D", "-std=")):
        keep.append(a)
    i += 1
open(sys.argv[2], "w").write("\n".join(keep) + "\n")
PY
find "$DEST.tmp/build" -name '*.a' | LC_ALL=C sort > "$DEST.tmp/libs.txt"
# The paths recorded so far point into $DEST.tmp; the build is about to move to $DEST.
sed -i '' "s#$DEST.tmp#$DEST#g" "$DEST.tmp/shim-flags.txt" "$DEST.tmp/libs.txt"
grep -q 'libVACore.a' "$DEST.tmp/libs.txt" || { say "the build made no libVACore.a" >&2; exit 1; }

echo "$COMMIT $PHASH" > "$DEST.tmp/.done"
mv "$DEST.tmp" "$DEST"
say "built vAmiga into $DEST"
