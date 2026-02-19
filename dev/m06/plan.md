# M6: CLI — Work Plan

## Overview

Polish the CLI (`src/main.rs`) and add comprehensive integration tests using
`assert_cmd` and `predicates`. The CLI subcommand handlers are already
implemented (M1 scaffolding, M4 warning fix); this milestone focuses on
edge-case hardening, error message consistency, and test coverage.

Issues: #18, #19, #20.

## Branch Strategy

- **Dev branch**: `dev/m06` (created from `main`)
- **Task branches**: `task/m06-<name>` (created from `dev/m06`)
- After each wave, task branches merge into `dev/m06`
- After all waves, PR from `dev/m06` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- `aigent::validate` — from M4 (`src/validator.rs`)
- `aigent::read_properties` — from M3 (`src/parser.rs`)
- `aigent::to_prompt` — from M5 (`src/prompt.rs`)
- `clap` — already in `Cargo.toml`
- `assert_cmd`, `predicates` — already in `[dev-dependencies]`

## Current State

`src/main.rs` is fully wired up from M1 scaffolding with M4 fixes:

- `Validate` handler calls `aigent::validate`, filters warnings vs errors,
  prints to stderr, exits 1 on errors only.
- `ReadProperties` handler calls `aigent::read_properties`, prints JSON to
  stdout, exits 1 on error with message to stderr.
- `ToPrompt` handler calls `aigent::to_prompt`, prints XML to stdout.
- `Build` and `Init` handlers print "not yet implemented" and exit 1 (M7 scope).
- `--about` prints project info from compile-time `env!()` macros.
- `resolve_skill_dir` resolves SKILL.md file paths to parent directories.
- No subcommand → usage message + exit 1.

There are currently **0 integration tests** in `tests/`. The `assert_cmd` and
`predicates` dev-dependencies are declared but unused.

---

## Review Finding Resolutions

### Finding 1 (Medium): `--about` format diverges from issue #20

**Resolution**: Fix `print_about` to match the issue #20 specification exactly.
Two changes:
- `authors:` → `author:` (singular, matching the issue spec)
- Remove the license URL suffix (`https://opensource.org/licenses/MIT`); the
  issue specifies only the license identifier.

