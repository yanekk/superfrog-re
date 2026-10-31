#!/bin/sh
# Forest Dumb Forever's test command. Anchors itself, so any caller's cwd holds. Shaped like the first
# game's: exit 5 from unittest discover means no tests found, which is not a failure.
cd "$(dirname "$0")" || exit 1
# tools/full.sh sets this; a plain run leaves the #[ignore]d tests out (smoke-tests DESIGN §2.1).
set --
if [ "${PIR_TEST_INCLUDE_IGNORED:-}" = 1 ]; then set -- -- --include-ignored; fi
failed=0
(cd engine && cargo test -q --workspace "$@") || failed=1
NO_COLOR=1 ../../venv/bin/python -m unittest discover -q -s tools -p 'test_*.py' || [ $? -eq 5 ] || failed=1
exit $failed
