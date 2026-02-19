## Review of `dev/m02/plan.md`

### Findings

1. Medium: Conflicting direction on `AigentError` `PartialEq` will cause rework.
   - References: `dev/m02/plan.md:15`, `dev/m02/plan.md:30`, `dev/m02/plan.md:33`
   - The plan first states `PartialEq` derive for `AigentError` is required, then later says to avoid it and prefer string-based assertions. These are mutually exclusive implementation paths.
   - Recommended fix: Pick one acceptance path (derive/manual `PartialEq` vs `Display`-based tests) and make it consistent across Current State, Wave 1, and Deliverables.

2. Medium: Test case #13 is internally contradictory, so it is not implementable as written.
   - Reference: `dev/m02/plan.md:85`
   - It says to deserialize YAML with only required fields and assert optionals are `None`, but the mapping text says construction fails without required fields. Those are different tests.
   - Recommended fix: Split into two explicit tests:
     - valid input with only required fields deserializes and optional fields are `None`;
     - invalid input missing `name` or `description` fails.

3. Low: Section title and body disagree on scope for models work.
   - References: `dev/m02/plan.md:35`, `dev/m02/plan.md:41`
   - The header promises `PartialEq + Default-like constructor`, but the body says no additional changes are needed.
   - Recommended fix: either remove "Default-like constructor" from the title or add concrete constructor requirements/tests.

4. Low: Required vs optional status of `SkillProperties` `PartialEq` is inconsistent.
   - References: `dev/m02/plan.md:16`, `dev/m02/plan.md:101`
   - Current State treats `PartialEq` as required; Deliverables mark it as "if feasible". This weakens closure criteria for #10.
   - Recommended fix: define one criterion for #10 closure and keep it uniform.

### Open Questions

1. Is issue #10 intended to require `PartialEq` on `SkillProperties`, or is serialization-based assertion an accepted substitute?
2. Is implementing `PartialEq` on `AigentError` still in scope for M2, or should tests avoid equality checks for non-comparable variants (`Io`, `Yaml`)?

### Summary

The plan is close, but it has requirement conflicts that should be resolved before implementation. Clarifying these now should prevent test churn and scope drift during M2 delivery.

---

## Supplemental Review

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Scope:** Deeper analysis of plan against actual codebase state on `main` (post-M1 merge)

### Verified Facts

The following items were verified against the actual crate source and dependency documentation
to resolve open questions in the plan:

1. **`serde_yaml_ng::Value` implements `PartialEq`** — confirmed via source at
   `value/mod.rs:25`: `#[derive(Clone, PartialEq, PartialOrd)]`. This means
   `SkillProperties` **can** derive `PartialEq` without workarounds.
   The plan's hedging ("verify; if not, test via serialization") is unnecessary.

2. **`serde_yaml_ng::Error` does NOT implement `PartialEq`** — confirmed.
   Neither does `std::io::Error`. This means `AigentError` **cannot** use
   `#[derive(PartialEq)]`. The plan's preferred approach (`.to_string()` comparisons)
   is the correct call.

3. **The crate already uses `serde_yaml_ng` 0.10**, not the deprecated `serde_yaml`.
   The plan references `serde_yaml_ng` correctly in some places, but issues #9 and #10
   still reference `serde_yaml` in their code blocks. This is cosmetic — the actual
   Cargo.toml on `main` is correct — but the issue descriptions are stale.

4. **Raku reference tests (`t/01-errors.rakutest`, `t/02-models.rakutest`) are not in
   the repo.** The plan references them as the porting source, which is fine for design
   context, but the implementer will need access to the original Raku project or must
   treat the plan's test table as the authoritative specification.

### Additional Findings

5. **Medium: `Validation` display format has an edge case for zero errors.**
   - Reference: `dev/m02/plan.md:28-29`
   - The plan defines behavior for single and multiple errors but not for an empty
     `errors: Vec<String>`. Since `Vec<String>` can be empty, the custom Display impl
     should either:
     (a) handle it (e.g., `"validation passed"` or `"validation failed: no details"`), or
     (b) make it a documented invariant that `errors` is never empty (enforce in constructor).
   - Recommended fix: add an empty-errors test case to the test table, or add a note that
     construction with `errors: vec![]` is considered a programming error.

