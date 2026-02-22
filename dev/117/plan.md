# Plan: Add `release` command to `version.sh` (#117)

---

## v1 (obsolete)

<details>
<summary>Original plan — superseded by v2 below</summary>

### Context

The release workflow currently requires four manual steps after running
`version.sh set`/`bump`:

```bash
./scripts/version.sh set 0.5.0
git add Cargo.toml Cargo.lock .claude-plugin/plugin.json README.md CHANGES.md
git commit -m "Bump version to 0.5.0"
git tag v0.5.0
git push origin v0.5.0
```

Pushing the `v*` tag triggers the CI release workflow (`.github/workflows/release.yml`),
which runs tests, cross-compiles five targets, creates a GitHub Release with
changelog and binaries, and publishes to crates.io.

The goal is to consolidate the full sequence into a single `release` subcommand,
including automatic generation of a meaningful CHANGES.md entry from merged PRs.

### Design

#### Interface

```bash
./scripts/version.sh release <x.y.z>              # explicit version
./scripts/version.sh release <patch|minor|major>   # bump level
./scripts/version.sh release <x.y.z> --dry-run     # preview only
```

The command detects whether the argument is a semver string or a bump level,
generates the changelog from merged PRs, updates all version files, and
performs the git commit/tag/push sequence.

#### Changelog generation

The `release` command replaces the `_No changes yet._` stub in CHANGES.md
with a list of PRs merged since the previous version tag. It uses `gh` to
query merged PRs:

```bash
PREV_TAG=$(git describe --tags --abbrev=0 HEAD)
SINCE=$(git log -1 --format=%aI "$PREV_TAG")

gh pr list --state merged --base main \
  --search "merged:>=$SINCE" \
  --json number,title \
  --jq '.[] | "- \(.title) (#\(.number))"'
```

This produces clean, PR-level entries:

```
- Fix version.sh: case-sensitive heading match (#123)
- Add implementation plan for format --check diff output (#115)
```

**Guard**: If `gh` returns zero PRs, the release aborts with a message
asking the user to either write the changelog manually or verify that
PRs were merged. This prevents releasing with an empty changelog.

**Prerequisite**: `gh` must be installed and authenticated. The script
checks for `gh` availability early and aborts with a helpful message
if missing.

#### Dry-run mode

A `--dry-run` flag previews the full release without executing. It shows
the generated changelog and the git commands that would run. Parsed by
scanning arguments before dispatch.

#### Dirty-tree guard

Before making any changes, the command checks for uncommitted modifications
**outside** the version-managed files. The five managed files
(`Cargo.toml`, `Cargo.lock`, `plugin.json`, `README.md`, `CHANGES.md`) are
excluded from the dirty check — they will be modified and staged by the
script itself.

#### Step sequence

1. **Preflight**: Check `gh` is available; abort if working tree is dirty
2. **Resolve version**: Parse argument as semver or bump level
3. **Generate changelog**: Query merged PRs via `gh`, abort if none found
4. **Update version files**: Call `cmd_set` (updates Cargo.toml, plugin.json, README.md, Cargo.lock)
5. **Write changelog**: Replace CHANGES.md stub with generated PR list
6. **Stage**: `git add` all managed files
7. **Commit**: `git commit -m "Bump version to <x.y.z>"`
8. **Tag**: `git tag v<x.y.z>`
9. **Push tag**: `git push origin v<x.y.z>`

#### Interaction with `cmd_set` and CHANGES.md

`cmd_set` currently inserts a stub entry (`_No changes yet._`) into
CHANGES.md when the version is new. The `release` command relies on this:
it calls `cmd_set` first to create the stub, then replaces the stub body
with the generated PR list. This avoids modifying `cmd_set` and keeps the
two code paths (manual `set` vs automated `release`) independent.

#### Error handling

Each step is guarded by `set -e`. If any step fails the script aborts
immediately with a non-zero exit code. The user can inspect state and
recover manually. This is intentionally simple — no rollback logic — since
the steps are individually reversible (`git reset`, `git tag -d`, etc.).

