#!/bin/sh
# Forest Dumb Forever's test command. Anchors itself, so any caller's cwd holds. The Python tools line
# arrives with the first Python tool.
cd "$(dirname "$0")" || exit 1
(cd engine && cargo test -q --workspace)
