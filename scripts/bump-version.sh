#!/usr/bin/env bash
# bump-version.sh — Update version across all project files atomically.
#
# Usage: ./scripts/bump-version.sh <version>
#   e.g.: ./scripts/bump-version.sh 0.3.0
#
# Updates:
#   1. Cargo.toml         — version = "x.y.z"
#   2. Cargo.lock          — regenerated via cargo check
#   3. .claude-plugin/plugin.json — "version": "x.y.z"
#   4. CHANGES.md          — adds ## [x.y.z] stub if not present

set -euo pipefail

# Portable in-place sed: BSD (macOS) requires -i '', GNU (Linux) requires -i alone.
sedi() {
    if sed --version 2>/dev/null | grep -q 'GNU'; then
        sed -i "$@"
    else
        sed -i '' "$@"
    fi
}

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 <version>" >&2
    echo "  e.g.: $0 0.3.0" >&2
    exit 1
fi

# Validate semver format (major.minor.patch, optional pre-release)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo "Error: '$VERSION' is not a valid semver (expected x.y.z)" >&2
    exit 1
fi

# Resolve project root (script lives in scripts/)
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

CHANGED=0

# 1. Cargo.toml
CARGO_TOML="$ROOT/Cargo.toml"
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [[ "$CURRENT" != "$VERSION" ]]; then
    sedi "s/^version = \".*\"/version = \"$VERSION\"/" "$CARGO_TOML"
    echo "Updated Cargo.toml: $CURRENT -> $VERSION"
    CHANGED=1
else
    echo "Cargo.toml: already $VERSION"
fi

# 2. plugin.json
PLUGIN_JSON="$ROOT/.claude-plugin/plugin.json"
if [[ -f "$PLUGIN_JSON" ]]; then
    PLUGIN_CURRENT=$(grep '"version"' "$PLUGIN_JSON" | sed 's/.*: *"\(.*\)".*/\1/')
    if [[ "$PLUGIN_CURRENT" != "$VERSION" ]]; then
        sedi "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" "$PLUGIN_JSON"
        echo "Updated plugin.json: $PLUGIN_CURRENT -> $VERSION"
        CHANGED=1
    else
        echo "plugin.json: already $VERSION"
    fi
else
    echo "Warning: $PLUGIN_JSON not found" >&2
fi

# 3. CHANGES.md — add stub if entry doesn't exist
CHANGES="$ROOT/CHANGES.md"
if [[ -f "$CHANGES" ]]; then
    if grep -q "## \[$VERSION\]" "$CHANGES"; then
        echo "CHANGES.md: entry for $VERSION already exists"
    else
        DATE=$(date +%Y-%m-%d)
        STUB="## [$VERSION] — $DATE

### Added

### Changed

### Fixed
"
        # Insert after the first line (title) using a temp file for portability.
        { head -1 "$CHANGES"; printf '\n%s\n' "$STUB"; tail -n +2 "$CHANGES"; } > "$CHANGES.tmp"
        mv "$CHANGES.tmp" "$CHANGES"
        echo "Updated CHANGES.md: added [$VERSION] stub"
        CHANGED=1
    fi
else
    echo "Warning: $CHANGES not found" >&2
fi

# 4. Cargo.lock — regenerate
if [[ $CHANGED -eq 1 ]]; then
    echo "Regenerating Cargo.lock..."
    (cd "$ROOT" && cargo check --quiet 2>/dev/null)
    echo "Cargo.lock regenerated"
else
    echo "Cargo.lock: no changes needed"
fi

if [[ $CHANGED -eq 0 ]]; then
    echo ""
    echo "All files already at version $VERSION — no changes made."
else
    echo ""
    echo "Version bumped to $VERSION across all files."
fi