### Changes

#### 1. Add `--dry-run` flag parsing (`scripts/version.sh`)

Add a top-level argument scan loop before the `case` dispatch. If `--dry-run`
is found, set `DRY_RUN=1` and remove it from the argument list. Introduce
a helper function:

```bash
DRY_RUN=0

run() {
    if [[ $DRY_RUN -eq 1 ]]; then
        echo "[dry-run] $*"
    else
        "$@"
    fi
}
```

#### 2. Add utility functions (`scripts/version.sh`)

**Dirty-tree check:**

```bash
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
```

**Changelog generation:**

```bash
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

    SINCE="$(git -C "$ROOT" log -1 --format=%aI "$PREV_TAG")"

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
```

#### 3. Add `cmd_release` function (`scripts/version.sh`)

```bash
cmd_release() {
    local ARG="$1"
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
    check_clean_tree

    # 2. Generate changelog
    echo "Generating changelog from merged PRs..."
    local CHANGELOG
    CHANGELOG="$(generate_changelog)"
    echo "$CHANGELOG"
    echo ""

    if [[ $DRY_RUN -eq 1 ]]; then
        echo "[dry-run] Would update version files, write changelog, commit, tag, and push."
        return
    fi

    # 3. Update version files (creates CHANGES.md stub)
    cmd_set "$VERSION"
    echo ""

    # 4. Replace stub in CHANGES.md with generated changelog
    local VERSION_ESCAPED
    VERSION_ESCAPED="$(echo "$VERSION" | sed 's/\./\\./g')"
    local TMPFILE
    TMPFILE="$(mktemp)"
    awk -v ver="$VERSION_ESCAPED" -v changelog="$CHANGELOG" '
        $0 ~ "^## \\[" ver "\\]" { print; getline; sub(/_No changes yet\._/, changelog); print; next }
        { print }
    ' "$CHANGES" > "$TMPFILE"
    mv "$TMPFILE" "$CHANGES"
    echo "Updated CHANGES.md with PR list"
    echo ""

    # 5. Stage
    git -C "$ROOT" add \
        Cargo.toml Cargo.lock .claude-plugin/plugin.json README.md CHANGES.md

    # 6. Commit
    git -C "$ROOT" commit -m "Bump version to $VERSION"

    # 7. Tag
    git -C "$ROOT" tag "v$VERSION"

    # 8. Push tag
    git -C "$ROOT" push origin "v$VERSION"

    echo ""
    echo "Released v$VERSION — CI workflow triggered."
}
```

#### 4. Update `case` dispatch and usage (`scripts/version.sh`)

Add `release` to the main dispatch and update the usage header comment:

```
# Usage:
#   ./scripts/version.sh release <x.y.z|patch|minor|major> [--dry-run]
```

```bash
    release)
        if [[ -z "${2:-}" ]]; then
            echo "Usage: $0 release <x.y.z|patch|minor|major> [--dry-run]" >&2
            exit 1
        fi
        cmd_release "$2"
        ;;
```

#### 5. Update README release instructions (`README.md`)

Replace the manual multi-step release instructions under
"### Release workflow" with the single-command form:

```bash
./scripts/version.sh release 0.5.0
# or
./scripts/version.sh release patch
```

Document that:
- `--dry-run` previews the release without executing
- The changelog is auto-generated from merged PRs via `gh`
- `gh` must be installed and authenticated

### Files to modify

| File | Change |
|------|--------|
| `scripts/version.sh` | Add `--dry-run` parsing, `check_clean_tree`, `generate_changelog`, `cmd_release`, update dispatch and usage |
| `README.md` | Update "Release workflow" section |

### Verification

```bash
# Dry-run — shows generated changelog and planned steps
./scripts/version.sh release patch --dry-run

# Confirm existing commands still work
./scripts/version.sh show
./scripts/version.sh set "$(./scripts/version.sh show)"  # idempotent

# Dirty-tree guard
echo test > /tmp/dirty && cp /tmp/dirty .
./scripts/version.sh release patch --dry-run  # should abort
rm dirty

# Full release (on a real release)
./scripts/version.sh release 0.5.0
```

