# Review of `dev/m04/plan.md`

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Scope:** Pre-implementation plan review for M4: Validator
**References:** Issues #14, #15; current `src/validator.rs` stub on `main`;
`src/parser.rs` (M3), `src/models.rs` (M2), `src/main.rs` (M1/M6)

---

## Overall Assessment

Thorough plan with clear validation rules, i18n considerations, and a
comprehensive 35-test suite. The design decisions section is unusually well
documented — NFKC normalization timing, errors-vs-warnings convention, and the
`validate_metadata` vs `validate` split are all explained. Two medium issues, two
low issues, and observations below.

---

## Findings

### 1. Medium: `warning:` prefix convention creates an integration conflict with `main.rs`

**References:** `plan.md:40-47,150-152`; `src/main.rs:70-78`

The plan introduces a convention where warnings are prefixed with `"warning: "`
and callers filter with `msg.starts_with("warning: ")`. However, the existing
CLI in `main.rs` treats `validate()` output as:

```rust
let errors = aigent::validate(&dir);
if errors.is_empty() {
    std::process::exit(0);
} else {
    for e in &errors {
        eprintln!("{e}");
    }
    std::process::exit(1);
}
```

This means a skill that is *valid* but has warnings (e.g., body > 500 lines)
would cause `main.rs` to exit with code 1 — a validation failure. The user would
see `"warning: body exceeds 500 lines (550 lines)"` and the tool would report
failure.

**Options:**

**(a) Fix `main.rs` in M4** to filter warnings from errors before deciding
the exit code. This keeps M4 self-contained but expands scope into M6 territory.

**(b) Defer to M6** (CLI milestone) and accept that warnings temporarily cause
exit code 1. Document this as a known issue.

**(c) Don't use a string prefix convention.** Return a structured type instead:

```rust
pub struct ValidationMessage {
    pub severity: Severity, // Error | Warning
    pub message: String,
}
pub enum Severity { Error, Warning }
```

This eliminates the fragile string-prefix convention entirely but changes the
public API signature from `Vec<String>`.

**Recommendation:** Option (a) or (c). The string-prefix convention is fragile —
a typo (`"Warning: "` vs `"warning: "`) or a message that accidentally starts
with `"warning: "` would silently mis-classify. If the `Vec<String>` return type
is load-bearing (changing it would affect M6 or other consumers), then (a) is the
minimum fix. If the API can change, (c) is the cleanest long-term design.

### 2. Medium: `validate_metadata` operates on raw `HashMap` but plan step 4 checks for "unexpected metadata fields"

**References:** `plan.md:139-141`

The plan says `validate_metadata` should warn about unexpected keys not in the
known set (`name`, `description`, `license`, `compatibility`, `allowed-tools`).
However, `validate_metadata` receives the *full* raw metadata HashMap — which
includes all keys before extraction. Meanwhile, `read_properties` (M3) *already*
removes known keys and puts the remainder into `SkillProperties.metadata`.

This creates two possible flows:

**(a) `validate_metadata` is called with the raw HashMap from `parse_frontmatter`:**
Then the known keys are present, and the "unexpected field" check works as
described. But the caller (e.g., `validate`) must call `parse_frontmatter` instead
of `read_properties`.

**(b) `validate_metadata` is called with the HashMap from `SkillProperties.metadata`:**
Then the known keys are already removed, and the check would warn about *every*
remaining key — which is wrong, since `metadata` is explicitly the catch-all for
unknown keys.

The plan's `validate` function (line 148-149) uses `parse_frontmatter` directly,
so flow (a) is intended. But this means `validate_metadata` cannot be used
standalone on a `SkillProperties` struct — its "unexpected fields" check only
makes sense on raw metadata. This should be documented.

Also: if someone later adds a new known key (e.g., `version`) to the parser's
`KNOWN_KEYS` but forgets to update the validator's known-key whitelist, the
validator would falsely warn about the new key. Consider importing or sharing the
`KNOWN_KEYS` constant from `parser.rs` rather than duplicating the list.

**Recommendation:** Either (1) import `parser::KNOWN_KEYS` (requires making it
`pub`) or (2) define the canonical list in one place and re-export. Also add a
doc comment to `validate_metadata` clarifying it expects raw `parse_frontmatter`
output, not post-extraction metadata.

### 3. Low: Chinese characters in name — `char::is_lowercase()` returns `false` for CJK

**References:** `plan.md:73,208`

The plan says names may contain "Unicode lowercase letters
(`char::is_lowercase()`)" and test #25 expects Chinese characters to be accepted.
However, CJK ideographs are classified as `Lo` (Letter, other) in Unicode — they
are not cased at all. `char::is_lowercase()` returns `false` for `'中'`, `'文'`,
etc.

