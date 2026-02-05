#!/bin/bash
# Render the railway topology example with different stylesheets
# Usage: ./render-example.sh [--debug]

set -e

OUTPUT_DIR="output"
mkdir -p "$OUTPUT_DIR"

DEBUG_FLAG=""
if [[ "$1" == "--debug" ]]; then
    DEBUG_FLAG="--debug"
    echo "Debug mode enabled"
fi

CSS_FLAG="--stylesheet-css stylesheets/kapernikov.css"

echo "Rendering railway-topology..."
cargo run -- $DEBUG_FLAG $CSS_FLAG examples/railway-topology.ail > "$OUTPUT_DIR/railway-topology.svg"
echo "  -> $OUTPUT_DIR/railway-topology.svg"

echo "Rendering railway-junction-direct..."
cargo run -- $DEBUG_FLAG $CSS_FLAG examples/railway-junction-direct.ail > "$OUTPUT_DIR/railway-junction-direct.svg"
echo "  -> $OUTPUT_DIR/railway-junction-direct.svg"

echo "Rendering label-test..."
cargo run -- $DEBUG_FLAG $CSS_FLAG examples/label-test.ail > "$OUTPUT_DIR/label-test.svg"
echo "  -> $OUTPUT_DIR/label-test.svg"

echo ""
echo "Done! SVGs are in $OUTPUT_DIR/"