6. **Low: Missing `#[must_use]` tracking.**
   - The M1 review (`dev/m01/review.md`) flagged missing `#[must_use]` annotations on
     all public value-returning functions. The M2 plan doesn't mention addressing this.
   - Since M2 touches `errors.rs` and `models.rs`, it would be natural to add
     `#[must_use]` to the `Result<T>` type alias here. However, this could also be
     deferred to a dedicated cleanup pass.
   - Recommended: explicitly note whether M2 will or will not address `#[must_use]`.

7. **Low: Test numbering assumes specific Raku test semantics.**
   - Tests #5 and #6 (error conversion via `?`) are idiomatic Rust tests, not direct
     Raku ports. The plan acknowledges this for #4 ("no Raku equivalent, Rust addition")
     but not for #5–#6, which have Raku column entries like `#[from] conversion` that
     don't exist in the Raku codebase. This is confusing rather than wrong — the mapping
     column should say "Rust-specific" for these.

8. **Observation: `Validation` variant's custom Display conflicts with `thiserror` derive.**
   - The plan says to change from `#[error("validation failed")]` to a custom `Display`
     impl or `thiserror` format function. If using a manual `Display` impl, you must
     remove the `#[error(...)]` attribute from the `Validation` variant entirely, or
     `thiserror` will generate a conflicting impl. The cleaner approach is to use
     `thiserror`'s `#[error(/* ... */)]` with a helper function:
     ```rust
     #[error("{}", format_validation_errors(errors))]
     Validation { errors: Vec<String> },
     ```
     This keeps all variants under `thiserror`'s control and avoids a partial-derive
     situation.

### Resolved Open Questions (from original review)

> 1. Is issue #10 intended to require `PartialEq` on `SkillProperties`?

**Answer: Yes, it's feasible.** `serde_yaml_ng::Value` implements `PartialEq`, so
`#[derive(PartialEq)]` on `SkillProperties` will compile. The plan should be updated
to state this as a firm requirement, not conditional.

> 2. Is implementing `PartialEq` on `AigentError` still in scope?

**Answer: No, and it shouldn't be.** Neither `std::io::Error` nor `serde_yaml_ng::Error`
implements `PartialEq`. A manual impl that skips those variants would be misleading.
The plan's preferred approach (`.to_string()` + pattern matching in tests) is correct.
Drop this from the plan requirements entirely.

### Updated Checklist for Plan Finalization

- [ ] Remove `PartialEq` derive from `AigentError` scope; confirm `.to_string()` test strategy
- [ ] Make `PartialEq` derive on `SkillProperties` a firm requirement (not conditional)
- [ ] Define behavior for `Validation { errors: vec![] }` (or document the invariant)
- [ ] Fix test #13 ambiguity (split into positive/negative cases, per original review)
- [ ] Clean up "Default-like constructor" title mismatch (per original review)
- [ ] Decide whether M2 addresses `#[must_use]` or defers it
- [ ] Note that `thiserror` `#[error]` attribute must be removed or replaced (not supplemented with manual `Display`)

---

## Code Review — `dev/m02` branch

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Commit:** `d1ce04e` M2: Implement errors and models with tests
**Scope:** Implementation review of `src/errors.rs` and `src/models.rs` changes
**Verification:** `cargo fmt --check` ✓, `cargo clippy -- -D warnings` ✓, `cargo test` ✓ (26/26 pass)

---

### Overall Assessment

Clean implementation that faithfully follows the revised plan. All plan items
are addressed, all prior review findings are resolved, and the code is idiomatic
Rust. No critical issues found. Two minor issues and several observations below.

---

### Plan Conformance

Every plan item maps to implemented code:

| Plan item | Status | Evidence |
|-----------|--------|----------|
| `Validation` display via `format_validation_errors` | ✓ | `errors.rs:11,32-41` |
| Empty/single/multi error formatting | ✓ | Tests at lines 59, 67, 78 |
| `PartialEq` on `SkillProperties` | ✓ | `models.rs:5` |
| No `PartialEq` on `AigentError` | ✓ | Tests use `.to_string()` and `matches!()` |
| Error tests: 11 planned → 11 implemented | ✓ | `errors::tests::*` |
| Model tests: 15 planned → 15 implemented | ✓ | `models::tests::*` |

Prior review checklist resolution:

