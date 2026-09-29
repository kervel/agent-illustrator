#!/usr/bin/env bash
# Re-render all example SVGs from their .ail sources.
# Run after building a new version of agent-illustrator.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

stylesheet_for() {
    case "$1" in
        mosfet-driver)        echo "$SCRIPT_DIR/../stylesheets/kapernikov-schematic.css" ;;
        agentic-loop-story)   echo "$SCRIPT_DIR/../stylesheets/agentic-loop-story.css" ;;
        *)                    echo "$SCRIPT_DIR/../stylesheets/kapernikov.css" ;;
    esac
}

extra_flags_for() {
    case "$1" in
        agentic-loop-story)   echo "--animate-css" ;;
        token-prediction)     echo "--animate-css" ;;
        *)                    echo "" ;;
    esac
}

for ail in "$SCRIPT_DIR"/*.ail; do
    name="$(basename "$ail" .ail)"
    svg="$SCRIPT_DIR/$name.svg"
    css="$(stylesheet_for "$name")"
    flags="$(extra_flags_for "$name")"
    # shellcheck disable=SC2086
    if cargo run -- "$ail" --stylesheet-css "$css" $flags > "$svg" 2>/dev/null; then
        echo "OK  $name.svg"
    else
        echo "FAIL $name.ail (skipped)"
        rm -f "$svg"
    fi
done

# Motion scenes (the git deck): their own stylesheet, animated.
for ail in "$SCRIPT_DIR"/motion/git-*.ail; do
    name="$(basename "$ail" .ail)"
    [ "$name" = "git-deck" ] && continue
    svg="$SCRIPT_DIR/motion/$name.svg"
    if cargo run -- "$ail" --stylesheet-css "$SCRIPT_DIR/motion/git-deck.css" --animate > "$svg" 2>/dev/null; then
        echo "OK  motion/$name.svg"
    else
        echo "FAIL motion/$name.ail (skipped)"
        rm -f "$svg"
    fi
done