The integration test (test #3) will assert against the corrected format.

### Finding 2 (Medium): Test #5 fixture must produce zero warnings

**Resolution**: Use a minimal fixture for test #5 with only `name` and
`description` fields and a short body (< 500 lines). This guarantees no
unexpected-metadata-key warnings and no body-length warnings, making the
"no stderr" assertion reliable.

```yaml
---
name: my-skill
description: A test skill
---
Short body.
```

### Finding 3 (Low): `resolve_skill_dir` not unit-tested

**Resolution**: No change needed. Integration tests #9 and #12 sufficiently
cover the realistic code paths. The function is private to `main.rs`.

### Finding 4 (Low): `to-prompt` empty test trailing newline from `println!`

**Resolution**: Use `predicate::str::trim()` combined with
`predicate::eq()` for exact-match stdout comparisons. This normalizes the
trailing newline added by `println!` without relying on fragile raw string
matching. For non-exact tests, use `predicate::str::contains()`.

---

## Design Decisions

### Test Placement

Integration tests go in `tests/cli.rs` (not inline `#[cfg(test)]` in main.rs).
This is the idiomatic Rust location for binary integration tests — `assert_cmd`
invokes the compiled binary as a subprocess, which requires an external test
file.

### Test Fixture Strategy

Tests create temporary skill directories using `tempfile::tempdir()` with
inline SKILL.md content. No static fixture directories — this keeps tests
self-contained, avoids brittle path dependencies, and matches the pattern
established in M3–M5.

Helper function `make_skill_dir(name, content)` creates a parent temp dir
with a named subdirectory containing SKILL.md, returning `(TempDir, PathBuf)`.
Same pattern as M4/M5 tests but in the integration test file.

### Error Message Prefix Convention

All CLI error messages to stderr are prefixed with `aigent <subcommand>:` for
consistency. This is already the pattern in `ReadProperties` (`"aigent
read-properties: {e}"`) but not in `Validate` (which prints raw validator
messages). The plan keeps `Validate` messages unprefixed because validator
output is meant to be machine-parseable (line-oriented error/warning messages).

### Unimplemented Subcommands

`Build` and `Init` currently print "not yet implemented" and exit 1. These are
M7 scope. Integration tests verify this behavior (exit 1, message on stderr)
so that M7 can later replace the stubs and update the tests.

### `resolve_skill_dir` Behavior

The existing function resolves a SKILL.md file path to its parent directory
using `path.is_file()`. This means:
- `aigent validate ./my-skill/SKILL.md` → validates `./my-skill/`
- `aigent validate ./my-skill/` → validates `./my-skill/`
- `aigent validate ./nonexistent` → passes `./nonexistent` (is_file=false for
  nonexistent paths), fails at `find_skill_md`

This behavior is correct and tested.

### `--version` Flag

`clap` auto-generates `--version` from `#[command(version)]` + `Cargo.toml`'s
`version` field. This is already present and tested via `--version` flag.
No implementation work needed.

---

## Wave 1 — CLI Polish

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m06-cli` | #18, #20 | Polish CLI error handling and `--about` output |

**Merge**: A → `dev/m06`. Checkpoint with user.

### Agent A — CLI Polish (#18, #20)

Most CLI behavior is already implemented. The remaining work:

#### 1. Fix `--about` output format

Fix `print_about` to match issue #20 exactly:
- Change `authors:` → `author:` (singular)
- Remove the license URL suffix — only print the license identifier
- The corrected output:
  ```
  aigent: Rust AI Agent Skills Tool
  ├─ version:    <CARGO_PKG_VERSION>
  ├─ author:     <CARGO_PKG_AUTHORS>
  ├─ source:     <CARGO_PKG_REPOSITORY>
  └─ license:    <CARGO_PKG_LICENSE>
  ```

#### 2. Consistent error prefix for `Validate`

Currently the `Validate` handler prints raw validator messages without an
`aigent validate:` prefix. This is intentional — validator messages are
self-descriptive (e.g., `"name exceeds 64 characters"`, `"warning: body
exceeds 500 lines"`). Keep as-is per the Design Decisions section above.

#### 3. Verify `Build` and `Init` stubs

Confirm both stubs print a descriptive message to stderr and exit 1. Current
behavior matches. No changes needed.

#### 4. Verify `resolve_skill_dir` edge cases

The function handles:
- File path → parent directory
- Directory path → pass through
- Nonexistent path → pass through (downstream handles error)

No changes needed if current behavior is correct.

**Expected outcome**: Minimal or no code changes — primarily a verification
pass confirming the existing implementation matches issue specifications. If
discrepancies are found, fix them.

---

## Wave 2 — Integration Tests (depends on Wave 1)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| B | `task/m06-tests` | #19 | Write CLI integration tests in `tests/cli.rs` |

**Merge**: B → `dev/m06`. Checkpoint with user.

### Agent B — Integration Tests (#19)

`tests/cli.rs` — integration tests using `assert_cmd` and `predicates`.

Test infrastructure:
- `use assert_cmd::Command;`
- `use predicates::prelude::*;`
- `use tempfile::tempdir;`
- Helper `fn aigent() -> Command` — returns `Command::cargo_bin("aigent").unwrap()`
- Helper `fn make_skill_dir(name, content) -> (TempDir, PathBuf)` — creates
  temp dir with named subdirectory containing SKILL.md

#### `--help` / `--version` / `--about` / no-args tests

| # | Test | Assert |
|---|------|--------|
| 1 | `aigent --help` | exit 0, stdout contains "AI agent skill builder" |
| 2 | `aigent --version` | exit 0, stdout contains version string |
| 3 | `aigent --about` | exit 0, stdout contains "aigent:", version, license |
| 4 | `aigent` (no args) | exit 1, stderr contains "Usage" |

#### `validate` tests

| # | Test | Assert |
|---|------|--------|
| 5 | Valid skill directory → exit 0 (minimal fixture: name+description only) | success, no stderr |
| 6 | Invalid skill (missing name) → exit 1, errors on stderr | failure, stderr contains "name" |
| 7 | Missing SKILL.md → exit 1, error on stderr | failure, stderr contains "SKILL.md" |
| 8 | Warnings only (e.g., body > 500 lines) → exit 0 | success, stderr contains "warning:" |
| 9 | SKILL.md file path resolves to parent dir | success (same as dir path) |

#### `read-properties` tests

| # | Test | Assert |
|---|------|--------|
| 10 | Valid skill → exit 0, valid JSON on stdout | success, stdout is valid JSON with "name" key |
| 11 | Invalid skill → exit 1, error on stderr | failure, stderr contains "aigent read-properties:" |
| 12 | SKILL.md file path resolves to parent dir | success, valid JSON output |

#### `to-prompt` tests

| # | Test | Assert |
|---|------|--------|
| 13 | Single skill directory → XML on stdout | success, stdout contains `<available_skills>` |
| 14 | Multiple skill directories → aggregated XML | success, stdout contains multiple `<skill>` |
| 15 | No directories → empty wrapper | success, trimmed stdout eq `<available_skills>\n</available_skills>` |
| 16 | Invalid directory mixed with valid → only valid in output | success, one `<skill>` block |

#### Unimplemented subcommand tests

| # | Test | Assert |
|---|------|--------|
| 17 | `aigent build "purpose"` → exit 1, "not yet implemented" | failure, stderr contains "not yet implemented" |
| 18 | `aigent init` → exit 1, "not yet implemented" | failure, stderr contains "not yet implemented" |

---

## Wave 3 — Verify (depends on Wave 2)

Single agent runs the full check suite on `dev/m06`.

| Agent | Branch | Task |
|-------|--------|------|
| C | `dev/m06` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Deliverables

- `src/main.rs` — `print_about` fixed to match issue #20 spec
- `tests/cli.rs` — 18 integration tests using `assert_cmd` and `predicates`
- PR: `M6: CLI`