| Checklist item | Status |
|----------------|--------|
| Remove `PartialEq` derive from `AigentError` scope | ✓ Plan updated, not derived |
| Make `PartialEq` on `SkillProperties` firm | ✓ Derived at `models.rs:5` |
| Define behavior for `Validation { errors: vec![] }` | ✓ Handled + tested (`errors.rs:34,78-81`) |
| Fix test #13 ambiguity (split positive/negative) | ✓ Split into tests #13, #14, #15 |
| Clean up "Default-like constructor" title | ✓ Plan revised |
| `#[must_use]` tracking | ✓ Plan notes M1 review fix addressed it |
| `thiserror` format-function approach | ✓ Used exactly as recommended |

---

### Findings

#### 1. Minor: Inconsistent casing in `Validation` display output

**File:** `errors.rs:34,38`

The empty-errors case uses lowercase `"validation failed: no details"` while the
multi-error case uses title case `"Validation failed:\n  - ..."`. The single-error
case passes through the raw error string (which could be any casing).

```
0 errors → "validation failed: no details"  ← lowercase
1 error  → "{error}"                        ← passthrough
N errors → "Validation failed:\n..."        ← title case
```

This is a design choice, not a bug — but consumers who pattern-match on the prefix
string will need to handle both cases. If this is intentional (e.g., the empty-errors
case is a "shouldn't happen" sentinel distinct from the real multi-error message),
document it in the doc comment. If not, make the casing consistent.

**Severity:** Minor — no functional impact, but a potential surprise for downstream
code that checks `err.to_string().starts_with("Validation")`.

#### 2. Minor: `PartialEq` on `SkillProperties` is derived but never exercised in tests

**File:** `models.rs:5`

`PartialEq` was added to the derive list (correctly resolving the plan item), but no
test actually uses `assert_eq!(sp1, sp2)` with two `SkillProperties` instances. All
tests compare individual fields or use serde round-trips. This means the derive is
untested — if a future refactor breaks `PartialEq` compatibility (e.g., by adding a
field with a non-`PartialEq` type), the failure would only be caught at compile time
rather than via a test that demonstrates the intended usage.

**Suggestion:** Add one test that constructs two identical `SkillProperties` and
asserts equality, and one that asserts inequality when a field differs. This documents
the intent and exercises the derive.

---

### Observations (not issues)

#### Test quality

- **Error tests** use a clean pattern: construct-then-assert for display, inner
  functions with `?` for `#[from]` conversion, and `matches!()` for variant matching.
  No `unwrap()` in library code; `unwrap()` / `unwrap_err()` only in tests (per
  CLAUDE.md convention).

- **Model tests** use well-named helpers (`minimal_props`, `full_props`) that avoid
  repetition. The `as_deref()` idiom in `field_accessor_allowed_tools` (line 161)
  is idiomatic for comparing `Option<String>` against `Option<&str>`.

- **Negative tests** (`deserialize_yaml_missing_name_fails`,
  `deserialize_yaml_missing_description_fails`) correctly test that serde rejects
  incomplete YAML. They assert `is_err()` rather than matching a specific error
  message, which is appropriate since the error text comes from `serde_yaml_ng`.

#### `format_validation_errors` visibility

The helper is `fn` (private to the module), which is correct — it's an implementation
detail of the `Display` formatting. It doesn't need `pub` or `#[must_use]`.

#### `#[must_use]` on `Result<T>` type alias

The plan notes that `#[must_use]` was addressed in the M1 review fix. However,
`errors.rs:44` defines `pub type Result<T>` without `#[must_use]`. In practice this
is fine because `std::result::Result` already has `#[must_use]` on the type itself,
so the alias inherits it. No action needed — just noting for completeness.

#### Remaining `#[must_use]` gaps in other files (not M2 scope)

`parser.rs:24` (`parse_frontmatter`) and `parser.rs:31` (`read_properties`) still
lack `#[must_use]`. These are stubs returning `Result`, which inherits `#[must_use]`
from `std::result::Result`, so the gap is cosmetic. Similarly `builder.rs:34`
(`build_skill`). These should be picked up when those modules are implemented (M3/M7).

---

### Verdict

**Ready to merge.** No blocking issues. The two minor findings (casing inconsistency,
untested `PartialEq` derive) are polish items that could be addressed in this PR or
deferred — neither affects correctness or the ability to close #9, #10, #11.
