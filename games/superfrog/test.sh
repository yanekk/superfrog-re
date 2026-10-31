#!/bin/sh
# Superfrog's test command (monorepo-and-knowledge DESIGN §2.4). Anchors itself, so any caller's cwd holds.
cd "$(dirname "$0")" || exit 1
# tools/full.sh sets this; a plain run leaves the #[ignore]d tests out (smoke-tests DESIGN §2.1).
set --
if [ "${PIR_TEST_INCLUDE_IGNORED:-}" = 1 ]; then
    set -- -- --include-ignored
    # The screen tests panic without SF_E2E_OS=1 and post real input to the screen with it, so a run
    # without it leaves them out. Found from their #[ignore] reason, so a new one needs no list.
    # --exact because --skip otherwise matches any test whose name contains the skipped one; it also
    # means a screen test inside a module would need its path here, and would fail loudly, not vanish.
    if [ "${SF_E2E_OS:-}" != 1 ]; then
        set -- "$@" --exact
        # The names hold no spaces, so the unquoted expansion is safe.
        for name in $(find engine/crates -name '*.rs' -exec awk '
            FNR == 1 { screen = 0 }
            /^[ \t]*#\[ignore[ \t]*=.*SF_E2E_OS=1/ { screen = 1; next }
            screen && /^[ \t]*(pub[ \t]+)?(async[ \t]+)?fn[ \t]/ {
                sub(/^[ \t]*(pub[ \t]+)?(async[ \t]+)?fn[ \t]+/, ""); sub(/[^A-Za-z0-9_].*/, ""); print; screen = 0
            }' {} +); do
            set -- "$@" --skip "$name"
        done
    fi
fi
failed=0
(cd engine && cargo test -q --workspace "$@") || failed=1
NO_COLOR=1 ../../venv/bin/python -m unittest discover -q -s tools -p 'test_*.py' || [ $? -eq 5 ] || failed=1
exit $failed
