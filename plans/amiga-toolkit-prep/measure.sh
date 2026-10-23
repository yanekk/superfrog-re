#!/bin/sh
# North-star measured questions (NORTH-STAR.md). Run from the repo root: sh plans/amiga-toolkit-prep/measure.sh
# Cheap: rg and cargo metadata only. M1 (the checks) is run separately; see NORTH-STAR.md.
set -u
core=engine/crates/sf-core/src

echo "## M2 duplicates left"
echo "kbd_poll definitions:          $(rg -c 'fn kbd_poll\b' $core | awk -F: '{s+=$2} END{print s+0}')"
# Code lines only: the encoding also appears in a doc comment.
echo "key-byte encode sites:         $(rg -n '!\([^)]*<< 1 \|' $core | rg -v '^\S+:\d+:\s*//' | wc -l | tr -d ' ')"
echo "blitter memory-effect impls:   $(rg -n '^\s*pub fn run<M: BlitMem>|^fn blit\(|pub\(crate\) fn blit\(|fn blit_panel' engine/crates --type rust | wc -l | tr -d ' ')"
echo "BLTSIZE decodes (size >> 6):   $(rg -n 'size >> 6' engine/crates --type rust | wc -l | tr -d ' ')"
echo "chip-memory containers:        $(rg -n 'pub struct AmigaMemory|pub struct StoryMem' engine/crates --type rust | wc -l | tr -d ' ')"
echo "is_paula definitions:          $(rg -n 'fn is_paula' engine/crates --type rust | wc -l | tr -d ' ')"
echo "frame/line length constants:   $(rg -n 'const (FRAME_LINES|LINE_CYCLES|BEAM_CYCLES_PER_LINE|LINE)\b\s*:' engine/crates --type rust | wc -l | tr -d ' ')"
echo "beam-position types/functions: $(rg -n 'pub struct Clock|pub struct Beam\b|struct Cx\b|struct Cx<|^fn spend|pub fn beam\(' $core | wc -l | tr -d ' ')"
echo "hunk-0 loads in sf-core:       $(rg -n 'loaded_hunk0\(game\)|relocate\(g, 0|SoundData::from_game\(' $core | wc -l | tr -d ' ')"

echo "## M3 address reads of hunk 0 through front/sound readers"
echo "h0_* reads:                    $(rg -o '\.h0_(b|w|l|bytes|byte|word)\(' $core | wc -l | tr -d ' ')"

echo "## M4/M5 toolkit-bound crates (no sf-* dependency, no Superfrog name outside comments)"
for dir in engine/crates/*/; do
  c=$(basename "$dir")
  deps=$(rg -c '^\s*sf-[a-z-]+\s*=|^\s*sf-[a-z-]+\.workspace' "$dir/Cargo.toml" 2>/dev/null || echo 0)
  names=$(rg -i -n 'superfrog|sf_core|sf_assets' "$dir/src" 2>/dev/null | rg -v '^\S+:\d+:\s*//' | wc -l | tr -d ' ')
  lines=$(find "$dir/src" -name '*.rs' -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
  if [ "$deps" = "0" ] && [ "$names" = "0" ]; then
    echo "PASS $c ($lines lines)"
  else
    echo "     $c: sf-deps=$deps superfrog-names=$names"
  fi
done
