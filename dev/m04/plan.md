# M4: Validator — Work Plan

## Overview

Implement the skill directory and metadata validator: check name format,
description/compatibility constraints, reserved words, XML injection,
i18n support with NFKC normalization, and body-length warnings. Returns
all validation errors/warnings in a single pass rather than failing on
the first.

Issues: #14, #15.

## Branch Strategy

- **Dev branch**: `dev/m04` (created from `main`)
- **Task branches**: `task/m04-<name>` (created from `dev/m04`)
- After each wave, task branches merge into `dev/m04`
- After all waves, PR from `dev/m04` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- `SkillProperties` — from M2 (`src/models.rs`)
- `parse_frontmatter`, `find_skill_md`, `KNOWN_KEYS` — from M3 (`src/parser.rs`)
- `AigentError::Validation` — from M2 (`src/errors.rs`)
- `unicode-normalization` — already in `Cargo.toml`
- `regex` — already in `Cargo.toml`

## Current State

`validate_metadata` and `validate` are stubs (`todo!()`) in `src/validator.rs`.
Both return `Vec<String>` (empty = valid). This signature collects all
errors/warnings in a single pass.

## Review Finding Resolutions

### Finding 1 (Medium): `warning:` prefix + `main.rs` exit code

**Resolution**: Fix `main.rs` in M4 to filter warnings before deciding the
exit code. Warnings are printed to stderr but do not cause exit code 1.
Only non-warning messages cause failure. This is a small scope expansion
into M6 territory but keeps M4 self-contained and prevents broken behavior.

### Finding 2 (Medium): `validate_metadata` and `KNOWN_KEYS` duplication

**Resolution**: Make `parser::KNOWN_KEYS` `pub` and import it in the
validator — single source of truth. Add a doc comment to `validate_metadata`
clarifying it expects raw `parse_frontmatter` output (the full HashMap
before known-key extraction), not `SkillProperties.metadata`.

### Finding 3 (Low): CJK characters and `is_lowercase()`

**Resolution**: Fix the character validation rule. CJK ideographs are
Unicode `Lo` (Letter, other) — they are not cased, so `is_lowercase()`
returns `false`. The correct rule is:

```rust
c.is_ascii_lowercase()
    || c.is_ascii_digit()
    || c == '-'
    || (c.is_alphabetic() && !c.is_uppercase())
```

This accepts CJK (alphabetic, not uppercase), Cyrillic lowercase
(alphabetic, not uppercase), and rejects Cyrillic uppercase.

### Finding 4 (Low): Reserved word substring vs segment matching

**Resolution**: Use hyphen-segment matching instead of substring:

```rust
name.split('-').any(|seg| RESERVED.contains(&seg))
```

This rejects `claude-tools` (segment `claude` matches) but accepts
`claudette` (single segment, not equal to `claude`). More precise
and avoids false positives. Add test #36 for a name containing a
reserved word as substring of a longer segment (e.g., `claudette`).

---

## Design Decisions

### Errors vs Warnings

The return type is `Vec<String>`. To distinguish errors from warnings:
- **Errors** are plain messages (e.g., `"name contains uppercase characters"`)
- **Warnings** are prefixed with `"warning: "` (e.g.,
  `"warning: body exceeds 500 lines"`)

This convention lets callers filter with `msg.starts_with("warning: ")`
without introducing a new type. Validation *fails* if there are any
non-warning messages.

`main.rs` is updated to filter warnings from errors — warnings are
printed but do not cause exit code 1.

### `validate_metadata` vs `validate`

- `validate_metadata(metadata, dir)` — validates a pre-parsed `HashMap`
  from raw `parse_frontmatter` output (before known-key extraction).
  The optional `dir` parameter enables the directory-name-matches-skill-name
  check (skipped when `None`).
- `validate(dir)` — full pipeline: `find_skill_md` → `read_to_string` →
  `parse_frontmatter` → `validate_metadata` + body-length check. If parsing
  fails, returns a single-element `Vec` with the parse error message.

### NFKC Normalization

The `name` field is NFKC-normalized before all checks. This means:
- `ﬁ` (ligature) → `fi`
- `Ⅲ` (Roman numeral) → `III` (which then fails lowercase check)
- `café` (precomposed) and `café` (decomposed) compare equal