```rust
'中'.is_lowercase() // false
'中'.is_uppercase() // false
'a'.is_lowercase()  // true
'щ'.is_lowercase()  // true (Cyrillic lowercase)
```

So the rule "must be `[a-z0-9-]` or `char::is_lowercase()`" (plan line 73) would
reject Chinese characters — contradicting test #25.

**Fix:** The character rule needs to also accept `char::is_alphabetic()` characters
that are *not* uppercase. This covers CJK (not cased → not uppercase → accepted)
and Cyrillic lowercase (cased, lowercase → accepted) while still rejecting
uppercase Cyrillic (cased, uppercase → rejected):

```rust
c.is_ascii_lowercase()
    || c.is_ascii_digit()
    || c == '-'
    || (c.is_alphabetic() && !c.is_uppercase())
```

This is a **specification bug** in the plan, not just an implementation concern —
the rule as stated cannot pass test #25. The plan should be corrected before
implementation.

### 4. Low: Reserved-word check uses substring matching, which may over-reject

**References:** `plan.md:76,109`

The plan says reserved words (`anthropic`, `claude`) must not appear "as substrings
in the normalized name." This means:

- `claude-tools` → rejected ✓ (reasonable)
- `anthropic-sdk` → rejected ✓ (reasonable)
- `declaude-r` → rejected (probably unintended? the word `claude` appears as a
  substring within a longer word)
- `claudette` → rejected (probably unintended?)

A stricter approach would check reserved words as *segments* between hyphens:

```rust
name.split('-').any(|seg| RESERVED.contains(&seg))
```

This accepts `claudette` (one segment, not equal to `claude`) but rejects
`claude-tools` (segment `claude` matches). The substring approach is more
conservative but risks false positives for creative names.

**Recommendation:** Document which approach is intended. If substring matching is
deliberate (maximum caution), note it in the plan. If segment matching is preferred,
update the plan. Either way, add a test for a name that contains a reserved word as
a *substring of a longer segment* (e.g., `claudette`) to pin down the behavior.

---

## Observations (not issues)

### `LazyLock` for regex

The plan uses `std::sync::LazyLock` (plan line 129) for the XML tag regex. This is
the correct modern approach — `LazyLock` was stabilized in Rust 1.80 and replaces
the `lazy_static!` or `once_cell` patterns. It's thread-safe and zero-overhead
after first initialization. The project's MSRV isn't specified, but the Cargo.toml
uses `edition = "2021"` and modern dependency versions, so Rust 1.80+ is a safe
assumption.

### Error aggregation design

The "collect all errors in a single pass" approach (plan line 8-9) is good UX —
users see all problems at once rather than fixing them one by one. This is
consistent with how `cargo clippy` and `eslint` work. The `Vec<String>` return type
makes this natural.

### `validate` uses `parse_frontmatter` not `read_properties`

The plan's `validate` function (line 148) calls `parse_frontmatter` directly rather
than `read_properties`. This is correct — `read_properties` would fail on missing
`name`/`description` with a hard `AigentError::Validation`, while `validate` wants
to collect that as a soft error alongside other issues. The parser and validator
have different error philosophies: the parser fails fast (Result), the validator
collects all (Vec).

### Test coverage

35 tests is comprehensive. The boundary tests (64/65 chars for name, 1024/1025 for
description, 500/501 for compatibility, 500/501 lines for body) are good practice.
The i18n tests (Chinese, Russian, NFKC normalization) demonstrate serious
internationalization support. The test helper naming (`make_skill_dir`) is clear.

### Body-length warning

The 500-line threshold (plan line 150-151) for body warnings is a reasonable
heuristic for detecting overly verbose skill definitions. Making it a warning
rather than an error is the right call — long bodies might be valid for complex
skills.

---

## Checklist for Plan Finalization

