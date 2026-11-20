#!/bin/sh
# The toolkit's test command, run from the repo root (monorepo-and-knowledge DESIGN §2.4).
# tools/full.sh sets this; a plain run leaves the #[ignore]d tests out (smoke-tests DESIGN §2.1).
set --
# --no-fail-fast: cargo otherwise stops at the first failing test binary and the rest never run.
if [ "${PIR_TEST_INCLUDE_IGNORED:-}" = 1 ]; then set -- --no-fail-fast -- --include-ignored; fi
failed=0
(cd toolkit && cargo test -q --workspace "$@") || failed=1
NO_COLOR=1 venv/bin/python -m unittest discover -q -s toolkit/tools -p 'test_*.py' || [ $? -eq 5 ] || failed=1
venv/bin/python tools/check_tier_names.py || failed=1
exit $failed
