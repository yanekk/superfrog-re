#!/bin/sh
# Moves the owner's disk images (any *.zip or *.adf whose name holds "Disk N") from the main checkout
# root into games/superfrog/ there (monorepo-and-knowledge DESIGN §5.3). Idempotent: moves nothing and
# exits 0 when none are at the root. Way back: mv games/superfrog/*.zip to the root.
set -eu
common=$(git -C "$(dirname "$0")" rev-parse --path-format=absolute --git-common-dir)
main=$(dirname "$common")
dest="$main/games/superfrog"
moved=0
for f in "$main"/*.zip "$main"/*.adf; do
    [ -f "$f" ] || continue
    case "$(basename "$f")" in
        *"Disk "[0-9]*)
            mkdir -p "$dest"
            mv -n "$f" "$dest/"
            if [ -e "$f" ]; then
                echo "move-disks: $(basename "$f") is already in $dest; left at the root" >&2
            else
                echo "moved: $(basename "$f") -> $dest/"
                moved=$((moved + 1))
            fi ;;
    esac
done
[ "$moved" -gt 0 ] || echo "move-disks: no disks to move"
