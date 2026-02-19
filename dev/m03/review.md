# Review of `dev/m03/plan.md`

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Scope:** Pre-implementation plan review for M3: Parser
**References:** Issues #12, #13; current `src/parser.rs` on `main`

---

## Overall Assessment

Well-structured plan with a clear test-first approach and detailed implementation
pseudocode. The separation between parser (M3, structural correctness) and validator
(M4, semantic rules) is well-articulated. Three issues found — one medium, two low.

---

## Findings

### 1. Medium: `parse_frontmatter` wraps YAML errors as `AigentError::Parse`, but the error type already has `AigentError::Yaml`

**References:** `plan.md:107,114`

The plan says all errors from `parse_frontmatter` use `AigentError::Parse { message }`,
including invalid YAML syntax (step 3: "error if YAML is invalid"). However,
`AigentError` already has a dedicated `Yaml(#[from] serde_yaml_ng::Error)` variant
with automatic `?` conversion.

This creates a design tension:

- If `parse_frontmatter` uses `serde_yaml_ng::from_str(yaml_str)?`, the `?` operator
  will automatically convert to `AigentError::Yaml` (via `#[from]`). To produce
  `AigentError::Parse` instead, the implementation must `.map_err()` on the YAML
  error, discarding the structured `serde_yaml_ng::Error` in favor of a string.
- If `read_properties` reads the file with `std::fs::read_to_string(path)?`, the
  `?` operator will automatically convert to `AigentError::Io` (via `#[from]`).
  This is acknowledged in the plan (step 2, line 122) and is correct.

The plan should state explicitly which approach to use for YAML parse failures:

**(a) Use `AigentError::Yaml` (let `?` work naturally):**
- Pro: Preserves the structured error from serde (line/column info). Consistent
  with the existing `#[from]` design.
- Con: Callers who match on error variants get `Parse` for structural issues
  (missing delimiters) but `Yaml` for syntax issues — two error types for what
  the caller perceives as "bad SKILL.md".

**(b) Wrap everything as `AigentError::Parse` (plan's stated approach):**
- Pro: Single error variant for all `parse_frontmatter` failures. Simpler matching.
- Con: Loses structured YAML error info. Requires `.map_err()` boilerplate. Makes
  the `Yaml` variant unused until a later module uses `serde_yaml_ng` directly.

**(c) Hybrid — `Parse` for structural issues, `Yaml` for syntax:**
- Pro: Each variant means what it says.
- Con: Callers must handle both.

**Recommendation:** Pick one and document it. Option (a) or (c) are most idiomatic
for Rust error handling. If (b) is chosen, test #9 should verify the error message
includes the YAML parser's line/column info so it isn't lost entirely.

### 2. Low: No test for `---` inside the YAML block itself

**Reference:** `plan.md:60-72`

The plan tests missing `---`, unclosed `---`, and invalid YAML, but does not test
a frontmatter block where the YAML content itself contains `---` (the YAML document
separator). For example:

```yaml
---
name: my-skill
description: >
  This describes a skill
  ---
  with dashes in the description
---
```

Depending on delimiter detection strategy (line-by-line exact match vs. substring),
a `---` embedded in a multiline YAML value could be mistaken for the closing
delimiter. This is a classic frontmatter parsing edge case.

**Recommendation:** Add a test case for YAML content containing `---` in a
multiline string value. The plan's step-by-step (line 103-104: "a line that is
exactly `---`") implies exact-match scanning, which would handle this correctly
for indented content but fail for a `---` that appears at column 0 inside a
literal block scalar. Documenting the exact matching rule and testing it would
prevent ambiguity.

### 3. Low: `read_properties` metadata extraction — "remaining keys" logic is under-specified

**References:** `plan.md:128,135-139`

The plan says known keys (`name`, `description`, `license`, `compatibility`,
`allowed-tools`) are extracted into typed fields and "everything else goes into
the `metadata` HashMap" (step 6, line 128). However, there are ambiguities:

- **What if a known key has an unexpected type?** For example, `name: [a, b]`
  (a YAML list instead of a string). Should this be a validation error, or
  should it be silently placed into `metadata`?
- **What happens to `description` in `metadata`?** After extracting `description`
  into the typed field, it should NOT also appear in the `metadata` map. The plan
  implies this but doesn't state it explicitly.
- **What about `allowed-tools` vs `allowed_tools`?** The YAML key is kebab-case
  (`allowed-tools`). The plan should confirm that the extraction looks for the
  kebab-case key in the HashMap (not snake_case), since `serde_yaml_ng` preserves
  the original key names in a `HashMap<String, Value>`.

**Recommendation:** Add a test that verifies known keys do NOT appear in the
`metadata` field after extraction (e.g., assert that `metadata` doesn't contain
"name" or "description"). The current test #15 ("preserves nested metadata values")
partially covers this but doesn't assert the absence of known keys. Also clarify
the behavior for type mismatches on known keys.

---

## Observations (not issues)

### Test-first ordering

The plan puts tests in Wave 1 and implementation in Wave 2 ("tests first — they
define the contract"). This is a good TDD approach. The tests will initially fail
(calling `todo!()` stubs), and Wave 2 makes them pass.

### `find_skill_md` test coverage

The four `find_skill_md` tests (lines 55-58) cover all cases: uppercase exists,
lowercase only, both exist (prefer uppercase), neither exists. This is thorough
for a function that's already implemented.

### Error variant choice for `read_properties`

The plan correctly uses different error variants for different failure modes:
- Missing SKILL.md → `Parse` (structural problem)
- IO failure → `Io` (automatic via `#[from]`)
- Missing required fields → `Validation` (semantic problem)

This is a good separation that will compose well with M4's validator.

### Scope boundary with M4

The plan explicitly notes (line 132-134) that the parser checks only minimum
requirements (name and description exist) and defers full validation to M4.
This is a clean separation. However, the plan should consider whether
`read_properties` should also check that `name` and `description` are
non-empty strings (as stated in step 4, line 125-126: "non-empty string").
An empty `name: ""` is arguably a validation concern, not a parsing concern.
If M3 enforces non-empty, document it; if not, remove "non-empty" from the step
description and let M4 handle it.

### Issue #12 vs actual scope

Issue #12 lists `find_skill_md` as part of the implementation, but the plan
correctly notes it's already implemented from M1. The issue description is stale
in this regard — not a plan problem, just worth noting.

---

## Checklist for Plan Finalization

- [ ] Decide on YAML error handling: `Parse` vs `Yaml` vs hybrid (finding #1)
- [ ] Add test for `---` inside YAML content (finding #2)
- [ ] Add test asserting known keys are absent from `metadata` (finding #3)
- [ ] Clarify behavior for type mismatches on known keys (finding #3)
- [ ] Resolve whether "non-empty" check for name/description belongs in M3 or M4