Normalization happens at validation time, not at parse time — the parser
stores the raw value; the validator normalizes before checking.

### Name Validation Rules

After NFKC normalization, the name must:
1. Be non-empty
2. Be ≤ 64 characters
3. Contain only characters that are `[a-z0-9-]` or
   `(c.is_alphabetic() && !c.is_uppercase())` — accepts CJK, Cyrillic
   lowercase, rejects Cyrillic uppercase
4. Not start or end with a hyphen
5. Not contain consecutive hyphens (`--`)
6. Not contain reserved words (`anthropic`, `claude`) as hyphen-delimited
   segments (checked via `name.split('-').any(...)`)
7. Match the directory name (when `dir` is `Some`)

### XML Tag Rejection

Names and descriptions must not contain XML/HTML tags. Detection uses a
regex: `<[a-zA-Z/][^>]*>`. This catches `<script>`, `</div>`, `<img/>`,
etc. without false-positiving on `<` in normal text like `a < b`.

### Shared `KNOWN_KEYS`

The `parser::KNOWN_KEYS` constant is made `pub` and imported by the
validator. This is the single source of truth for known frontmatter keys.

---

## Wave 1 — Implementation

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m04-validator` | #14 | Implement validator in `src/validator.rs`; make `parser::KNOWN_KEYS` pub; fix `main.rs` warning handling |

**Merge**: A → `dev/m04`. Checkpoint with user.

### Agent A — Validator (#14)

#### Pre-requisite changes

1. In `src/parser.rs`: change `const KNOWN_KEYS` to `pub const KNOWN_KEYS`
2. In `src/lib.rs`: add `pub use parser::KNOWN_KEYS;` re-export
3. In `src/main.rs`: update `Validate` handler to filter warnings from
   errors — warnings print but don't cause exit code 1

#### Internal helpers

##### `validate_name(name: &str, dir: Option<&Path>) -> Vec<String>`

1. NFKC-normalize the name
2. Check non-empty → `"name must not be empty"`
3. Check ≤ 64 characters → `"name exceeds 64 characters"`
4. Check each character: must be `c.is_ascii_lowercase() || c.is_ascii_digit()
   || c == '-' || (c.is_alphabetic() && !c.is_uppercase())`
   → `"name contains invalid character: '{c}'"`
5. Check no leading hyphen → `"name must not start with a hyphen"`
6. Check no trailing hyphen → `"name must not end with a hyphen"`
7. Check no consecutive hyphens → `"name contains consecutive hyphens"`
8. Check reserved words (`anthropic`, `claude`) not present as hyphen-delimited
   segments: `normalized.split('-').any(|seg| RESERVED.contains(&seg))`
   → `"name contains reserved word: '{word}'"`
9. If `dir` is `Some`, compare normalized name to the directory's final
   component (also NFKC-normalized) → `"name '{name}' does not match
   directory name '{dir_name}'"`

##### `validate_description(description: &str) -> Vec<String>`

1. Check non-empty → `"description must not be empty"`
2. Check ≤ 1024 characters → `"description exceeds 1024 characters"`
3. Check no XML tags (regex `<[a-zA-Z/][^>]*>`) →
   `"description contains XML/HTML tags"`

##### `validate_compatibility(compatibility: &str) -> Vec<String>`

1. Check ≤ 500 characters → `"compatibility exceeds 500 characters"`

##### `contains_xml_tags(s: &str) -> bool`

Regex helper: returns `true` if string contains `<[a-zA-Z/][^>]*>`.
Compile regex once with `std::sync::LazyLock` (stable since Rust 1.80).

#### Public functions

##### `validate_metadata(metadata: &HashMap<String, Value>, dir: Option<&Path>) -> Vec<String>`

Expects raw `parse_frontmatter` output — the full HashMap before known-key
extraction. Not suitable for use on `SkillProperties.metadata` (which has
known keys already removed).

1. Extract `name` (must be a string) → validate with `validate_name`
2. Extract `description` (must be a string) → validate with `validate_description`
3. Extract `compatibility` if present (must be a string) → validate with
   `validate_compatibility`
4. Check for unexpected metadata keys: any key not in `parser::KNOWN_KEYS`
   → `"warning: unexpected metadata field: '{key}'"`
5. Collect and return all errors/warnings

