#!/usr/bin/env bash
set -euo pipefail

REPO="kervel/agent-illustrator"
FLAKE="flake.nix"
PLATFORMS=("x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin")
ARTIFACTS=("agent-illustrator-linux-x86_64" "agent-illustrator-linux-aarch64" "agent-illustrator-macos-x86_64" "agent-illustrator-macos-aarch64")

usage() {
    echo "Usage: $0 [--release]"
    echo ""
    echo "Update flake.nix to match the latest GitHub release."
    echo ""
    echo "Options:"
    echo "  --release    Also create a new git tag from HEAD, push it,"
    echo "               wait for CI, then update the flake."
    echo "  --help       Show this help message."
    echo ""
    echo "Without --release, updates flake.nix to whatever the latest"
    echo "GitHub release already is."
}

get_latest_release() {
    gh release view --repo "$REPO" --json tagName -q .tagName
}

wait_for_ci() {
    local tag="$1"
    echo "Waiting for GitHub Actions release workflow for $tag..."

    # Find the run for this tag
    local run_id
    for i in $(seq 1 30); do
        run_id=$(gh run list --repo "$REPO" --limit 5 --json databaseId,headBranch \
            -q ".[] | select(.headBranch == \"$tag\") | .databaseId" 2>/dev/null | head -1)
        if [[ -n "$run_id" ]]; then
            break
        fi
        echo "  Waiting for workflow to appear..."
        sleep 5
    done

    if [[ -z "$run_id" ]]; then
        echo "ERROR: Could not find GitHub Actions run for $tag after 150s" >&2
        exit 1
    fi

    echo "  Found run $run_id, waiting for completion..."
    gh run watch "$run_id" --repo "$REPO" --exit-status
    echo "  CI passed."
}

bump_version() {
    local current="$1"
    # Strip leading 'v' if present
    local ver="${current#v}"
    local major minor patch
    IFS='.' read -r major minor patch <<< "$ver"
    patch=$((patch + 1))
    echo "v${major}.${minor}.${patch}"
}

prefetch_hash() {
    local url="$1"
    local base32
    base32=$(nix-prefetch-url "$url" 2>/dev/null)
    # Convert to SRI hash
    nix --extra-experimental-features nix-command hash to-sri --type sha256 "$base32" 2>/dev/null \
        | grep -o 'sha256-.*'
}

update_flake() {
    local version="$1"
    local ver="${version#v}"

    echo "Fetching release artifact hashes for $version..."

    declare -A hashes
    for i in "${!PLATFORMS[@]}"; do
        local platform="${PLATFORMS[$i]}"
        local artifact="${ARTIFACTS[$i]}"
        local url="https://github.com/$REPO/releases/download/$version/$artifact"
        echo "  Fetching $artifact..."
        hashes[$platform]=$(prefetch_hash "$url")
        echo "    ${hashes[$platform]}"
    done

    echo "Updating $FLAKE to $ver..."

    # Update version
    sed -i "s/version = \"[0-9.]*\";/version = \"$ver\";/" "$FLAKE"

    # Update hash comment
    sed -i "s/# Hashes for v[0-9.]* release binaries/# Hashes for $version release binaries/" "$FLAKE"

    # Update each platform hash
    for platform in "${PLATFORMS[@]}"; do
        local old_pattern="\"$platform\" = \"sha256-[^\"]*\""
        local new_value="\"$platform\" = \"${hashes[$platform]}\""
        sed -i "s|$old_pattern|$new_value|" "$FLAKE"
    done

    echo "Done. Updated $FLAKE to $version."
}

do_release() {
    local current
    current=$(get_latest_release)
    local next
    next=$(bump_version "$current")

    echo "Current release: $current"
    echo "Next release:    $next"
    echo ""

    # Check for uncommitted changes
    if ! git diff --quiet; then
        echo "ERROR: You have uncommitted changes. Commit first." >&2
        exit 1
    fi

    # Create and push tag
    echo "Creating tag $next..."
    git tag -a "$next" -m "$next"
    git push origin "$next" 2>/dev/null || true
    git push github "$next"

    # Wait for CI
    wait_for_ci "$next"

    # Update flake
    update_flake "$next"

    # Commit and push flake update
    git add "$FLAKE"
    git commit -m "chore: Update flake.nix to $next release"
    git push origin main 2>/dev/null || true
    git push github main

    echo ""
    echo "Released $next and updated flake."
}

do_update() {
    local latest
    latest=$(get_latest_release)
    echo "Latest release: $latest"
    update_flake "$latest"
}

case "${1:-}" in
    --release)
        do_release
        ;;
    --help|-h)
        usage
        ;;
    "")
        do_update
        ;;
    *)
        echo "Unknown option: $1" >&2
        usage >&2
        exit 1
        ;;
esac
