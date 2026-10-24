#!/bin/sh
# Runs the route-follower prototype at low priority (the smart-planner build shares the machine).
# Usage: experiments/route-follower/f.sh run W1-1 --exec B3 --cond P1 [...]
#        experiments/route-follower/f.sh many <file of argument lines>   (one run per line)
here=$(cd "$(dirname "$0")" && pwd)
bin="$here/../../engine/${FOLLOW_TARGET:-target}/release/examples/follow"
cd "$here/../../engine" || exit 1
if [ "$1" = many ]; then
  while IFS= read -r line; do
    case "$line" in ''|'#'*) continue ;; esac
    # shellcheck disable=SC2086
    nice -n 10 "$bin" $line
  done < "$2"
else
  exec nice -n 10 "$bin" "$@"
fi
