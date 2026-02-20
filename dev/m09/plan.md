# M9: Claude Code Plugin — Work Plan

## Overview

Package aigent as a distributable Claude Code plugin with two skills
(aigent-builder, aigent-validator), a plugin manifest, and an install script
for non-Rust users. Both skills operate in hybrid mode: delegating to the
`aigent` CLI when available, falling back to prompt-only generation using
Claude's built-in knowledge of the Anthropic skill spec.

Issues: #31, #32, #33, #34, #35, #36.

## Branch Strategy

- **Dev branch**: `dev/m09` (created from `main`)
- **Task branches**: `task/m09-<name>` (created from `dev/m09`)
- After each wave, task branches merge into `dev/m09`
- After all waves, PR from `dev/m09` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- `aigent validate` — from M4/M6 (validates SKILL.md files)
- `aigent build` — from M7/M6 (builds skills from descriptions)
- Release workflow — from M8 (`.github/workflows/release.yml` produces binaries)
- Plugin format — Claude Code plugin specification

## Current State

No plugin infrastructure exists. The repository has:
- A working CLI binary (`aigent`) with all subcommands
- `.github/workflows/ci.yml` (CI) and `.github/workflows/release.yml` (release)
- No `skills/` directory, no `.claude-plugin/` directory, no `install.sh`

The release workflow (M8) already builds standalone binaries for 5 targets
and attaches them to GitHub Releases. M9 builds on this by adding an install
script that downloads those binaries.

---

## Design Decisions

### Plugin Directory Structure

```
aigent/                          # Repository root (= plugin root)
├── .claude-plugin/
│   └── plugin.json              # Plugin manifest
├── skills/
│   ├── aigent-builder/
│   │   └── SKILL.md             # Builder skill
│   └── aigent-validator/
│       └── SKILL.md             # Validator skill
├── install.sh                   # Binary install script
├── src/                         # Rust source (existing)
├── Cargo.toml                   # (existing)
└── ...
```

The plugin root is the repository root. This means users who install the
plugin from the GitHub repository get both the skills and the Rust source.
The `.claude-plugin/plugin.json` manifest makes the repo a valid Claude Code
plugin.

Skills live in `skills/` at the repo root (not inside `.claude-plugin/`).
This follows the Claude Code convention where `.claude-plugin/` only contains
`plugin.json`, and all other components live at the root.

### Plugin Manifest

```json
{
  "name": "aigent",
  "description": "AI agent skill builder and validator — create and validate SKILL.md files",
  "version": "0.1.0",
  "author": {
    "name": "Wacław Kuśnierczyk",
    "url": "https://github.com/wkusnierczyk"
  },
  "homepage": "https://github.com/wkusnierczyk/aigent",
  "repository": "https://github.com/wkusnierczyk/aigent",
  "license": "MIT",
  "keywords": ["ai", "agent", "skills", "skill-builder", "validator"]
}
```

