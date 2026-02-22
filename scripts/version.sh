#!/usr/bin/env bash
# version.sh — Show, set, or bump the project version across all files.
#
# Usage:
#   ./scripts/version.sh              # defaults to show
#   ./scripts/version.sh show         # print current version
#   ./scripts/version.sh set <x.y.z>  # sync all files to given version
#   ./scripts/version.sh bump patch   # auto-increment patch
#   ./scripts/version.sh bump minor   # auto-increment minor (reset patch)
#   ./scripts/version.sh bump major   # auto-increment major (reset minor+patch)
#
# Files updated by set/bump:
#   1. Cargo.toml                  — version = "x.y.z"
#   2. .claude-plugin/plugin.json  — "version": "x.y.z"
#   3. README.md                   — --about block (rebuilt from binary)
#   4. CHANGES.md                  — stub entry for new version
#   5. Cargo.lock                  — regenerated via cargo check

set -euo pipefail

# Resolve project root (script lives in scripts/)
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

CARGO_TOML="$ROOT/Cargo.toml"
PLUGIN_JSON="$ROOT/.claude-plugin/plugin.json"
README="$ROOT/README.md"
CHANGES="$ROOT/CHANGES.md"

# Portable in-place sed: BSD (macOS) requires -i '', GNU (Linux) requires -i alone.
sedi() {
    if sed --version 2>/dev/null | grep -q 'GNU'; then
        sed -i "$@"
    else
        sed -i '' "$@"
    fi
}

# Read current version from Cargo.toml
current_version() {
    grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

# --- show -------------------------------------------------------------------
cmd_show() {
    current_version
}

# --- set <version> -----------------------------------------------------------
cmd_set() {
    local VERSION="$1"

    # Validate semver format (strict: major.minor.patch, no pre-release)
    if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Error: '$VERSION' is not a valid version (expected x.y.z)" >&2
        exit 1
    fi

    local CURRENT
    CURRENT="$(current_version)"

    local CHANGED=0

    # 1. Cargo.toml
    if [[ "$CURRENT" != "$VERSION" ]]; then
        sedi "s/^version = \".*\"/version = \"$VERSION\"/" "$CARGO_TOML"
        echo "Updated Cargo.toml: $CURRENT -> $VERSION"
        CHANGED=1
    else
        echo "Cargo.toml: already $VERSION"
    fi

    # 2. plugin.json
    if [[ -f "$PLUGIN_JSON" ]]; then
        local PLUGIN_CURRENT
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

    # 3. README.md — rebuild --about block from actual binary output
    if [[ -f "$README" ]]; then
        echo "Building binary for --about output..."
        touch "$ROOT/src/main.rs"
        (cd "$ROOT" && cargo build --quiet)
        local ABOUT_FILE
        ABOUT_FILE="$(mktemp)"
        "$ROOT/target/debug/aigent" --about > "$ABOUT_FILE" 2>/dev/null
        if [[ -s "$ABOUT_FILE" ]]; then
            # Verify the binary reports the expected version
            if ! grep -Fq "$VERSION" "$ABOUT_FILE"; then
                echo "Error: built binary reports wrong version (expected $VERSION)" >&2
                cat "$ABOUT_FILE" >&2
                rm -f "$ABOUT_FILE"
                exit 1
            fi
            # Replace the code block under "## About and licence":
            # print everything, but skip lines inside the ``` block
            # and insert the fresh --about output instead.
            local TMPFILE
            TMPFILE="$(mktemp)"
            awk -v aboutfile="$ABOUT_FILE" '
                /^## About and [Ll]icence/ { in_section=1; print; next }
                in_section && /^```$/ && !in_block { in_block=1; print; next }
                in_section && in_block && /^```$/ {
                    while ((getline line < aboutfile) > 0) print line
                    print; in_section=0; in_block=0; next
                }
                in_section && in_block { next }
                { print }
            ' "$README" > "$TMPFILE"
            # Verify the replacement actually happened
            if ! grep -Fq "$VERSION" "$TMPFILE"; then
                echo "Error: README --about block not updated (heading mismatch?)" >&2
                rm -f "$TMPFILE" "$ABOUT_FILE"
                exit 1
            fi
            mv "$TMPFILE" "$README"
            echo "Updated README.md --about block from binary output"
            CHANGED=1
        else
            echo "Warning: aigent --about produced no output" >&2
        fi
        rm -f "$ABOUT_FILE"
    else
        echo "Warning: $README not found" >&2
    fi

    # 4. CHANGES.md — add stub entry if version not already present
    if [[ -f "$CHANGES" ]]; then
        if ! grep -q "## \[$VERSION\]" "$CHANGES"; then
            local TODAY
            TODAY="$(date +%Y-%m-%d)"
            local STUB
            STUB="## [$VERSION] — $TODAY\n\n_No changes yet._\n"
            # Insert after the "# Changes" header line
            sedi "s/^# Changes$/# Changes\n\n$STUB/" "$CHANGES"
            echo "Added CHANGES.md stub for $VERSION"
            CHANGED=1
        else
            echo "CHANGES.md: already has entry for $VERSION"
        fi
    else
        echo "Warning: $CHANGES not found" >&2
    fi

    # 5. Cargo.lock — regenerate
    if [[ $CHANGED -eq 1 ]]; then
        echo "Regenerating Cargo.lock..."
        (cd "$ROOT" && cargo check --quiet)
        echo "Cargo.lock regenerated"
    else
        echo "Cargo.lock: no changes needed"
    fi

    if [[ $CHANGED -eq 0 ]]; then
        echo ""
        echo "All files already at version $VERSION — no changes made."
    else
        echo ""
        echo "Version set to $VERSION across all files."
    fi
}

# --- bump <patch|minor|major> ------------------------------------------------
cmd_bump() {
    local LEVEL="$1"
    local CURRENT
    CURRENT="$(current_version)"

    local MAJOR MINOR PATCH
    IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

    case "$LEVEL" in
        patch)
            PATCH=$((PATCH + 1))
            ;;
        minor)
            MINOR=$((MINOR + 1))
            PATCH=0
            ;;
        major)
            MAJOR=$((MAJOR + 1))
            MINOR=0
            PATCH=0
            ;;
        *)
            echo "Error: unknown bump level '$LEVEL' (expected patch, minor, or major)" >&2
            exit 1
            ;;
    esac

    local NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
    echo "Bumping version: $CURRENT -> $NEW_VERSION"
    cmd_set "$NEW_VERSION"
}

# --- main --------------------------------------------------------------------
SUBCOMMAND="${1:-show}"

case "$SUBCOMMAND" in
    show)
        cmd_show
        ;;
    set)
        if [[ -z "${2:-}" ]]; then
            echo "Usage: $0 set <x.y.z>" >&2
            exit 1
        fi
        cmd_set "$2"
        ;;
    bump)
        if [[ -z "${2:-}" ]]; then
            echo "Usage: $0 bump <patch|minor|major>" >&2
            exit 1
        fi
        cmd_bump "$2"
        ;;
    *)
        echo "Usage: $0 [show | set <x.y.z> | bump <patch|minor|major>]" >&2
        exit 1
        ;;
esac
