#!/bin/sh
# Superfrog's test command (monorepo-and-knowledge DESIGN §2.4). Anchors itself, so any caller's cwd holds.
cd "$(dirname "$0")" || exit 1
failed=0
(cd engine && cargo test -q --workspace) || failed=1
NO_COLOR=1 ../../venv/bin/python -m unittest discover -q -s tools -p 'test_*.py' || [ $? -eq 5 ] || failed=1
exit $failed
