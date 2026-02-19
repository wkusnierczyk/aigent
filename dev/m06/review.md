# Review of `dev/m06/plan.md`

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Scope:** Pre-implementation plan review for M6: CLI
**References:** Issues #18, #19, #20; current `src/main.rs` on `main`;
`Cargo.toml` dev-dependencies; M4 review (warning/exit code fix)

---

## Overall Assessment

Well-scoped plan for a milestone that is primarily about *testing* existing code
rather than writing new code. The design decisions section is unusually
self-aware — it acknowledges that most of Wave 1 is verification, not
implementation. The 18-test integration suite is comprehensive for the CLI
surface area. Two medium findings, two low findings, and observations below.

---

## Findings

### 1. Medium: `--about` output format diverges from issue #20

**References:** `plan.md:113-117`; issue #20; `main.rs:111-124`

The current `print_about` implementation produces:

```
aigent: Rust AI Agent Skills Tool
├─ version:    0.1.0
├─ authors:    Wacław Kuśnierczyk
├─ source:     https://github.com/wkusnierczyk/aigent
└─ license:    MIT https://opensource.org/licenses/MIT
```

Issue #20 specifies:

```
aigent: Rust AI Agent Skills Tool
├─ version:    <CARGO_PKG_VERSION>
├─ author:     <CARGO_PKG_AUTHORS>
├─ source:     <CARGO_PKG_REPOSITORY>
└─ license:    <CARGO_PKG_LICENSE>
```

Two differences:

**(a) `authors:` vs `author:`** — The implementation uses `authors:` (plural),
matching the `Cargo.toml` field name (`[package] authors = [...]`). The issue
specifies `author:` (singular). This affects integration test assertions
(test #3 checks for `"aigent:"` in the output, but specific field labels
matter).

**(b) License URL suffix** — The implementation appends
`https://opensource.org/licenses/MIT` after the license name. The issue shows
only the license identifier (`MIT`). The extra URL is arguably helpful but
diverges from the spec.

The plan says "Verify `--about` output format" (Wave 1, step 1) and "Confirm
it includes the license URL suffix." This confirms awareness of (b) but
doesn't address (a).

**Recommendation:** Decide which format is canonical — the issue spec or the
current implementation. If the issue is canonical, fix the two discrepancies.
If the current implementation is preferred (reasonable choices — plural
`authors` matches Cargo.toml, URL suffix is useful), update the issue. Either
way, the integration test (test #3) must match whatever format is chosen.

### 2. Medium: `validate` on valid skill with warnings → exit 0 but stderr non-empty

**References:** `plan.md:178-183`; test #5 vs test #8

Test #5 says: "Valid skill directory → exit 0" with "success, no stderr."
Test #8 says: "Warnings only (e.g., body > 500 lines) → exit 0" with
"success, stderr contains 'warning:'."

The issue is test #5's assertion "no stderr." If the valid skill happens to
trigger unexpected-metadata-key warnings (e.g., because the test SKILL.md has
non-standard fields), stderr would be non-empty even though the exit code is 0.
The assertion "no stderr" is fragile — it requires the test fixture to produce
*exactly zero* warnings.

**Options:**

**(a)** Test #5 uses a fixture with only `name` and `description` (no
unexpected fields, body ≤ 500 lines) to guarantee zero warnings. This is
the simplest fix.

**(b)** Change test #5's assertion from "no stderr" to "exit 0 and no
non-warning stderr" (i.e., allow warnings but no errors). More resilient but
harder to express with `predicates`.

**(c)** Keep "no stderr" but document that the fixture must be minimal.

**Recommendation:** Option (a). Use a minimal fixture:
```yaml
---
name: my-skill
description: A test skill
---
Short body.
```
This produces zero warnings (no unknown keys, body < 500 lines), making the
"no stderr" assertion reliable. The plan's test infrastructure section should
explicitly note that test #5's fixture must be warning-free.

### 3. Low: Test #9 and #12 — `resolve_skill_dir` tested through CLI, not unit-tested

