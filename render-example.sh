#!/bin/bash
# Render examples with Kapernikov stylesheets
# Usage: ./render-example.sh [--debug]

set -e

OUTPUT_DIR="output"
mkdir -p "$OUTPUT_DIR"

DEBUG_FLAG=""
if [[ "$1" == "--debug" ]]; then
    DEBUG_FLAG="--debug"
    echo "Debug mode enabled"
fi

CSS="--stylesheet-css stylesheets/kapernikov.css"
CSS_SCHEMATIC="--stylesheet-css stylesheets/kapernikov-schematic.css"

echo "Rendering railway-topology..."
cargo run -- $DEBUG_FLAG $CSS examples/railway-topology.ail > "$OUTPUT_DIR/railway-topology.svg"
echo "  -> $OUTPUT_DIR/railway-topology.svg"

echo "Rendering railway-junction-direct..."
cargo run -- $DEBUG_FLAG $CSS examples/railway-junction-direct.ail > "$OUTPUT_DIR/railway-junction-direct.svg"
echo "  -> $OUTPUT_DIR/railway-junction-direct.svg"

echo "Rendering label-test..."
cargo run -- $DEBUG_FLAG $CSS examples/label-test.ail > "$OUTPUT_DIR/label-test.svg"
echo "  -> $OUTPUT_DIR/label-test.svg"

echo "Rendering mosfet-driver (schematic)..."
cargo run -- $DEBUG_FLAG $CSS_SCHEMATIC examples/mosfet-driver.ail > "$OUTPUT_DIR/mosfet-driver.svg"
echo "  -> $OUTPUT_DIR/mosfet-driver.svg"

echo ""
echo "Done! SVGs are in $OUTPUT_DIR/"
