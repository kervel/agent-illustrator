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

echo "Rendering railway-topology with default stylesheet..."
cargo run -- $DEBUG_FLAG examples/railway-topology.ail > "$OUTPUT_DIR/railway-default.svg"
echo "  -> $OUTPUT_DIR/railway-default.svg"

echo "Rendering railway-topology with Kapernikov stylesheet..."
cargo run -- $DEBUG_FLAG --stylesheet examples/stylesheets/kapernikov.toml examples/railway-topology.ail > "$OUTPUT_DIR/railway-kapernikov.svg"
echo "  -> $OUTPUT_DIR/railway-kapernikov.svg"

echo "Rendering railway-junction-direct..."
cargo run -- $DEBUG_FLAG examples/railway-junction-direct.ail > "$OUTPUT_DIR/railway-junction-direct.svg"
echo "  -> $OUTPUT_DIR/railway-junction-direct.svg"

echo "Rendering label-test..."
cargo run -- $DEBUG_FLAG examples/label-test.ail > "$OUTPUT_DIR/label-test.svg"
echo "  -> $OUTPUT_DIR/label-test.svg"

echo ""
echo "Done! SVGs are in $OUTPUT_DIR/"
