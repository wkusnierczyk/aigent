# M3: Parser — Work Plan

## Overview

Implement the SKILL.md parser: extract YAML frontmatter, validate required
fields, and construct `SkillProperties`. Then write tests covering valid and
invalid inputs.
Issues: #12, #13.

## Branch Strategy

- **Dev branch**: `dev/m03` (created from `main`)
- **Task branches**: `task/m03-<name>` (created from `dev/m03`)
- After each wave, task branches merge into `dev/m03`
- After all waves, PR from `dev/m03` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- `AigentError::Parse` — for structural parser errors (from M2)
- `AigentError::Yaml` — for YAML syntax errors (from M2, via `#[from]`)
- `AigentError::Validation` — for missing required fields (from M2)
- `SkillProperties` — construction target (from M2)
- `serde_yaml_ng` — YAML parsing

## Current State

`find_skill_md` is already implemented (M1). `parse_frontmatter` and
`read_properties` are stubs (`todo!()`).

---

## Wave 1 — Implementation

Since `find_skill_md` is already implemented and tests need a working parser
to pass, implementation comes first. Tests follow in Wave 2.

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m03-parser` | #12 | Implement `parse_frontmatter` and `read_properties` in `src/parser.rs` |

**Merge**: A → `dev/m03`. Checkpoint with user.

### Agent A — Parser (#12)

#### `parse_frontmatter(content: &str) -> Result<(HashMap<String, Value>, String)>`

Extract YAML frontmatter and markdown body from raw file content.

1. Verify content starts with a line that is exactly `---` → `AigentError::Parse`
2. Find the closing delimiter: next line that is exactly `---` (exact match,
   not substring — a `---` indented or within a YAML multiline value won't match)
   → `AigentError::Parse` if not found
3. Extract the YAML between delimiters, parse with `serde_yaml_ng::from_str`
   → let `?` propagate as `AigentError::Yaml` (preserves structured error with
   line/column info)
4. Verify the parsed value is a mapping (not a list or scalar)
   → `AigentError::Parse` if not a mapping
5. Convert to `HashMap<String, Value>`
6. Extract the body (everything after closing `---`, strip leading newline)
7. Return `(metadata, body)`

Error variant strategy (hybrid):
- `AigentError::Parse` for structural issues detected by parser logic
  (missing delimiters, non-mapping YAML)
- `AigentError::Yaml` for YAML syntax errors (automatic via `?` and `#[from]`)

#### `read_properties(dir: &Path) -> Result<SkillProperties>`

Full pipeline: find → read → parse → validate required fields → construct.

1. Call `find_skill_md(dir)` → `AigentError::Parse` if `None`
2. Read the file with `std::fs::read_to_string`
   (IO errors propagate via `AigentError::Io` automatically from `#[from]`)
3. Call `parse_frontmatter(&content)` to get `(metadata, _body)`
4. Extract and validate required fields:
   - `name` must exist and be a `Value::String` → `AigentError::Validation` if
     missing or wrong type
   - `description` must exist and be a `Value::String` →
     `AigentError::Validation` if missing or wrong type
   - Empty string check deferred to M4 validator (M3 only checks presence + type)
5. Extract optional string fields: `license`, `compatibility`, `allowed-tools`
   (kebab-case key in the HashMap)
6. Remove known keys from the HashMap; remaining entries become `metadata`
   (known keys: `name`, `description`, `license`, `compatibility`, `allowed-tools`)
7. If `metadata` is empty, set it to `None`; otherwise `Some(remaining)`
8. Construct and return `SkillProperties`

Key decisions:
- This is a *parser*, not a *validator* — it checks minimum requirements for
  `SkillProperties` construction (name/description exist and are strings). Full
  validation (name format, length limits, non-empty, etc.) is M4's job.
- Known keys are extracted by their YAML names (kebab-case: `allowed-tools`,
  not snake_case). After extraction, they do NOT appear in `metadata`.
- Values in `metadata` may be nested (hashes, arrays) — stored as
  `serde_yaml_ng::Value` without flattening.
- `allowed-tools` is stored as a plain string; parsing tool specs is the
  caller's responsibility.
- Empty body after closing `---` is valid.
- Wrong type for a known key (e.g., `name: [a, b]`) → `AigentError::Validation`.

---

## Wave 2 — Tests (depends on Wave 1)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| B | `task/m03-tests` | #13 | Write parser tests in `src/parser.rs` `#[cfg(test)]` |

**Merge**: B → `dev/m03`. Checkpoint with user.

### Agent B — Tests (#13)

`src/parser.rs` — `#[cfg(test)] mod tests`

Test infrastructure:
- Use `tempfile::tempdir()` for temporary skill directories
- Helper function to write a SKILL.md with given content into a temp dir

#### `find_skill_md` tests

| # | Test |
|---|------|
| 1 | Returns `SKILL.md` path when uppercase file exists |
| 2 | Returns `skill.md` path when only lowercase exists |
| 3 | Prefers uppercase when both exist |
| 4 | Returns `None` when neither exists |

#### `parse_frontmatter` tests

| # | Test |
|---|------|
| 5 | Valid frontmatter with body → `(metadata, body)` |
| 6 | Valid frontmatter with empty body → `(metadata, "")` |
| 7 | Content not starting with `---` → `AigentError::Parse` |
| 8 | Missing closing `---` → `AigentError::Parse` |
| 9 | Invalid YAML syntax → `AigentError::Yaml` |
| 10 | Non-mapping YAML (e.g., list) → `AigentError::Parse` |
| 11 | Metadata with nested values (hash inside metadata) |
| 12 | `allowed-tools` kebab-case field preserved in metadata |
| 13 | `---` inside YAML multiline value not treated as closing delimiter |

#### `read_properties` tests

| # | Test |
|---|------|
| 14 | Valid skill directory → returns `SkillProperties` with all fields |
| 15 | Correctly parses `allowed-tools` into `allowed_tools` field |
| 16 | Correctly preserves nested metadata; known keys absent from metadata |
| 17 | Missing SKILL.md → `AigentError::Parse` |
| 18 | Missing `name` in frontmatter → `AigentError::Validation` |
| 19 | Missing `description` in frontmatter → `AigentError::Validation` |
| 20 | Valid directory with only required fields → optionals are `None` |
| 21 | Empty metadata map → `metadata` field is `None` (not `Some({})`) |

---

## Wave 3 — Verify (depends on Wave 2)

Single agent runs the full check suite on `dev/m03`.

| Agent | Branch | Task |
|-------|--------|------|
| C | `dev/m03` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Deliverables

- `src/parser.rs` — `parse_frontmatter` and `read_properties` implemented
- 21 tests inline in `#[cfg(test)] mod tests`
- PR: `M3: Parser`