##### `validate(dir: &Path) -> Vec<String>`

1. Call `find_skill_md(dir)` → if `None`, return `["SKILL.md not found"]`
2. Read the file → if IO error, return the error as a single message
3. Call `parse_frontmatter(&content)` → if error, return it as a single message
4. Call `validate_metadata(&metadata, Some(dir))` to get metadata errors
5. Count body lines: if > 500, append `"warning: body exceeds 500 lines
   ({n} lines)"`
6. Return collected errors/warnings

---

## Wave 2 — Tests (depends on Wave 1)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| B | `task/m04-tests` | #15 | Write validator tests in `src/validator.rs` `#[cfg(test)]` |

**Merge**: B → `dev/m04`. Checkpoint with user.

### Agent B — Tests (#15)

`src/validator.rs` — `#[cfg(test)] mod tests`

Test infrastructure:
- Use `tempfile::tempdir()` for temporary skill directories
- Helper function `make_skill_dir(name, frontmatter)` — creates a temp dir
  named `name` inside a parent temp dir, writes SKILL.md, returns `TempDir`
- Helper function `make_metadata(pairs)` — builds a `HashMap<String, Value>`
  from key-value pairs for `validate_metadata` tests

#### `validate_metadata` tests

| # | Test | Type |
|---|------|------|
| 1 | Valid metadata with all fields → empty errors | happy path |
| 2 | Missing name → error | error |
| 3 | Missing description → error | error |
| 4 | Empty name `""` → error | error |
| 5 | Empty description `""` → error | error |
| 6 | Name too long (65 chars) → error | error |
| 7 | Name exactly 64 chars → passes | boundary |
| 8 | Name with uppercase → error | error |
| 9 | Name with leading hyphen → error | error |
| 10 | Name with trailing hyphen → error | error |
| 11 | Name with consecutive hyphens → error | error |
| 12 | Name with invalid character (e.g., `_`, `!`) → error | error |
| 13 | Name contains reserved word `anthropic` as segment → error | error |
| 14 | Name contains reserved word `claude` as segment → error | error |
| 15 | Name does not match directory → error | error |
| 16 | Name matches directory → passes | happy path |
| 17 | Description too long (1025 chars) → error | error |
| 18 | Description exactly 1024 chars → passes | boundary |
| 19 | Description with XML tags → error | error |
| 20 | Name with XML tags → error | error |
| 21 | Compatibility too long (501 chars) → error | error |
| 22 | Compatibility exactly 500 chars → passes | boundary |
| 23 | Unexpected metadata field → warning (prefixed) | warning |
| 24 | All optional fields accepted without warning | happy path |

#### i18n / Unicode tests

| # | Test | Type |
|---|------|------|
| 25 | Chinese characters in name accepted | i18n |
| 26 | Russian lowercase with hyphens accepted | i18n |
| 27 | Uppercase Cyrillic in name rejected | i18n |
| 28 | NFKC normalization applied to name before checks | i18n |

#### `validate` (full pipeline) tests

| # | Test | Type |
|---|------|------|
| 29 | Valid skill directory → empty errors | happy path |
| 30 | Nonexistent path → error | error |
| 31 | Missing SKILL.md → error | error |
| 32 | Body > 500 lines → warning | warning |
| 33 | Body ≤ 500 lines → no warning | boundary |
| 34 | Multiple validation errors collected in one pass | aggregation |

#### Reserved word edge case tests

| # | Test | Type |
|---|------|------|
| 35 | Reserved word as substring of longer segment accepted (e.g., `claudette`) | edge case |
| 36 | Reserved word as exact segment rejected (e.g., `my-claude-tool`) | edge case |

---

## Wave 3 — Verify (depends on Wave 2)

Single agent runs the full check suite on `dev/m04`.

| Agent | Branch | Task |
|-------|--------|------|
| C | `dev/m04` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Deliverables

- `src/validator.rs` — `validate_metadata` and `validate` implemented with
  helpers for name, description, compatibility, XML detection
- `src/parser.rs` — `KNOWN_KEYS` made `pub`
- `src/lib.rs` — re-export `KNOWN_KEYS`
- `src/main.rs` — warning-aware exit code handling
- 36 tests inline in `#[cfg(test)] mod tests`
- PR: `M4: Validator`
