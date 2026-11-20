#!/bin/bash
# Empty this machine's Claude memory directory for the repo to a pointer (monorepo-and-knowledge
# DESIGN §5.2, §5.3). The check is the seatbelt: nothing is deleted until every one of the twelve
# files is held by a note committed in the repo, and the pointer is written last.
# Idempotent: a second run finds no files and writes the same pointer.
#
# MEMORY_DIR overrides the directory; REPO_ROOT overrides where the notes are looked up.
set -eu

root=${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}
if [ -z "${MEMORY_DIR:-}" ]; then
  # Claude's project folder is the main checkout's path with / and . turned into -; a worktree
  # shares it, so the path comes from the git common dir and not from this checkout.
  main=$(cd "$(git -C "$root" rev-parse --path-format=absolute --git-common-dir)/.." && pwd)
  MEMORY_DIR="$HOME/.claude/projects/$(printf %s "$main" | tr '/.' '--')/memory"
fi
[ -d "$MEMORY_DIR" ] || { echo "empty_memory: no such directory: $MEMORY_DIR" >&2; exit 1; }

NOTES="knowledge/self-testing-faithful-bar.md
knowledge/owner-controls.md
knowledge/pir-midrun-amendment.md
toolkit/docs/reuse-for-another-game.md
toolkit/docs/explorer-go-explore-idea.md
games/superfrog/docs/atn-decompression.md
games/superfrog/docs/t7mp-level-format.md
games/superfrog/docs/collision-and-secrets.md
games/superfrog/docs/milestone-2-bug-recorder.md
games/superfrog/docs/next-plan-difficulty-modes.md
games/superfrog/docs/video-ghost-trial.md
games/superfrog/docs/planner-stall-ask-owner.md"

# A moved note is corrected and reworded, so it cannot be compared byte for byte; a note holding
# most of the memory's longer words is the same note, and a stub or the wrong file is not.
MIN_SHARED_PERCENT=60

words() {
  awk 'NR==1 && $0=="---" {f=1; next} f && $0=="---" {f=0; next} !f' "$1" |
    tr -cs 'A-Za-z0-9' '\n' | tr 'A-Z' 'a-z' | awk 'length >= 5' | LC_ALL=C sort -u
}

fail=0
known=" MEMORY.md "
for note in $NOTES; do
  name=$(basename "$note")
  known="$known$name "
  memory="$MEMORY_DIR/$name"
  [ -e "$memory" ] || continue
  if ! git -C "$root" ls-files --error-unmatch -- "$note" >/dev/null 2>&1; then
    echo "empty_memory: $name: the repo does not hold $note (missing or not committed)" >&2
    fail=1
    continue
  fi
  if ! git -C "$root" diff --quiet HEAD -- "$note"; then
    echo "empty_memory: $name: $note has uncommitted changes" >&2
    fail=1
    continue
  fi
  total=$(words "$memory" | wc -l)
  shared=$(LC_ALL=C comm -12 <(words "$memory") <(words "$root/$note") | wc -l)
  if [ $((shared * 100)) -lt $((total * MIN_SHARED_PERCENT)) ]; then
    echo "empty_memory: $name: $note holds only $shared of its $total words" >&2
    fail=1
  fi
done

# Another session may have written a new memory since; rewriting the index would orphan it.
for f in "$MEMORY_DIR"/*; do
  [ -e "$f" ] || continue
  case "$known" in
    *" $(basename "$f") "*) ;;
    *) echo "empty_memory: $(basename "$f"): not one of the twelve; move it to the repo first" >&2
       fail=1 ;;
  esac
done

[ "$fail" -eq 0 ] || { echo "empty_memory: nothing deleted" >&2; exit 1; }

# Removed by bare name from inside the directory, so no value of a variable can aim this elsewhere.
cd "$MEMORY_DIR"
for note in $NOTES; do
  rm -f -- "${note##*/}"
done

cat >MEMORY.md <<'POINTER'
- Knowledge lives in the repo — since 2026-10-30 every note is in the repo at its tier: knowledge/ (index knowledge/KNOWLEDGE.md), toolkit/docs/, games/<game>/docs/. Keep here only what is about this machine or this conversation.
POINTER
echo "empty_memory: $MEMORY_DIR now holds only the pointer"
