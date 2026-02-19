# Review of `dev/m05/plan.md`

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Scope:** Pre-implementation plan review for M5: Prompt Generation
**References:** Issues #16, #17; current `src/prompt.rs` stub on `main`;
`src/parser.rs` (M3), `src/main.rs` (M1/M6);
[skills-ref `prompt.py`](https://github.com/agentskills/agentskills/blob/main/skills-ref/src/skills_ref/prompt.py);
[Anthropic best-practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)

---

## Overall Assessment

Compact, well-scoped plan for a relatively straightforward module. The design
decisions section correctly identifies and resolves the signature tension between
issue #16 (`Result<String>`) and the existing stub (`String`). The plan's
choice to keep `to_prompt` infallible with silent skip is sound and arguably
more resilient than the reference implementation's exception-propagating
approach. Two medium findings, one low finding, and observations below.

---

## Findings

### 1. Medium: XML output format diverges from reference implementation

**References:** `plan.md:56-74`; skills-ref `prompt.py`

The plan specifies indented inline XML:

```xml
<available_skills>
  <skill>
    <name>skill-name</name>
    <description>what it does</description>
    <location>/path/to/SKILL.md</location>
  </skill>
</available_skills>
```

The reference implementation (`skills-ref/prompt.py`) produces flat
newline-delimited XML with text content on separate lines:

```xml
<available_skills>
<skill>
<name>
skill-name
</name>
<description>
what it does
</description>
<location>
/path/to/SKILL.md
</location>
</skill>
</available_skills>
```

Key differences:
- **Indentation**: Plan uses 2/4-space indentation. Ref uses none.
- **Text placement**: Plan puts text inline (`<name>X</name>`). Ref puts text
  on its own line between open/close tags.
- **Trailing newline**: Plan has `</available_skills>` as the last line
  (no trailing newline implied). Ref uses `"\n".join(lines)` which produces
  no trailing newline either.

Both are valid XML and functionally equivalent for LLM system-prompt injection.
However, there are practical considerations:

**(a)** If anyone ever compares aigent output against the reference
implementation (e.g., cross-validation tests), the string differences will
cause false failures.

**(b)** The reference format puts content on separate lines, which means the
`<description>` value can span multiple lines naturally. The inline format
makes multi-line descriptions awkward (though descriptions shouldn't contain
newlines per the spec).

**(c)** The inline format is more compact (fewer tokens in the system prompt),
which is actually better aligned with the spec's emphasis on token economy.

**Recommendation:** Document that the XML format intentionally differs from
the reference implementation, and why (token efficiency). No change needed
unless exact compatibility with `skills-ref` output is a project goal.

### 2. Medium: Reference implementation resolves paths; plan does not

**References:** `plan.md:84-89`; skills-ref `prompt.py`

The reference implementation resolves each directory to an absolute path
before processing:

```python
skill_dir = Path(skill_dir).resolve()
```

This means `<location>` always contains an absolute path regardless of whether
the caller passed a relative path. The plan does not mention resolving paths —
`find_skill_md(dir)` returns whatever `dir.join("SKILL.md")` produces, which
preserves the caller's relative/absolute choice.

Impact:
- If the caller passes `"./my-skill"`, aigent produces
  `<location>./my-skill/SKILL.md</location>` while the ref produces
  `<location>/home/user/project/my-skill/SKILL.md</location>`.
- The `<location>` element is used by the LLM to find the skill file. Relative
  paths may not resolve correctly if the LLM's working directory differs from
  the caller's.

**Options:**

**(a) Resolve to absolute paths** via `dir.canonicalize()` or
`std::fs::canonicalize(dir)` before calling `find_skill_md`. This matches the
reference implementation and ensures `<location>` is always unambiguous.

**(b) Keep relative paths** and let the caller decide. This is simpler but
places the burden on the caller (main.rs) to pass absolute paths.

**Recommendation:** Option (a) — canonicalize. This matches the reference
implementation and is more robust. If `canonicalize` fails (e.g., path doesn't
exist), the directory will be skipped anyway by the `read_properties` error
path, so there's no additional failure mode.

### 3. Low: `xml_escape` does not escape `'` (single quote)

**References:** `plan.md:92-96`

The plan acknowledges that `xml_escape` does not escape `'` → `&apos;` and
justifies it by noting there are no XML attributes in the output. This is
correct for the *current* schema.

However, the `xml_escape` function is `pub` and may be used by other consumers
(e.g., future modules, or external users of the library). The standard XML
predefined entities are all five: `& < > " '`. Omitting `'` is a landmine
if someone later uses `xml_escape` for attribute values in single quotes.

The reference implementation uses Python's `html.escape()`, which by default
escapes `& < >` and optionally `"` (via `quote=True`) but **not** `'`. So
aigent's behavior is actually *more complete* than the reference (it escapes
`"` while the ref's default doesn't).

