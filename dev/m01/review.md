# Code Review — `dev/m01` (M1: Project Scaffolding)

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Commits:** `eb4a73e` M1: Project scaffolding, `7362fcd` fix: correct author name
**Status:** Builds clean, clippy passes, formatting OK

---

## Overall Assessment

The scaffolding is well-structured. Module layout matches the architecture documented in
CLAUDE.md, all public items have doc comments, no `unwrap()` in library code, CI is
comprehensive (multi-OS matrix), and lefthook pre-commit hooks are properly configured.
The issues below are all fixable within the M1 scope.

---

## Critical Issues

### 1. Deprecated `serde_yaml` crate

**Files:** `Cargo.toml:25`, `models.rs:20`, `errors.rs:20`, `parser.rs:22`, `validator.rs:8`

`serde_yaml 0.9` is archived and deprecated by its maintainer. The dependency is woven
through public API types (`SkillProperties.metadata`, `AigentError::Yaml`,
`parse_frontmatter` return type, `validate_metadata` parameter).

**Recommendation:** Replace with `serde_yml` (the community-recommended successor) before
these types become part of a stable public API. Deferring makes the migration harder
since downstream consumers would also need to change.

### 2. Missing `#[must_use]` annotations

**Files:** `parser.rs`, `prompt.rs`, `validator.rs`, `builder.rs`

CLAUDE.md requires `#[must_use]` on functions returning values that shouldn't be ignored.
Every public function in these modules returns a meaningful value but lacks the attribute:

| Function | File |
|---|---|
| `find_skill_md` | `parser.rs:7` |
| `parse_frontmatter` | `parser.rs:22` |
| `read_properties` | `parser.rs:27` |
| `xml_escape` | `prompt.rs:4` |
| `to_prompt` | `prompt.rs:12` |
| `validate_metadata` | `validator.rs:7` |
| `validate` | `validator.rs:17` |
| `build_skill` | `builder.rs:33` |
| `derive_name` | `builder.rs:38` |
| `assess_clarity` | `builder.rs:43` |

### 3. Misplaced import in `parser.rs`

**File:** `parser.rs:31`

`use std::collections::HashMap;` is placed at the bottom of the file, after all function
definitions. It should be grouped with the other imports at lines 1–4. This is not just a
style issue — it makes the module harder to scan and could confuse contributors.

---

## Minor Issues

### 4. Edge case in `resolve_skill_dir`

**File:** `main.rs:128–136`

When `path.is_file()` is true but `path.parent()` returns `None`, the fallback returns
the file path itself rather than a directory. This is a near-impossible scenario (only
root-level files on some systems), but the silent fallback masks the error. Consider
returning an error or using `PathBuf::from(".")` as the fallback.

```rust
// Current
path.parent()
    .map(|p| p.to_path_buf())
    .unwrap_or_else(|| path.to_path_buf())

// Suggested
path.parent()
    .map(|p| p.to_path_buf())
    .unwrap_or_else(|| PathBuf::from("."))
```

### 5. Redundant `debug/` in `.gitignore`

**File:** `.gitignore:3`

`target/` already covers `target/debug/`. A standalone `debug/` entry only matters if
debug output is generated outside `target/`, which Cargo doesn't do. Consider removing it
to reduce confusion.

### 6. Untracked `lib/` directory

**Git status:** `?? lib/`

The working tree contains `lib/.precomp/` with cache files. These are not committed and
not in `.gitignore`. Add `lib/` to `.gitignore` if these are build/cache artifacts.

---

## What's Good

- **Module structure** matches CLAUDE.md architecture exactly
- **Doc comments** on all public items (structs, enums, functions, type aliases)
- **Error design** — `AigentError` with thiserror, `Result<T>` alias, `#[from]` for IO/YAML
- **CI matrix** — ubuntu, macos, windows with fmt → clippy → test → release build
- **Pre-commit hooks** — parallel fmt + clippy checks, pre-push test + clippy
- **Cargo.toml** — clean metadata, dual lib+bin targets, appropriate categories/keywords
- **No `unwrap()` in library code** — stubs use `todo!()` which is correct for scaffolding
- **`xml_escape`** is the one implemented function; it correctly handles `& < > "`

---

## Checklist for Merge

- [ ] Replace `serde_yaml` with `serde_yml` (or defer with tracked issue)
- [ ] Add `#[must_use]` to all public value-returning functions
- [ ] Move `HashMap` import to top of `parser.rs`
- [ ] Add `lib/` to `.gitignore`
- [ ] Consider removing `debug/` from `.gitignore`
- [ ] Consider hardening `resolve_skill_dir` fallback
