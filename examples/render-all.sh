#!/usr/bin/env bash
# Re-render all example SVGs from their .ail sources.
# Run after building a new version of agent-illustrator.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
AI="${AI_BIN:-agent-illustrator}"

for ail in "$SCRIPT_DIR"/*.ail; do
    name="$(basename "$ail" .ail)"
    svg="$SCRIPT_DIR/$name.svg"
    if "$AI" "$ail" > "$svg" 2>/dev/null; then
        echo "OK  $name.svg"
    else
        echo "FAIL $name.ail (skipped)"
        rm -f "$svg"
    fi
done
