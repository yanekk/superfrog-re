#!/bin/sh
# Superfrog's test command (monorepo-and-knowledge DESIGN §2.4). Anchors itself, so any caller's cwd holds.
cd "$(dirname "$0")" || exit 1
# tools/full.sh sets this; a plain run leaves the #[ignore]d tests out (smoke-tests DESIGN §2.1).
set --
if [ "${PIR_TEST_INCLUDE_IGNORED:-}" = 1 ]; then
    set -- -- --include-ignored
    # The screen tests panic without SF_E2E_OS=1 and post real input to the screen with it, so a run
    # without it leaves them out. The zips tool panics without SF_T14_ZIPS and checks nothing, so a run
    # without that leaves it out (owner, T04 review). Found from the #[ignore] reason, so no list.
    # --exact because --skip otherwise matches any test whose name contains the skipped one; it also
    # means a test inside a module would need its path here, and would fail loudly, not vanish.
    reasons=
    if [ "${SF_E2E_OS:-}" != 1 ]; then reasons='SF_E2E_OS=1'; fi
    if [ -z "${SF_T14_ZIPS:-}" ]; then reasons="${reasons:+$reasons|}SF_T14_ZIPS"; fi
    # The names hold no spaces, so the unquoted expansion is safe.
    for name in $(find engine/crates -name '*.rs' -exec awk -v reasons="$reasons" '
        FNR == 1 { left_out = 0 }
        /^[ \t]*#\[ignore[ \t]*=/ && reasons != "" && $0 ~ reasons { left_out = 1; next }
        left_out && /^[ \t]*(pub[ \t]+)?(async[ \t]+)?fn[ \t]/ {
            sub(/^[ \t]*(pub[ \t]+)?(async[ \t]+)?fn[ \t]+/, ""); sub(/[^A-Za-z0-9_].*/, ""); print; left_out = 0
        }' {} +); do
        case " $* " in *" --exact "*) ;; *) set -- "$@" --exact ;; esac
        set -- "$@" --skip "$name"
    done
fi
failed=0
(cd engine && cargo test -q --workspace "$@") || failed=1
NO_COLOR=1 ../../venv/bin/python -m unittest discover -q -s tools -p 'test_*.py' || [ $? -eq 5 ] || failed=1
exit $failed