</details>

---

## v2

Incorporates findings from `dev/117/review.md`. Changes from v1:

- **§3.1 fix**: Eliminated fragile awk stub replacement entirely — write
  CHANGES.md entry *before* `cmd_set` so `cmd_set` skips its stub insertion
- **§3.2 fix**: Push commit (`git push origin HEAD`) before pushing the tag
- **§3.3 fix**: No longer depends on `cmd_set`'s `\n`-in-sed stub insertion
  (the portability bug is sidestepped, not inherited)
- **§4.3 fix**: Preflight checks for existing tag
- **§4.1 fix**: Warn on re-release (existing CHANGES.md entry preserved)

## Context

The release workflow currently requires manual steps after running
`version.sh set`/`bump`:

```bash
./scripts/version.sh set 0.5.0
git add Cargo.toml Cargo.lock .claude-plugin/plugin.json README.md CHANGES.md
git commit -m "Bump version to 0.5.0"
git tag v0.5.0
git push origin v0.5.0
```

Pushing the `v*` tag triggers the CI release workflow
(`.github/workflows/release.yml`), which runs tests, cross-compiles
five targets, creates a GitHub Release with changelog and binaries,
and publishes to crates.io.

The goal is to consolidate the full sequence into a single `release`
subcommand, including automatic generation of a meaningful CHANGES.md
entry from merged PRs.

## Design

### Interface

```bash
./scripts/version.sh release <x.y.z>              # explicit version
./scripts/version.sh release <patch|minor|major>   # bump level
./scripts/version.sh release <x.y.z> --dry-run     # preview only
```

### Changelog generation

The `release` command generates the CHANGES.md entry from merged PRs
via `gh`:

```bash
PREV_TAG=$(git describe --tags --abbrev=0 HEAD)
SINCE=$(git log -1 --format=%aI "$PREV_TAG")

gh pr list --state merged --base main \
  --search "merged:>=$SINCE" \
  --json number,title \
  --jq '.[] | "- \(.title) (#\(.number))"'
```

This produces clean, PR-level entries:

```
- Fix version.sh: case-sensitive heading match (#123)
- Add implementation plan for format --check diff output (#115)
```

**Guards:**
- Abort if `gh` is not installed
- Abort if no previous version tag exists
- Abort if no merged PRs found since the previous tag

### Changelog-first strategy (v1 §3.1 + §3.3 fix)

**v1 approach (flawed):** Call `cmd_set` first (creates `_No changes yet._`
stub via sed with `\n` — has BSD portability bug), then replace stub with
awk (fragile blank-line handling).

**v2 approach:** Write the CHANGES.md entry *before* calling `cmd_set`.
When `cmd_set` runs, it sees the `## [x.y.z]` entry already exists
(line 137: `grep -q "## \[$VERSION\]"`) and prints
`"CHANGES.md: already has entry for x.y.z"` — no stub insertion, no sed,
no awk replacement.

The write itself uses `head`/`tail` to splice the entry after the
`# Changes` header. This is portable, doesn't depend on sed `\n`
behavior, and doesn't require awk multi-line variable handling.

### Dry-run mode

`--dry-run` previews the generated changelog and lists the git commands
that would run, without executing anything destructive. Parsed by
scanning arguments before dispatch.

### Dirty-tree guard

Checks for uncommitted changes **excluding** the five managed files
(`Cargo.toml`, `Cargo.lock`, `plugin.json`, `README.md`, `CHANGES.md`)
since those will be modified by the script.

### Step sequence

1. **Preflight**: Check `gh` available, tree clean, tag doesn't exist
2. **Resolve version**: Parse argument as semver or bump level
3. **Generate changelog**: Query merged PRs via `gh`, abort if none
4. **Write CHANGES.md**: Insert version heading + PR list
5. **Update version files**: Call `cmd_set` (skips CHANGES.md — entry exists)
6. **Stage**: `git add` all managed files
7. **Commit**: `git commit -m "Bump version to <x.y.z>"`
8. **Tag**: `git tag v<x.y.z>`
9. **Push**: `git push origin HEAD v<x.y.z>` (commit + tag)

