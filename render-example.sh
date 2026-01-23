#!/bin/bash
# Render the railway topology example with different stylesheets

set -e

OUTPUT_DIR="output"
mkdir -p "$OUTPUT_DIR"

echo "Rendering railway-topology with default stylesheet..."
cargo run -- examples/railway-topology.ail > "$OUTPUT_DIR/railway-default.svg"
echo "  -> $OUTPUT_DIR/railway-default.svg"

echo "Rendering railway-topology with Kapernikov stylesheet..."
cargo run -- --stylesheet examples/stylesheets/kapernikov.toml examples/railway-topology.ail > "$OUTPUT_DIR/railway-kapernikov.svg"
echo "  -> $OUTPUT_DIR/railway-kapernikov.svg"

echo "Rendering railway-junction-direct..."
cargo run -- examples/railway-junction-direct.ail > "$OUTPUT_DIR/railway-junction-direct.svg"
echo "  -> $OUTPUT_DIR/railway-junction-direct.svg"

echo "Rendering label-test..."
cargo run -- examples/label-test.ail > "$OUTPUT_DIR/label-test.svg"
echo "  -> $OUTPUT_DIR/label-test.svg"

echo ""
echo "Done! SVGs are in $OUTPUT_DIR/"
