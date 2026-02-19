# M2: Core Data Model & Errors — Work Plan

## Overview

Refine the error display formatting and add test coverage for the error types
and data model.
Issues: #9, #10, #11.

## Branch Strategy

- **Dev branch**: `dev/m02` (created from `main`)
- **Task branches**: `task/m02-<name>` (created from `dev/m02`)
- After each wave, task branches merge into `dev/m02`
- After all waves, PR from `dev/m02` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Current State

Both `errors.rs` and `models.rs` already contain full struct/enum definitions
from M1 scaffolding. `#[must_use]` was addressed in the M1 review fix.

The remaining work is:
1. Refine `AigentError::Validation` display formatting
2. Add `PartialEq` derive to `SkillProperties`
3. Write comprehensive tests for both modules

---

## Wave 1 — Implementation (parallel)

Errors and Models changes are independent — both can be implemented in parallel.

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m02-errors` | #9 | Refine `Validation` display in `src/errors.rs` |
| B | `task/m02-models` | #10 | Add `PartialEq` derive in `src/models.rs` |

**Merge**: A, B → `dev/m02`. Checkpoint with user.

### Agent A — Errors (#9)

Replace `#[error("validation failed")]` on the `Validation` variant with a
`thiserror` format-function pattern:

```rust
#[error("{}", format_validation_errors(errors))]
Validation { errors: Vec<String> },
```

Add a helper `fn format_validation_errors(errors: &[String]) -> String`:
- Empty errors  → `"validation failed: no details"`
- Single error  → the error message itself
- Multiple errors → `"Validation failed:\n  - err1\n  - err2"`

This keeps all variants under `thiserror`'s `#[derive(Error)]` — no manual
`Display` impl needed.

**Not in scope:** `PartialEq` on `AigentError`. Neither `std::io::Error` nor
`serde_yaml_ng::Error` implements `PartialEq`, so the enum cannot derive it.
Tests will use `.to_string()` and pattern matching instead.

### Agent B — Models (#10)

- Add `PartialEq` to the derive list for `SkillProperties`.
  `serde_yaml_ng::Value` implements `PartialEq`, so this compiles.
- No other code changes needed — the struct is already complete from M1.

---

## Wave 2 — Tests (depends on Wave 1)

Tests import the modules and verify Wave 1 changes. Both test modules are
independent and can be written in parallel.

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| C | `task/m02-error-tests` | #11 | Write error tests in `src/errors.rs` `#[cfg(test)]` |
| D | `task/m02-model-tests` | #11 | Write model tests in `src/models.rs` `#[cfg(test)]` |

**Merge**: C, D → `dev/m02`. Checkpoint with user.

### Agent C — Error tests (part of #11)

`src/errors.rs` — `#[cfg(test)] mod tests`

| # | Test |
|---|------|
| 1 | Construct `Parse`, verify `.to_string()` output |
| 2 | Construct `Validation` with single error, verify message is the error itself |
| 3 | Construct `Validation` with multiple errors, verify bullet format |
| 4 | Construct `Validation` with empty errors, verify fallback message |
| 5 | Construct `Build`, verify `.to_string()` output |
| 6 | `std::io::Error` converts to `AigentError::Io` via `?` |
| 7 | `serde_yaml_ng::Error` converts to `AigentError::Yaml` via `?` |
| 8 | `Validation.errors` holds multiple strings accessible via pattern match |
| 9 | Pattern match on `Parse` extracts message |
| 10 | `AigentError` implements `std::error::Error` trait |
| 11 | `Result<T>` alias works with `?` operator |

### Agent D — Model tests (part of #11)

`src/models.rs` — `#[cfg(test)] mod tests`

| # | Test |
|---|------|
| 1 | Construct with required fields only |
| 2 | Construct with all fields |
| 3 | Serialize to JSON — optional fields omitted when `None` |
| 4 | Serialize to JSON — license included when `Some` |
| 5 | Serialize to JSON — all optional fields included |
| 6 | Serialize to JSON — metadata excluded when `None` |
| 7 | Serialize to JSON — metadata included when `Some` |
| 8 | Deserialize from YAML — all fields |
| 9 | `allowed-tools` kebab-case round-trip (YAML → struct → JSON) |
| 10 | Field accessors: `.name`, `.description` |
| 11 | Field accessor: `.allowed_tools` |
| 12 | Field accessor: `.metadata` |
| 13 | Deserialize from YAML — required fields only, optionals are `None` |
| 14 | Deserialize from YAML — missing `name` fails |
| 15 | Deserialize from YAML — missing `description` fails |

---

## Wave 3 — Verify (depends on Wave 2)

Single agent runs the full check suite on `dev/m02`.

| Agent | Branch | Task |
|-------|--------|------|
| E | `dev/m02` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Deliverables

- `src/errors.rs` — `Validation` display via `format_validation_errors` helper
- `src/models.rs` — `PartialEq` derive added
- Tests inline in both modules (`#[cfg(test)] mod tests`)
- PR: `M2: Core Data Model & Errors`
