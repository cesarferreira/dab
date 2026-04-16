#!/usr/bin/env bash
# Install the dab SKILL.md into common AI agent skill directories.
# Supports: Cursor (~/.cursor/skills/), Claude Code (~/.claude/skills/),
#           Codex (~/.codex/skills/), and custom paths via --dest.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_SRC="$SCRIPT_DIR/../SKILL.md"
SKILL_NAME="dab"

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Install the dab SKILL.md into AI agent skill directories.

Options:
  --dest <DIR>   Install into a custom directory instead of the defaults
  --dry-run      Show what would be done without making changes
  --help         Show this help message

Default install locations:
  ~/.cursor/skills/$SKILL_NAME/SKILL.md       (Cursor)
  ~/.claude/skills/$SKILL_NAME/SKILL.md       (Claude Code)
  ~/.codex/skills/$SKILL_NAME/SKILL.md        (Codex)
EOF
}

DRY_RUN=false
CUSTOM_DEST=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dest)    CUSTOM_DEST="$2"; shift 2 ;;
    --dry-run) DRY_RUN=true; shift ;;
    --help)    usage; exit 0 ;;
    *)         echo "Unknown option: $1"; usage; exit 1 ;;
  esac
done

if [[ ! -f "$SKILL_SRC" ]]; then
  echo "Error: SKILL.md not found at $SKILL_SRC"
  exit 1
fi

install_skill() {
  local dest_dir="$1/$SKILL_NAME"
  local dest_file="$dest_dir/SKILL.md"

  if $DRY_RUN; then
    echo "[dry-run] Would install to: $dest_file"
    return
  fi

  mkdir -p "$dest_dir"
  cp "$SKILL_SRC" "$dest_file"
  echo "Installed: $dest_file"
}

if [[ -n "$CUSTOM_DEST" ]]; then
  install_skill "$CUSTOM_DEST"
else
  INSTALLED=0
  for base in \
    "$HOME/.cursor/skills" \
    "$HOME/.cursor/skills-cursor" \
    "$HOME/.claude/skills" \
    "$HOME/.codex/skills" \
    "$HOME/.agents/skills"
  do
    if [[ -d "$base" ]]; then
      install_skill "$base"
      INSTALLED=$((INSTALLED + 1))
    fi
  done

  if [[ $INSTALLED -eq 0 ]]; then
    echo "No known skill directories found."
    echo "Run with --dest <DIR> to specify a custom location, e.g.:"
    echo "  $0 --dest ~/.cursor/skills"
  fi
fi
