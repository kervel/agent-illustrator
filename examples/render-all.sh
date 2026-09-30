#!/usr/bin/env bash
# Re-render every committed example image listed in examples/renders.txt.
# Run after building a new version of agent-illustrator.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --quiet
BIN="$ROOT/target/debug/agent-illustrator"

while read -r out src flags; do
    case "$out" in ''|'#'*) continue ;; esac
    # shellcheck disable=SC2086
    if "$BIN" "$src" $flags > "$out.tmp" 2>/dev/null; then
        mv "$out.tmp" "$out"
        echo "OK  ${out#examples/}"
    else
        rm -f "$out.tmp"
        echo "FAIL $src ($flags)"
    fi
done < examples/renders.txt
