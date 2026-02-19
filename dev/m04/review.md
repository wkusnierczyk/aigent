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

- [ ] Resolve warning/error integration with `main.rs` exit code (finding #1)
- [ ] Fix CJK character acceptance rule to use `is_alphabetic() && !is_uppercase()` instead of `is_lowercase()` (finding #3)
- [ ] Document that `validate_metadata` expects raw `parse_frontmatter` output (finding #2)
- [ ] Consider sharing `KNOWN_KEYS` between parser and validator (finding #2)
- [ ] Decide on substring vs segment matching for reserved words (finding #4)
- [ ] Add test for reserved word as substring of longer segment (finding #4)