The `version` field must stay in sync with `Cargo.toml`'s `version`. This is
enforced by a test (issue #34).

Skills are auto-discovered from the `skills/` directory — no explicit paths
needed in `plugin.json`.

### Namespaced Skill Invocation

When installed as a plugin, the skills are invoked as:
- `/aigent:aigent-builder [description]`
- `/aigent:aigent-validator [path]`

The `aigent:` prefix is the plugin namespace (from `plugin.json` name).

### Hybrid Mode Design

Both skills detect whether `aigent` is on `$PATH` and adapt:

**With `aigent` available (CLI mode):**
- Builder: invokes `aigent build "<purpose>" --no-llm` (or without `--no-llm`
  if LLM env vars are configured)
- Validator: invokes `aigent validate <dir>`
- Advantages: authoritative validation, full spec compliance, deterministic
  output

**Without `aigent` (prompt-only mode):**
- Builder: Claude generates SKILL.md directly from its knowledge of the
  Anthropic skill specification
- Validator: Claude reads the SKILL.md and checks it against the spec rules
  embedded in the skill instructions
- Advantages: zero installation, works everywhere Claude Code runs

Detection mechanism: `command -v aigent` (checked at skill invocation time).

### Builder Skill: aigent-builder

**Frontmatter:**
```yaml
---
name: aigent-builder
description: >-
  Generates AI agent skill definitions (SKILL.md files) from natural language
  descriptions. Creates complete skill directories with valid frontmatter and
  markdown body following the Anthropic agent skill specification. Use when
  creating new skills, generating SKILL.md files, or scaffolding skill
  directories.
allowed-tools: Bash(aigent *), Bash(command -v *), Write, Read, Glob
argument-hint: "[skill-description]"
---
```

**Body structure:**
1. Detection: check for `aigent` on `$PATH`
2. CLI mode workflow:
   - Derive output directory (default: `.claude/skills/<name>/`)
   - Run `aigent build "<purpose>"` with `--dir` pointing to the output
   - Run `aigent validate` on the result
   - Report success or fix issues
3. Prompt-only mode workflow:
   - Extract skill purpose from the user's description
   - Derive a kebab-case gerund-form name
   - Generate frontmatter (name, description) following spec rules
   - Generate concise markdown body
   - Write to `.claude/skills/<name>/SKILL.md`
   - Self-check the output against known spec rules
4. Spec rules summary (embedded in skill body for prompt-only mode):
   - Name: ≤ 64 chars, lowercase + hyphens, no reserved words, no XML tags
   - Description: non-empty, ≤ 1024 chars, third person, no XML tags
   - Body: concise, ≤ 500 lines recommended

### Validator Skill: aigent-validator

**Frontmatter:**
```yaml
---
name: aigent-validator
description: >-
  Validates AI agent skill definitions (SKILL.md files) against the Anthropic
  agent skill specification. Checks frontmatter fields (name, description),
  format rules, and body guidelines. Use when validating skills, checking
  SKILL.md files, or reviewing skill definitions for spec compliance.
allowed-tools: Bash(aigent *), Bash(command -v *), Read, Glob
argument-hint: "[skill-directory-or-file]"
---
```

**Body structure:**
1. Detection: check for `aigent` on `$PATH`
2. Resolve target: argument, or auto-detect from context (look for nearby
   SKILL.md files)
3. CLI mode: run `aigent validate <dir>`, report results, suggest fixes
4. Prompt-only mode:
   - Read the SKILL.md file
   - Check frontmatter against embedded spec rules
   - Report errors and warnings with suggested fixes
5. Spec rules checklist (embedded for prompt-only mode):
   - Name validation (format, length, reserved words, XML tags)
   - Description validation (non-empty, length, XML tags, third person)
   - Body length warning (> 500 lines)
   - Frontmatter structure (`---` delimiters, required fields)

### Install Script Design

`install.sh` detects OS and architecture, downloads the correct binary from
GitHub Releases, and installs it to `~/.local/bin` (user-writable, commonly
on `$PATH`).

Features:
- Auto-detect OS: `uname -s` → Linux, Darwin (macOS)
- Auto-detect arch: `uname -m` → x86_64, aarch64/arm64
- Map to release asset names: `aigent-<version>-<target>.tar.gz`
- Download via `curl` (with `wget` fallback)
- Extract to `~/.local/bin/aigent`
- Verify the binary runs (`aigent --version`)
- Print success message with install path and version

The script uses the GitHub API to find the latest release tag, avoiding
hardcoded versions.

Windows is not supported by the install script — Windows users should use
`cargo install aigent` or download the binary manually from Releases.

### Version Synchronization

`plugin.json` version must match `Cargo.toml` version. Enforced by:
1. A test in `tests/plugin.rs` that parses both files and asserts equality
2. A note in CLAUDE.md or CHANGES.md documenting the convention

---

## Wave 1 — Skills (parallel)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m09-builder-skill` | #31 | Create `skills/aigent-builder/SKILL.md` |
| B | `task/m09-validator-skill` | #32 | Create `skills/aigent-validator/SKILL.md` |

**Merge**: A, B → `dev/m09`. Checkpoint with user.

### Agent A — Builder Skill (#31)

Create `skills/aigent-builder/SKILL.md`:

#### Frontmatter

```yaml
---
name: aigent-builder
description: >-
  Generates AI agent skill definitions (SKILL.md files) from natural language
  descriptions. Creates complete skill directories with valid frontmatter and
  markdown body following the Anthropic agent skill specification. Use when
  creating new skills, generating SKILL.md files, or scaffolding skill
  directories.
allowed-tools: Bash(aigent *), Bash(command -v *), Write, Read, Glob
argument-hint: "[skill-description]"
---
```

#### Body

```markdown
# Skill Builder

## Setup

Check if `aigent` is available:

\`\`\`bash
command -v aigent
\`\`\`

## With aigent CLI

If `aigent` is on `$PATH`, use the CLI for authoritative skill generation:

1. Build the skill:
   \`\`\`bash
   aigent build "<purpose>" --dir .claude/skills/<name>/
   \`\`\`

2. Validate the result:
   \`\`\`bash
   aigent validate .claude/skills/<name>/
   \`\`\`

3. If validation reports errors, fix the SKILL.md and re-validate.

Use `--no-llm` to force deterministic mode (no API keys needed).
Use `--name <name>` to override the derived skill name.

## Without aigent CLI

If `aigent` is not available, generate the SKILL.md directly.

### Name rules

- Lowercase letters, numbers, and hyphens only
- Maximum 64 characters
- Prefer gerund form: "processing-pdfs", "analyzing-data"
- No reserved words: "anthropic", "claude"
- No XML tags
- Must match directory name

### Description rules

- Non-empty, maximum 1024 characters
- Third person: "Processes PDFs..." not "I process PDFs..."
- Include what the skill does AND when to use it
- No XML tags

### Body guidelines

- Keep under 500 lines
- Be concise — only add context Claude doesn't already have
- Use ## headings for sections
- Link to additional files for large content

### Output

Write the SKILL.md to `.claude/skills/<name>/SKILL.md` with:

\`\`\`
---
name: <kebab-case-name>
description: <what-it-does-and-when-to-use-it>
---

# <Title>

## Quick start

[Concise instructions]

## Usage

[Detailed usage]
\`\`\`
```

The body is intentionally concise — it provides just enough rules for
Claude to generate a valid skill without `aigent`. Claude already knows
markdown syntax and general writing conventions.

### Agent B — Validator Skill (#32)

Create `skills/aigent-validator/SKILL.md`:

#### Frontmatter

```yaml
---
name: aigent-validator
description: >-
  Validates AI agent skill definitions (SKILL.md files) against the Anthropic
  agent skill specification. Checks frontmatter fields (name, description),
  format rules, and body guidelines. Use when validating skills, checking
  SKILL.md files, or reviewing skill definitions for spec compliance.
allowed-tools: Bash(aigent *), Bash(command -v *), Read, Glob
argument-hint: "[skill-directory-or-file]"
---
```

#### Body

```markdown
# Skill Validator

## Setup

Check if `aigent` is available:

\`\`\`bash
command -v aigent
\`\`\`

## With aigent CLI

If `aigent` is on `$PATH`, use the CLI for authoritative validation:

\`\`\`bash
aigent validate <skill-directory>
\`\`\`

Exit code 0 means valid. Non-zero means errors. Warnings are printed but
do not cause failure.

If a SKILL.md file path is provided instead of a directory, `aigent`
automatically resolves to the parent directory.

## Without aigent CLI

If `aigent` is not available, validate manually by reading the SKILL.md
and checking these rules:

### Frontmatter checks

1. File starts and ends with `---` delimiters
2. `name` field: present, string, ≤ 64 chars, lowercase + hyphens + numbers
   only, no consecutive hyphens, no leading/trailing hyphens, no reserved
   words ("anthropic", "claude"), no XML tags, matches directory name
3. `description` field: present, string, non-empty, ≤ 1024 chars, no XML tags

### Optional field checks

4. `compatibility`: if present, string, ≤ 500 chars
5. `license`: if present, string
6. `allowed-tools`: if present, string

### Body checks

7. Body ≤ 500 lines (warning, not error)

### Report format

Report each issue on its own line. Prefix warnings with "warning: ".
```

---

## Wave 2 — Plugin Packaging + Install Script (depends on Wave 1)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| C | `task/m09-plugin` | #33, #35, #36 | Create plugin manifest and install script |

**Merge**: C → `dev/m09`. Checkpoint with user.

### Agent C — Plugin Packaging (#33, #35, #36)

#### `.claude-plugin/plugin.json` (#33)

Create the plugin manifest:

```json
{
  "name": "aigent",
  "description": "AI agent skill builder and validator — create and validate SKILL.md files following the Anthropic agent skill specification",
  "version": "0.1.0",
  "author": {
    "name": "Wacław Kuśnierczyk",
    "url": "https://github.com/wkusnierczyk"
  },
  "homepage": "https://github.com/wkusnierczyk/aigent",
  "repository": "https://github.com/wkusnierczyk/aigent",
  "license": "MIT",
  "keywords": ["ai", "agent", "skills", "skill-builder", "validator"]
}
```

Skills are auto-discovered from `skills/` — no explicit paths needed.

#### `install.sh` (#35, #36)

Create a POSIX-compatible install script at the repository root:

```bash
#!/bin/sh
set -eu

# Detect OS
OS="$(uname -s)"
case "$OS" in
  Linux)  OS_TARGET="unknown-linux-gnu" ;;
  Darwin) OS_TARGET="apple-darwin" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64)         ARCH_TARGET="x86_64" ;;
  aarch64|arm64)  ARCH_TARGET="aarch64" ;;
  *)              echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"

# Get latest release tag
REPO="wkusnierczyk/aigent"
VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | sed 's/.*"tag_name": "//;s/".*//')

if [ -z "$VERSION" ]; then
  echo "Failed to determine latest version"; exit 1
fi

# Download and install
ASSET="aigent-${VERSION#v}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
INSTALL_DIR="${HOME}/.local/bin"

echo "Installing aigent ${VERSION} for ${TARGET}..."
mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" | tar xz -C "$INSTALL_DIR"
chmod +x "${INSTALL_DIR}/aigent"

# Verify
if "${INSTALL_DIR}/aigent" --version > /dev/null 2>&1; then
  echo "Installed aigent $(${INSTALL_DIR}/aigent --version) to ${INSTALL_DIR}/aigent"
else
  echo "Installation failed — binary not functional"; exit 1
fi

# PATH hint
case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "Add ${INSTALL_DIR} to your PATH if not already present." ;;
esac
```

Key decisions:
- POSIX `sh` (not bash) for maximum portability
- `~/.local/bin` is the XDG-standard user binary directory
- No `sudo` required
- Uses GitHub API to resolve latest release (avoids hardcoded version)
- Verifies the binary runs after installation
- Warns if `~/.local/bin` is not on `$PATH`

---

## Wave 3 — Tests (depends on Waves 1 + 2)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| D | `task/m09-tests` | #34 | Write plugin tests in `tests/plugin.rs` and validate skills |

**Merge**: D → `dev/m09`. Checkpoint with user.

### Agent D — Plugin Tests (#34)

Tests in `tests/plugin.rs` — integration tests using `assert_cmd`,
`serde_json`, and `std::fs`.

Test infrastructure:
- `use assert_cmd::Command;`
- `use serde_json::Value;`
- Helper `fn aigent() -> Command` — same as in `tests/cli.rs`

#### Self-validation tests

| # | Test | Assert |
|---|------|--------|
| 1 | `aigent validate skills/aigent-builder/` → exit 0 | success, no errors |
| 2 | `aigent validate skills/aigent-validator/` → exit 0 | success, no errors |

These are the most important tests: the plugin's own skills must pass the
project's own validator. This is "eating our own dogfood" — if validation
fails, either the skill or the validator has a bug.

#### Plugin manifest tests

| # | Test | Assert |
|---|------|--------|
| 3 | `plugin.json` is valid JSON | parses without error |
| 4 | `plugin.json` has `name` field equal to `"aigent"` | field value match |
| 5 | `plugin.json` has `version` field | field exists and is non-empty |
| 6 | `plugin.json` has `description` field | field exists and is non-empty |

#### Version sync test

| # | Test | Assert |
|---|------|--------|
| 7 | `plugin.json` version matches `Cargo.toml` version | string equality |

This test parses both files:
- `Cargo.toml`: extract `version = "X.Y.Z"` (use `cargo_toml` crate or
  simple regex on file content)
- `plugin.json`: parse JSON, extract `.version`
- Assert they are equal

To avoid adding `cargo_toml` as a dev-dependency, use a simple regex or
string search on the raw `Cargo.toml` content:

```rust
let cargo = std::fs::read_to_string("Cargo.toml").unwrap();
let cargo_version = cargo.lines()
    .find(|l| l.starts_with("version"))
    .and_then(|l| l.split('"').nth(1))
    .unwrap();
```

#### Skill content tests

| # | Test | Assert |
|---|------|--------|
| 8 | Builder skill has `allowed-tools` in frontmatter | read + parse frontmatter |
| 9 | Validator skill has `allowed-tools` in frontmatter | read + parse frontmatter |
| 10 | Builder skill frontmatter name matches directory name | `name` == `"aigent-builder"` |
| 11 | Validator skill frontmatter name matches directory name | `name` == `"aigent-validator"` |

These use `aigent::read_properties` directly (library API) to parse the
skill files and check specific fields.

#### Install script tests

| # | Test | Assert |
|---|------|--------|
| 12 | `install.sh` exists and is executable | file exists, has `+x` permission |
| 13 | `install.sh` starts with shebang `#!/bin/sh` | first line check |

Full install script testing (actual download + install) is impractical in
CI — it requires a published release. The tests verify the script's
structure, not its runtime behavior.

---

## Wave 4 — Verify (depends on Wave 3)

Single agent runs the full check suite on `dev/m09`.

| Agent | Branch | Task |
|-------|--------|------|
| E | `dev/m09` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Deliverables

- `skills/aigent-builder/SKILL.md` — builder skill with hybrid CLI/prompt mode
- `skills/aigent-validator/SKILL.md` — validator skill with hybrid CLI/prompt mode
- `.claude-plugin/plugin.json` — plugin manifest with version sync
- `install.sh` — POSIX install script for non-Rust users
- `tests/plugin.rs` — 13 tests (2 self-validation + 4 manifest + 1 version
  sync + 4 skill content + 2 install script)
- PR: `M9: Claude Code Plugin`
