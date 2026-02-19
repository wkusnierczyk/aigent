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

- [ ] Decide on path resolution: canonicalize or keep as-is (finding #2)
- [ ] Document XML format divergence from reference implementation (finding #1)
- [ ] Consider adding `'` → `&apos;` to `xml_escape` (finding #3)