- [x] Resolve warning/error integration with `main.rs` exit code (finding #1)
- [x] Fix CJK character acceptance rule to use `is_alphabetic() && !is_uppercase()` instead of `is_lowercase()` (finding #3)
- [x] Document that `validate_metadata` expects raw `parse_frontmatter` output (finding #2)
- [x] Consider sharing `KNOWN_KEYS` between parser and validator (finding #2)
- [x] Decide on substring vs segment matching for reserved words (finding #4)
- [x] Add test for reserved word as substring of longer segment (finding #4)

---
---

# Code Review of `dev/m04`

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Scope:** Implementation review for M4: Validator
**Commit:** `2b5c5cc M4: Implement skill directory and metadata validator`
**Files changed:** `src/validator.rs`, `src/parser.rs`, `src/lib.rs`, `src/main.rs`,
`dev/m04/plan.md`, `dev/m04/review.md`

---

## Verification

| Check | Result |
|-------|--------|
| `cargo fmt --check` | ✅ Clean |
| `cargo clippy -- -D warnings` | ✅ Clean |
| `cargo test` | ✅ 87 passed, 0 failed |
| Test count: validator | 36 (matches plan: 36) |
| Test count: parser | 23 (was 21 in M3; +2 new) |
| Test count: total | 87 (was 49 in M3; +38 new) |

---

## Plan Conformance

### Review Finding Resolutions — All 4 Resolved

**Finding #1 (Medium): `warning:` prefix + `main.rs` exit code.**
✅ Resolved via option (a). `main.rs` now filters warnings with
`messages.iter().any(|m| !m.starts_with("warning: "))` before deciding exit code.
Warnings are still printed to stderr but do not cause exit code 1.

**Finding #2 (Medium): `validate_metadata` + `KNOWN_KEYS` duplication.**
✅ Resolved. `parser::KNOWN_KEYS` made `pub` (line 124). Validator imports it
via `use crate::parser::{find_skill_md, parse_frontmatter, KNOWN_KEYS}`. Doc
comment on `validate_metadata` explicitly states it expects raw
`parse_frontmatter` output:

> "Expects raw `parse_frontmatter` output — the full `HashMap` before
> known-key extraction. **Not** suitable for use on
> `SkillProperties.metadata` (which has known keys already removed)."

**Finding #3 (Low): CJK `is_lowercase()` spec bug.**
✅ Resolved. Character rule correctly uses:
```rust
if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' { continue; }
if c.is_alphabetic() && !c.is_uppercase() { continue; }
```
Tests `chinese_characters_accepted` and `uppercase_cyrillic_rejected` confirm
the rule works for CJK (accepted), Cyrillic lowercase (accepted), and Cyrillic
uppercase (rejected).

**Finding #4 (Low): Reserved word substring vs segment matching.**
✅ Resolved. Uses segment matching: `normalized.split('-').any(|seg| seg == *word)`.
Tests `reserved_word_as_substring_accepted` (name `claudette` passes) and
`reserved_word_as_exact_segment_rejected` (name `my-claude-tool` fails) confirm
the behavior.

### Plan vs Implementation Mapping

| Plan Item | Status | Notes |
|-----------|--------|-------|
| `validate_name` — 9 checks | ✅ | All 9 implemented in order |
| `validate_description` — 3 checks | ✅ | Empty, length, XML |
| `validate_compatibility` — 1 check | ✅ | Length only |
| `contains_xml_tags` helper | ✅ | `LazyLock<Regex>` |
| `validate_metadata` — 4 steps | ✅ | name, description, compatibility, unexpected keys |
| `validate` — 6 steps | ✅ | find → read → parse → validate → body warning |
| `KNOWN_KEYS` made `pub` | ✅ | `src/parser.rs:124` |
| `KNOWN_KEYS` re-exported | ✅ | `src/lib.rs` |
| `main.rs` warning filter | ✅ | `src/main.rs:71` |
| 36 tests | ✅ | 36 validator tests confirmed |

### Parser Changes (M3 → M4)

Two changes to `src/parser.rs`:

1. **`KNOWN_KEYS` visibility**: `const KNOWN_KEYS` → `pub const KNOWN_KEYS`.
   Single-line change, correct.

2. **Re-export in `lib.rs`**: `pub use parser::{..., KNOWN_KEYS}` added.
   Correct.

No other parser changes — the M3 implementation is untouched.

---

## Findings

### 1. Low: `validate_name` returns early on empty but not on XML-containing names

**References:** `validator.rs:31-35,53-56`

When `name` is empty, `validate_name` returns early (line 34: `return errors`).
This is correct — subsequent checks (leading hyphen, trailing hyphen, etc.)
would produce misleading errors on an empty string.

However, when `name` contains XML tags (line 54), the check does not short-
circuit. A name like `<script>alert('x')</script>` would accumulate:
- "name contains invalid character: '<'" (char check)
- "name contains invalid character: '>'" (char check)
- "name contains invalid character: '('" (char check)
- "name contains invalid character: '''" (char check)
- "name contains invalid character: ')'" (char check)
- "name contains XML/HTML tags" (XML check)

The XML error is redundant with the character errors — the invalid characters
already flag the problem. This is not a bug (all errors are accurate), but it
is noisy. The XML check on `name` mainly adds value for names that are
*otherwise valid* but happen to contain a cleverly-crafted tag using only valid
characters — which is impossible since `<` and `>` are always invalid.

**Recommendation:** Consider removing the XML tag check from `validate_name`
entirely. The character validation already rejects `<` and `>`. The XML check
adds value only for `description` and `compatibility`, which accept a wider
character set. This is cosmetic — no functional impact.

### 2. Low: `validate` body line counting with `lines()` vs trailing newline

**Reference:** `validator.rs:202`

The body line count uses `body.lines().count()`. Rust's `str::lines()` does
*not* include a trailing empty line for a string ending in `\n`. For example:

```rust
"line1\nline2\n".lines().count() // → 2, not 3
"line1\nline2".lines().count()   // → 2
```

This is consistent — `parse_frontmatter` preserves the trailing newline from
the original content (line 110: `format!("{joined}\n")`), and `lines()` ignores
it. So a 500-line body with a trailing newline correctly counts as 500, not 501.

The tests (`validate_body_over_500_lines_warning` and
`validate_body_at_500_lines_no_warning`) construct bodies using
`(0..N).map(...).collect::<Vec<_>>().join("\n")` then wrap with
`format!("---\nname: my-skill\ndescription: desc\n---\n{body}\n")`. This
adds a trailing `\n` after the body, which `lines()` ignores. The tests pass,
confirming the count is correct.

Not an issue — documenting for completeness.

### 3. Low: Non-deterministic warning order for unexpected metadata keys

**Reference:** `validator.rs:165-169`

The unexpected-key warning loop iterates `metadata.keys()`, which returns keys
in the `HashMap`'s internal (non-deterministic) order. If a SKILL.md has
multiple unexpected keys, the warnings may appear in different orders across
runs.

This is not a functional problem — the warnings are independent and their order
doesn't affect correctness. The test `unexpected_metadata_field_warning` checks
for the presence of a warning via `any()`, so it's order-independent.

**Recommendation:** If stable output is desired (e.g., for snapshot testing or
deterministic CI logs), sort the keys before iterating:

```rust
let mut keys: Vec<_> = metadata.keys().collect();
keys.sort();
for key in keys { ... }
```

This is optional polish — no functional impact.

---

## Observations (not issues)

### Clean `validate` pipeline

The `validate` function is a model of defensive programming. Each step
(find → read → parse → validate → body) can fail independently, and each
failure mode returns an appropriate error list. Parse failures produce a single-
element `Vec` with the error message, while validation failures accumulate.
The pattern of converting `parse_frontmatter` errors via `e.to_string()` bridges
the `Result<T>` world (parser) to the `Vec<String>` world (validator) cleanly.

### Test helper design

The `make_metadata` and `make_skill_dir` helpers are well-designed:
- `make_metadata` builds `HashMap<String, Value>` from string pairs — concise
  for most tests.
- `make_skill_dir` creates a named subdirectory in a temp dir with SKILL.md —
  the parent `TempDir` is returned for lifetime management, a common pattern
  for Rust test fixtures.

Both avoid test boilerplate while keeping the setup explicit.

### Boundary tests with warning filtering

Several boundary tests (e.g., `name_exactly_64_chars`,
`compatibility_exactly_500_chars`) filter out warnings before asserting
emptiness:

```rust
let real_errors: Vec<_> = errors
    .iter()
    .filter(|e| !e.starts_with("warning: "))
    .collect();
assert!(real_errors.is_empty(), ...);
```

This is necessary because boundary-value metadata may trigger the unexpected-
key warning (if the test doesn't include all known keys). The filtering pattern
is consistent across all boundary tests. Good practice.

### `#[must_use]` annotations

Both `validate_metadata` and `validate` have `#[must_use]`, following the
project convention from CLAUDE.md. This prevents callers from accidentally
discarding the validation results — `rustc` will warn if the return value is
unused.

### Reserved words as `const` slice

`RESERVED_WORDS` is `&[&str]` rather than a `HashSet`. For a 2-element list
this is correct — linear scan is faster than hash lookup for tiny collections,
and the constant is entirely stack-allocated with no heap overhead. The
implementation uses `RESERVED_WORDS` in a loop that iterates both the reserved
list and the name segments, which is O(segments × reserved). For realistic
names (≤5 segments) and reserved lists (2 words), this is negligible.

---

## Verdict

**Ready to merge.** All four plan review findings are resolved. Implementation
matches the plan exactly — 36 validator tests, all pre-requisite changes
applied. The three findings are all low-severity cosmetic issues that do not
affect correctness or safety. The verification suite is fully green.
