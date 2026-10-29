#!/bin/sh
# The vAmiga build moved to the toolkit (toolkit-extract T05); this path stays because setup blocks,
# docs and the owner run it. cmake comes from this repo's brew deps if missing.
command -v cmake >/dev/null 2>&1 || [ -x /opt/homebrew/bin/cmake ] || "$(dirname "$0")/install-brew-deps.sh"
exec "$(dirname "$0")/../../toolkit/tools/setup/oracle.sh" "$@"
