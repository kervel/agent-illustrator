#!/usr/bin/env bash
# Re-render all example SVGs from their .ail sources.
# Run after building a new version of agent-illustrator.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

stylesheet_for() {
    case "$1" in
        mosfet-driver) echo "$SCRIPT_DIR/../stylesheets/kapernikov-schematic.css" ;;
        *)             echo "$SCRIPT_DIR/../stylesheets/kapernikov.css" ;;
    esac
}

for ail in "$SCRIPT_DIR"/*.ail; do
    name="$(basename "$ail" .ail)"
    svg="$SCRIPT_DIR/$name.svg"
    css="$(stylesheet_for "$name")"
    if cargo run -- "$ail" --stylesheet-css "$css" > "$svg" 2>/dev/null; then
        echo "OK  $name.svg"
    else
        echo "FAIL $name.ail (skipped)"
        rm -f "$svg"
    fi
done
