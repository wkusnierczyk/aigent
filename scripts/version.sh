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
#   ./scripts/version.sh release <x.y.z|patch|minor|major> [--dry-run]
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
        if ! grep -Fq "## [$VERSION]" "$CHANGES"; then
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

# --- release utilities -------------------------------------------------------

# Check that working tree is clean (excluding version-managed files).
check_clean_tree() {
    local EXCLUDED=(
        Cargo.toml Cargo.lock
        .claude-plugin/plugin.json
        README.md CHANGES.md
    )
    local EXCLUDE_ARGS=()
    for f in "${EXCLUDED[@]}"; do
        EXCLUDE_ARGS+=(":!$f")
    done
    local DIRTY
    DIRTY="$(git -C "$ROOT" status --porcelain -- . "${EXCLUDE_ARGS[@]}")"
    if [[ -n "$DIRTY" ]]; then
        echo "Error: working tree has uncommitted changes:" >&2
        echo "$DIRTY" >&2
        echo "" >&2
        echo "Commit or stash them before releasing." >&2
        exit 1
    fi
}

# Generate changelog entries from merged PRs since the previous tag.
generate_changelog() {
    local PREV_TAG SINCE ENTRIES

    if ! command -v gh &>/dev/null; then
        echo "Error: 'gh' CLI is required for release (https://cli.github.com)" >&2
        exit 1
    fi

    PREV_TAG="$(git -C "$ROOT" describe --tags --abbrev=0 HEAD 2>/dev/null || true)"
    if [[ -z "$PREV_TAG" ]]; then
        echo "Error: no previous version tag found" >&2
        exit 1
    fi

    SINCE="$(git -C "$ROOT" log -1 --format=%cI "$PREV_TAG")"

    ENTRIES="$(gh pr list --repo "$(gh repo view --json nameWithOwner -q .nameWithOwner)" \
        --state merged --base main \
        --search "merged:>=$SINCE" \
        --json number,title \
        --jq '.[] | "- \(.title) (#\(.number))"')"

    if [[ -z "$ENTRIES" ]]; then
        echo "Error: no merged PRs found since $PREV_TAG" >&2
        echo "Either merge PRs first or write CHANGES.md manually." >&2
        exit 1
    fi

    echo "$ENTRIES"
}

# Write a version entry into CHANGES.md (before the existing entries).
write_changelog() {
    local VERSION="$1"
    local CHANGELOG="$2"
    local TODAY
    TODAY="$(date +%Y-%m-%d)"

    # Check if entry already exists (re-release)
    if grep -Fq "## [$VERSION]" "$CHANGES"; then
        echo "Warning: CHANGES.md already has entry for $VERSION — keeping existing content." >&2
        return
    fi

    # Insert new entry after "# Changes" header (line 1) and blank line (line 2)
    local TMPFILE
    TMPFILE="$(mktemp)"
    {
        echo "# Changes"
        echo ""
        echo "## [$VERSION] — $TODAY"
        echo ""
        echo "$CHANGELOG"
        echo ""
        # Rest of file: skip "# Changes" header + blank line
        tail -n +3 "$CHANGES"
    } > "$TMPFILE"
    mv "$TMPFILE" "$CHANGES"
    echo "Updated CHANGES.md with PR list for $VERSION"
}

# --- release <x.y.z|patch|minor|major> [--dry-run] --------------------------
cmd_release() {
    # Parse --dry-run from arguments
    local DRY_RUN=0
    local RELEASE_ARGS=()
    for arg in "$@"; do
        case "$arg" in
            --dry-run) DRY_RUN=1 ;;
            *) RELEASE_ARGS+=("$arg") ;;
        esac
    done

    local ARG="${RELEASE_ARGS[0]}"
    local VERSION

    # Determine target version
    case "$ARG" in
        patch|minor|major)
            local CURRENT MAJOR MINOR PATCH
            CURRENT="$(current_version)"
            IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"
            case "$ARG" in
                patch) PATCH=$((PATCH + 1)) ;;
                minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
                major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
            esac
            VERSION="${MAJOR}.${MINOR}.${PATCH}"
            ;;
        *)
            VERSION="$ARG"
            ;;
    esac

    # Validate semver
    if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Error: '$VERSION' is not a valid version (expected x.y.z)" >&2
        exit 1
    fi

    echo "Releasing version $VERSION..."
    echo ""

    # 1. Preflight
    local BRANCH
    BRANCH="$(git -C "$ROOT" symbolic-ref --short HEAD 2>/dev/null || true)"
    if [[ "$BRANCH" != "main" ]]; then
        echo "Error: releases must be made from 'main' (currently on '$BRANCH')" >&2
        exit 1
    fi

    git -C "$ROOT" fetch origin main --quiet
    local LOCAL REMOTE
    LOCAL="$(git -C "$ROOT" rev-parse HEAD)"
    REMOTE="$(git -C "$ROOT" rev-parse origin/main)"
    if [[ "$LOCAL" != "$REMOTE" ]]; then
        echo "Error: local main is not up-to-date with origin/main" >&2
        echo "Run 'git pull' first." >&2
        exit 1
    fi

    check_clean_tree

    if git -C "$ROOT" rev-parse --verify --quiet "refs/tags/v$VERSION" &>/dev/null; then
        echo "Error: tag v$VERSION already exists" >&2
        exit 1
    fi

    # 2. Generate changelog
    echo "Generating changelog from merged PRs..."
    local CHANGELOG
    CHANGELOG="$(generate_changelog)"
    echo "$CHANGELOG"
    echo ""

    if [[ $DRY_RUN -eq 1 ]]; then
        echo "[dry-run] Would update CHANGES.md, version files, commit, tag, and push."
        echo "[dry-run] git add Cargo.toml Cargo.lock .claude-plugin/plugin.json README.md CHANGES.md"
        echo "[dry-run] git commit -m \"Bump version to $VERSION\""
        echo "[dry-run] git tag v$VERSION"
        echo "[dry-run] git push origin HEAD v$VERSION"
        return
    fi

    # 3. Write CHANGES.md (before cmd_set, so cmd_set skips stub insertion)
    write_changelog "$VERSION" "$CHANGELOG"
    echo ""

    # 4. Update version files (Cargo.toml, plugin.json, README.md, Cargo.lock)
    cmd_set "$VERSION"
    echo ""

    # 5. Stage
    git -C "$ROOT" add \
        Cargo.toml Cargo.lock .claude-plugin/plugin.json README.md CHANGES.md

    # 6. Commit
    git -C "$ROOT" commit -m "Bump version to $VERSION"

    # 7. Tag
    git -C "$ROOT" tag "v$VERSION"

    # 8. Push commit and tag together
    git -C "$ROOT" push origin HEAD "v$VERSION"

    echo ""
    echo "Released v$VERSION — CI workflow triggered."
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
    release)
        if [[ -z "${2:-}" ]]; then
            echo "Usage: $0 release <x.y.z|patch|minor|major> [--dry-run]" >&2
            exit 1
        fi
        shift
        cmd_release "$@"
        ;;
    *)
        echo "Usage: $0 [show | set <x.y.z> | bump <patch|minor|major> | release <x.y.z|patch|minor|major>]" >&2
        exit 1
        ;;
esac