### Error handling

Each step is guarded by `set -e`. If any step fails the script aborts
with a non-zero exit code. No rollback logic — each step is individually
reversible (`git reset`, `git tag -d`, etc.).

## Changes

### 1. Add `--dry-run` flag parsing (`scripts/version.sh`)

Add a top-level argument scan loop before the `case` dispatch. If
`--dry-run` is found, set `DRY_RUN=1` and remove it from the argument
list.

```bash
DRY_RUN=0
ARGS=()
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
        *) ARGS+=("$arg") ;;
    esac
done
set -- "${ARGS[@]}"
```

### 2. Add utility functions (`scripts/version.sh`)

**`check_clean_tree`** — dirty-tree guard:

```bash
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
```

**`generate_changelog`** — PR list via `gh`:

```bash
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

    SINCE="$(git -C "$ROOT" log -1 --format=%aI "$PREV_TAG")"

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
```

**`write_changelog`** — splice entry into CHANGES.md:

```bash
write_changelog() {
    local VERSION="$1"
    local CHANGELOG="$2"
    local TODAY
    TODAY="$(date +%Y-%m-%d)"

    # Check if entry already exists (re-release)
    if grep -q "## \[$VERSION\]" "$CHANGES"; then
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
```

### 3. Add `cmd_release` function (`scripts/version.sh`)

```bash
cmd_release() {
    local ARG="$1"
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
    check_clean_tree

    if git -C "$ROOT" rev-parse "v$VERSION" &>/dev/null; then
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
```

### 4. Update `case` dispatch and usage (`scripts/version.sh`)

Add `release` to the main dispatch and update the usage header:

```bash
#   ./scripts/version.sh release <x.y.z|patch|minor|major> [--dry-run]
```

```bash
    release)
        if [[ -z "${2:-}" ]]; then
            echo "Usage: $0 release <x.y.z|patch|minor|major> [--dry-run]" >&2
            exit 1
        fi
        cmd_release "$2"
        ;;
```

### 5. Update README release instructions (`README.md`)

Replace the manual multi-step release instructions under
"### Release workflow" with the single-command form:

```bash
./scripts/version.sh release 0.5.0
# or
./scripts/version.sh release patch
```

Document that:
- `--dry-run` previews the release without executing
- The changelog is auto-generated from merged PRs via `gh`
- `gh` must be installed and authenticated

## Files to modify

| File | Change |
|------|--------|
| `scripts/version.sh` | Add `--dry-run` parsing, `check_clean_tree`, `generate_changelog`, `write_changelog`, `cmd_release`, update dispatch and usage |
| `README.md` | Update "Release workflow" section |

## Review findings addressed

| Review item | Resolution |
|-------------|------------|
| §3.1 Awk stub replacement bug | Eliminated — write CHANGES.md before `cmd_set` |
| §3.2 Missing commit push | Fixed — `git push origin HEAD v$VERSION` |
| §3.3 Inherited sed `\n` bug | Sidestepped — `cmd_set` never inserts stub |
| §4.3 Tag already exists | Added preflight check |
| §4.1 Re-release warning | `write_changelog` warns and preserves existing entry |

## Verification

```bash
# Dry-run — shows generated changelog and planned steps
./scripts/version.sh release patch --dry-run

# Confirm existing commands still work
./scripts/version.sh show
./scripts/version.sh set "$(./scripts/version.sh show)"  # idempotent

# Dirty-tree guard
touch dirty-test-file
./scripts/version.sh release patch --dry-run  # should abort
rm dirty-test-file

# Tag guard
git tag v99.99.99
./scripts/version.sh release 99.99.99 --dry-run  # should abort
git tag -d v99.99.99

# Full release (on a real release)
./scripts/version.sh release 0.5.0
```
