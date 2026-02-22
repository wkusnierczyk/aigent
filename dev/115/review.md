# Review: Show diff in `format --check` output (#115)

Review of `dev/115/plan.md` against the current codebase (`dev/m14`
branch at `35bcd88`).

---

## Verdict

The plan is clean, well-scoped, and accurate. All code location claims
verified. The design choice to embed the original in `FormatResult`
avoids a re-read and is the right call. Two design points to consider
before execution (§3.1 and §3.2).

---

## 1. Code Location Accuracy

| Claim | Status | Notes |
|-------|:------:|-------|
| `FormatResult` struct at `formatter.rs:13` | ✅ | Two fields: `changed`, `content` |
| `format_skill()` reads `original` at line 49 | ✅ | Calls `read_file_checked`, variable named `original` |
| CLI handler `--check` branch at ~line 960 | ✅ | `eprintln!("Would reformat: ...")` at line 961 |
| `fmt_check_unformatted_exits_nonzero` test | ✅ | `tests/cli.rs:1662` |

No discrepancies.

---

## 2. Dependency Choice

`similar = "2"` is appropriate. It's the standard Rust diff library
(successor to `difference`), has no transitive dependencies beyond
core, and provides `TextDiff` with unified diff formatting out of the
box. No concerns.

---

## 3. Design Review

### 3.1 `original` field adds a full copy to `FormatResult`

Adding `original: String` to `FormatResult` means every call to
`format_skill()` — including the non-`--check` write path — carries
a full copy of the original content. In the write path (no `--check`),
the original is never used.

This is acceptable for a CLI tool processing files under 1 MiB (the
`read_file_checked` cap). But if this bothers you, an alternative is
to compute the diff inside `format_skill` or to return `original` only
via a separate function. The current plan is simpler and fine.

**Recommendation:** Proceed as planned. The extra allocation is negligible
for a CLI tool.

### 3.2 Diff output destination: stderr vs stdout

The plan says "print the output to stderr" (§4). This is correct for
`--check` mode, where stdout should remain clean for scripting. Verify
the test assertions target stderr (`cmd.assert().stderr(...)`) not
stdout.

### 3.3 No `--diff` flag — always show diff in `--check`

The plan explicitly states "No new `--diff` flag needed — the diff is
always shown when `--check` detects changes." This matches the issue
description and follows `rustfmt --check` behavior. Clean design.

### 3.4 `diff_skill` function signature

```rust
pub fn diff_skill(result: &FormatResult, path: &str) -> String
```

Using `&str` for `path` is fine since it's only used as a display label
in the diff headers. The function only needs `result.original` and
`result.content`, so the `FormatResult` reference is appropriate.

Consider returning an empty string when `!result.changed` to make the
function safe to call unconditionally. The plan doesn't specify this,
but it's a natural guard.

---

## 4. Test Coverage

### 4.1 Planned tests

| Test | What it verifies |
|------|------------------|
| Update `fmt_check_unformatted_exits_nonzero` | Diff markers (`---`, `+++`, `@@`) present in stderr |
| New `fmt_check_shows_diff_content` | Actual changed lines appear in diff output |

### 4.2 Suggested additional tests

| Test | Why |
|------|-----|
| `diff_skill` unit test with no changes | Returns empty string |
| `diff_skill` unit test with known input | Verify exact diff output format |
| `fmt_check_formatted_no_diff` | Already-formatted file produces no diff output |
| CRLF input produces clean diff | Interaction with the CRLF normalization from #94 |

The first two are unit tests in `formatter.rs`, fast and targeted. The
CRLF test verifies that the diff doesn't show spurious line-ending
changes.

---

## 5. Files Assessment

| File | Change | Risk |
|------|--------|:----:|
| `Cargo.toml` | Add `similar = "2"` | None |
| `src/formatter.rs` | Add field + function | Low — additive |
| `src/main.rs` | One call site change | Low — 2–3 lines |
| `tests/cli.rs` | Test updates | None |

No cross-cutting concerns. No API-breaking changes (`FormatResult` gains
a field, which is additive — existing pattern matches use named fields or
`.changed`/`.content` access, so a new field is backward-compatible).

Wait — actually, if any code destructures `FormatResult` with `..` or
positional matching, the new field is fine. But if there's explicit struct
construction elsewhere, it would break. Let me check: `FormatResult` is
only constructed inside `format_skill()` (line 54), so adding a field is
safe.

---

## 6. Scope

| Metric | Estimate |
|--------|----------|
| New dependencies | 1 (`similar`) |
| Modified files | 4 |
| New functions | 1 (`diff_skill`) |
| New tests | 2–4 |
| Net line delta | +30–50 |

Compact and well-contained.

---

## 7. Summary

| Dimension | Rating |
|-----------|:------:|
| Accuracy | ✅ |
| Completeness | ✅ |
| Design | ✅ |
| Risk | Low |

**Recommendation:** Proceed with execution. Consider the additional
test cases from §4.2, particularly the CRLF interaction test.
