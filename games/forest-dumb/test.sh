#!/bin/sh
# Forest Dumb Forever's test command. Anchors itself, so any caller's cwd holds.
cd "$(dirname "$0")" || exit 1
(cd engine && cargo test -q --workspace) || exit 1
NO_COLOR=1 ../../venv/bin/python -m unittest discover -q -s tools -p 'test_*.py'
