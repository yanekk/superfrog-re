#!/bin/sh
# The smoke command (smoke-tests DESIGN §2.1): checks the labels, then runs the smoke set under its clock.
# exec, so that the runner is the process pir's kill of this line's process group reaches.
cd "$(dirname "$0")/.." || exit 1
exec venv/bin/python tools/smoke.py "$@"