**Recommendation:** Consider adding `'` → `&apos;` for completeness. The cost
is one more `.replace()` call. Not blocking — the current behavior is correct
for all known use cases.

---

## Observations (not issues)

### Infallible signature is better than the ref

The plan's decision to keep `-> String` with silent skip is a deliberate
improvement over the reference implementation. In `skills-ref/prompt.py`,
`read_properties(skill_dir)` can raise exceptions that propagate uncaught
through `to_prompt`, crashing the entire call if *one* directory is bad.
aigent's approach — catch errors per-directory and skip — is more resilient
and better suited to a best-effort prompt-assembly function.

### Test coverage is appropriate

14 tests for a small module is proportional. The test plan covers:
- All 4 xml_escape character substitutions + no-op + combined + ordering
- Empty/single/multiple skill cases
- Error handling (skip invalid, mix valid+invalid)
- Path verification

The double-escape ordering test (#7) is particularly important — it verifies
that `&` is escaped first so that `&lt;` in the input becomes `&amp;lt;`
(correct) rather than `&lt;` being silently preserved.

### No `lib.rs` changes needed

The plan doesn't mention `lib.rs` changes because `to_prompt` is already
re-exported: `pub use prompt::to_prompt;` (line 12). Similarly, `main.rs`
already has the `ToPrompt` command wired up. This is a clean, isolated module
change.

### `xml_escape` already exists and is tested (sort of)

The plan lists 7 `xml_escape` tests, but `xml_escape` already exists in the
stub (it was implemented in M1). The plan treats these as new tests, which
makes sense — there are currently no tests for `xml_escape` in the codebase.
The function was implemented but never tested. M5 closes this gap.

### Compliance with reference implementation

Beyond the format and path-resolution differences noted above, the plan is
compliant with the `skills-ref` reference:

| Feature | Ref | Plan | Match? |
|---------|-----|------|--------|
| Outer wrapper | `<available_skills>` | `<available_skills>` | ✅ |
| Skill element | `<skill>` | `<skill>` | ✅ |
| Child elements | name, description, location | name, description, location | ✅ |
| XML escaping | `html.escape()` | `xml_escape` | ✅ |
| Location source | `find_skill_md()` | `find_skill_md()` | ✅ |
| Empty input | Empty wrapper | Empty wrapper | ✅ |
| Error handling | Exception propagates | Silent skip | ⚠️ Deliberate divergence |
| Path resolution | `.resolve()` | Not resolved | ⚠️ Finding #2 |
| Output format | Flat newlines | Indented inline | ⚠️ Finding #1 |

### Compliance with Anthropic spec

The [best-practices page](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
and [overview page](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview)
describe the XML format only in examples:

```xml
<available_skills>
  <skill>
    <name>skill-name</name>
    <description>what it does</description>
    <location>/path/to/SKILL.md</location>
  </skill>
</available_skills>
```

This matches the plan's indented inline format exactly. The reference
implementation's flat-newline format is actually the one that diverges from
the spec examples. So aigent's format is *more spec-compliant* than the ref.

---

## Checklist for Plan Finalization

- [x] Decide on path resolution: canonicalize or keep as-is (finding #2)
- [x] Document XML format divergence from reference implementation (finding #1)
- [x] Consider adding `'` → `&apos;` to `xml_escape` (finding #3)

---
---

# Code Review of `dev/m05`

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Scope:** Implementation review for M5: Prompt Generation
**Commit:** `f4eda5d M5: Implement XML prompt generation with skill directory support`
**Files changed:** `src/prompt.rs`, `dev/m05/plan.md`, `dev/m05/review.md`

---

## Verification

| Check | Result |
|-------|--------|
| `cargo fmt --check` | ✅ Clean |
| `cargo clippy -- -D warnings` | ✅ Clean |
| `cargo test` | ✅ 102 passed, 0 failed |
| Test count: prompt | 15 (matches plan: 15) |
| Test count: total | 102 (was 87 in M4; +15 new) |

No changes to `src/lib.rs` or `src/main.rs` — both were already wired up.

---

## Plan Conformance

### Review Finding Resolutions — All 3 Resolved

**Finding #1 (Medium): XML format diverges from reference implementation.**
✅ Resolved. Plan updated to document that the indented inline format follows
the Anthropic spec examples, not the reference implementation. Doc comment on
`to_prompt` (line 25-27) explicitly references the spec and describes the
format. The divergence will be documented in M8: Docs.

**Finding #2 (Medium): Path resolution.**
✅ Resolved. `std::fs::canonicalize(dir)` is called before `read_properties`
(line 46). If `canonicalize` fails, the directory is skipped. Test
`to_prompt_location_is_absolute` (line 230) extracts the `<location>` value
and asserts `Path::new(location).is_absolute()`.

**Finding #3 (Low): `xml_escape` missing `'` → `&apos;`.**
✅ Resolved. Line 16: `.replace('\'', "&apos;")` added. Test
`xml_escape_single_quote` (line 127) verifies `"it's"` → `"it&apos;s"`.
Doc comment updated: "Escape all five XML predefined entities."

### Plan vs Implementation Mapping

| Plan Item | Status | Notes |
|-----------|--------|-------|
| `xml_escape`: add `'` → `&apos;` | ✅ | Line 16 |
| `to_prompt`: init `<available_skills>\n` | ✅ | Line 42 |
| `to_prompt`: canonicalize each dir | ✅ | Lines 46-49 |
| `to_prompt`: `read_properties` skip on error | ✅ | Lines 52-55 |
| `to_prompt`: `find_skill_md` defensive | ✅ | Lines 58-61 |
| `to_prompt`: 2/4-space indented XML | ✅ | Lines 67-81 |
| `to_prompt`: close `</available_skills>` | ✅ | Line 84 |
| `xml_escape` tests: 8 | ✅ | 8 tests |
| `to_prompt` tests: 7 | ✅ | 7 tests |
| Total: 15 tests | ✅ | 15 confirmed |
| `#[must_use]` on both functions | ✅ | Lines 10, 40 |
| `use std::fmt::Write` | ✅ | Line 1 |
| Signature unchanged: `&[&Path] -> String` | ✅ | Line 41 |

---

## Findings

### 1. Low: `to_string_lossy()` silently replaces invalid UTF-8 in paths

**Reference:** `prompt.rs:63`

The location path is converted to a string via `location.to_string_lossy()`,
which replaces invalid UTF-8 bytes with the Unicode replacement character
`U+FFFD` (�). On most systems this is irrelevant — filesystem paths are
typically valid UTF-8 (macOS enforces UTF-8; Linux allows arbitrary bytes
but they're rare in practice).

However, if a path *did* contain non-UTF-8 bytes, the `<location>` element
would contain `�` characters, making it unresolvable by the LLM. The
alternative is `to_str()` returning `Option<&str>` and skipping paths that
aren't valid UTF-8. This would be more correct but adds complexity for an
extremely rare edge case.

**Recommendation:** No change needed. `to_string_lossy()` is the standard
Rust idiom for path-to-string conversion, and the project already uses it
elsewhere. Documenting the behavior in a comment would be sufficient if
desired.

### 2. Low: `writeln!` + `unwrap()` vs project convention

**Reference:** `prompt.rs:66-81`

The implementation uses `writeln!(out, ...).unwrap()` seven times. The
doc comment on line 66 correctly explains why this is safe: `write!` on
`String` is infallible. This is a well-known Rust pattern — `String`'s
`fmt::Write` implementation cannot fail.

The project convention (CLAUDE.md) says "No `unwrap()` in library code."
Strictly, `prompt.rs` is library code (it's in `src/`, not `main.rs`). The
`unwrap()` calls are provably safe, but they technically violate the letter
of the convention.

**Options:**

**(a) Keep `unwrap()` with the justification comment.** The convention
exists to prevent panics on fallible operations; these are infallible.

**(b) Use `let _ = writeln!(...);`** to silently discard the always-Ok
result. This avoids `unwrap()` but hides intent.

**(c) Use `write!` + `push_str` alternatives** to avoid `fmt::Write`
entirely:
```rust
out.push_str("  <skill>\n");
out.push_str(&format!("    <name>{}</name>\n", xml_escape(&props.name)));
```
This replaces `writeln!` + `unwrap()` with `push_str` + `format!`, avoiding
both the `Write` trait and `unwrap()`. Slightly more verbose but no
convention tension.

**Recommendation:** Option (a) is fine — the comment documents the safety
invariant, and the intent is clear. This is a style question, not a
correctness issue.

---

## Observations (not issues)

### Clean three-guard pipeline

The `to_prompt` loop body has three sequential guards:
1. `canonicalize(dir)` — skip if path doesn't exist
2. `read_properties(&canonical)` — skip if SKILL.md missing/unparsable
3. `find_skill_md(&canonical)` — defensive, should always succeed after #2

Each uses `match ... { Ok(x) => x, Err(_) => continue }`, which is the
idiomatic Rust pattern for "skip on error in a loop." The three guards are
ordered from cheapest to most expensive (filesystem stat → file read + YAML
parse → filesystem lookup), which is good practice even though the difference
is negligible for small directory counts.

### Test helper reuses M4 pattern

The `make_skill_dir` helper (line 96) follows the same pattern as the M4
validator tests: create a parent `TempDir`, create a named subdirectory,
write SKILL.md, return `(TempDir, PathBuf)`. The parent `TempDir` is kept
alive for RAII lifetime management. This consistency across milestones makes
the test infrastructure predictable.

### `to_prompt_location_is_absolute` test is well-designed

This test (line 230) doesn't just check `contains("/")` — it extracts the
actual `<location>` value by finding `<location>` and `</location>` markers,
then asserts `Path::new(location).is_absolute()`. This is the right way to
test canonicalization: it verifies the semantic property (absolute path)
rather than a fragile string pattern.

### No trailing newline after `</available_skills>`

The output ends with `</available_skills>` (no trailing `\n`). This means
`main.rs`'s `println!("{}", aigent::to_prompt(&dirs))` adds exactly one
newline at the end. If `push_str` were replaced with `writeln!`, there'd
be a double-newline. The current approach is correct.

### `xml_escape` test #7 is comprehensive

The `xml_escape_multiple_special_characters` test (line 137) uses a string
that exercises all five entity escapes in a single input:
`<tag attr="v">&'x'</tag>`. This is a good integration-style test that
catches ordering bugs (e.g., if `<` were escaped before `&`, the `&` in
`&lt;` would be double-escaped).

---

## Verdict

**Ready to merge.** All three plan review findings are resolved. Implementation
matches the plan exactly — 15 prompt tests, `xml_escape` updated with
`&apos;`, paths canonicalized, XML format matches Anthropic spec. Two low
findings are style/edge-case observations with no functional impact. The
verification suite is fully green (102 tests, clean fmt and clippy).
