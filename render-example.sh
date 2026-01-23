#!/bin/bash
# Render the railway topology example with different stylesheets

set -e

echo "Rendering railway-topology with default stylesheet..."
cargo run -- examples/railway-topology.ail > /tmp/railway-default.svg
echo "  -> /tmp/railway-default.svg"

echo "Rendering railway-topology with Kapernikov stylesheet..."
cargo run -- --stylesheet examples/stylesheets/kapernikov.toml examples/railway-topology.ail > /tmp/railway-kapernikov.svg
echo "  -> /tmp/railway-kapernikov.svg"

echo ""
echo "Done! Open with:"
echo "  xdg-open /tmp/railway-default.svg"
echo "  xdg-open /tmp/railway-kapernikov.svg"