**References:** `plan.md:183,191`

Tests #9 and #12 verify that passing a SKILL.md file path works the same as
passing the directory path. This exercises `resolve_skill_dir` through the CLI,
which is good. However, `resolve_skill_dir` itself has no unit tests — it's a
private function in `main.rs` with three code paths:

1. `path.is_file()` → return parent
2. `path.is_file()` and parent is `None` (root file) → return `.`
3. `!path.is_file()` → return path as-is

Path 2 is nearly impossible to trigger in practice (a file with no parent
directory), but path 3 covers both directories *and* nonexistent paths. The
integration tests cover paths 1 and 3. Path 2's `unwrap_or_else` fallback
is untested but safe.

**Recommendation:** No change needed — integration tests sufficiently cover
the realistic cases. If the function were `pub`, unit tests would be warranted,
but it's `fn resolve_skill_dir` (private to `main.rs`).

### 4. Low: `to-prompt` with no directories (test #15) assertion specificity

**References:** `plan.md:199`

Test #15 asserts stdout is `<available_skills>\n</available_skills>`. But
`main.rs` uses `println!("{}", aigent::to_prompt(&dirs))`, which adds a
trailing newline. So the actual stdout is:

```
<available_skills>\n</available_skills>\n
```

The assertion needs to account for the trailing newline from `println!`.
`assert_cmd`'s `stdout()` includes the trailing newline in the captured
output. Using `predicate::eq("...")` would need the exact string including
the trailing `\n`. Using `predicate::str::contains(...)` would be more
forgiving.

**Recommendation:** Use `predicate::str::trim()` + `predicate::eq()` to
compare trimmed output, or use `contains("<available_skills>")` +
`contains("</available_skills>")` and verify no `<skill>` block. The plan
should specify which assertion style to use for exact-match tests.

---

## Observations (not issues)

### Wave 1 is mostly a verification pass

The plan explicitly acknowledges that Wave 1 involves minimal or no code
changes. This is honest and correct — the CLI was already implemented in M1
and fixed in M4. M6's value is the integration test suite, not the CLI code.
This makes Wave 1 a "confirm nothing is broken" step, which is still valuable
as it forces a deliberate review before writing tests.

### `assert_cmd` + `predicates` is the idiomatic choice

The `assert_cmd` crate is the standard Rust library for CLI integration
testing. It compiles and runs the binary, captures stdout/stderr/exit code,
and integrates with the `predicates` crate for fluent assertions. The
dev-dependencies are already declared with reasonable major versions
(`assert_cmd = "2"`, `predicates = "3"`). This is the right tooling choice.

### Test count is proportional

18 tests for 5 subcommands + flags is reasonable:
- 4 tests for global flags (`--help`, `--version`, `--about`, no-args)
- 5 tests for `validate`
- 3 tests for `read-properties`
- 4 tests for `to-prompt`
- 2 tests for unimplemented stubs

This covers happy paths, error paths, and edge cases (SKILL.md file path
resolution, mixed valid/invalid inputs). The `build` and `init` stub tests
are forward-looking — they establish baseline behavior that M7 will change.

### Parallel test execution consideration

`assert_cmd` tests run as separate processes, which means they're naturally
parallelized by `cargo test`. Since each test creates its own `TempDir`,
there's no shared state. The `make_skill_dir` helper returns ownership of
the `TempDir` so it's cleaned up when the test ends. This is safe for
parallel execution.

### No `lib.rs` or library code changes

The plan correctly identifies that this is a main.rs + tests-only milestone.
No library modules are modified. The only source file that might change is
`main.rs` (if Wave 1 finds discrepancies), and even that is unlikely.

---

## Checklist for Plan Finalization

- [ ] Decide on `--about` format: issue #20 spec vs current implementation (finding #1)
- [ ] Ensure test #5 fixture produces zero warnings (finding #2)
- [ ] Specify assertion style for exact-match stdout tests (finding #4)
