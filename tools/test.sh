#!/bin/sh
# The repo's one test command (monorepo-and-knowledge DESIGN §2.4): runs the suites this branch's
# diff against main needs, from the repo root. tools/select_scope.py decides; any doubt runs everything.
# PIR_TEST_SCOPE=all runs everything. One printed line per suite; any failure fails the whole.
# --no-renames: a rename otherwise lists only its new path, so a file moved out of a project would not
# run that project's suite.
cd "$(dirname "$0")/.." || exit 1

games=$(cd games && for d in */; do printf '%s ' "${d%/}"; done)
for g in $games; do
    if [ ! -f "games/$g/test.sh" ]; then
        echo "test.sh: games/$g has no test.sh; a skipped project is a suite nobody knows is gone" >&2
        exit 1
    fi
done

if base=$(git merge-base HEAD main 2>/dev/null) && changed=$(git diff --name-only --no-renames "$base"..HEAD 2>/dev/null); then
    :
else
    changed=DOUBT
fi
# shellcheck disable=SC2086
choice=$(printf '%s\n' "$changed" | venv/bin/python tools/select_scope.py "${PIR_TEST_SCOPE:-}" $games) || {
    echo "test.sh: the selector failed; running everything" >&2
    choice=EVERYTHING
}

failed=0
suite() { # name, command...
    name=$1; shift
    echo "test.sh: $name"
    "$@" || { echo "test.sh: $name FAILED" >&2; failed=1; }
}
root_checks() {
    NO_COLOR=1 venv/bin/python -m unittest discover -q -s tools -p 'test_*.py' || [ $? -eq 5 ]
}

case "$choice" in
    NOTHING)
        echo "test.sh: plans only, nothing to run"
        exit 0 ;;
    EVERYTHING)
        suite toolkit sh toolkit/test.sh
        for g in $games; do suite "$g" sh "games/$g/test.sh"; done
        suite "root checks" root_checks ;;
    *)
        # toolkit sorts after games/, but it runs first as in the everything order.
        case " $choice " in *" toolkit "*) suite toolkit sh toolkit/test.sh ;; esac
        for p in $choice; do
            case "$p" in games/*) suite "${p#games/}" sh "$p/test.sh" ;; esac
        done ;;
esac
exit $failed
