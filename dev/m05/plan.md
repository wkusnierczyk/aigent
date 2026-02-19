# M5: Prompt Generation — Work Plan

## Overview

Implement XML prompt generation from skill directories: read each
directory's SKILL.md, extract properties, and produce an
`<available_skills>` XML block suitable for injecting into an LLM
system prompt. Includes proper XML escaping for injection prevention.

Issues: #16, #17.

## Branch Strategy

- **Dev branch**: `dev/m05` (created from `main`)
- **Task branches**: `task/m05-<name>` (created from `dev/m05`)
- After each wave, task branches merge into `dev/m05`
- After all waves, PR from `dev/m05` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- `read_properties(dir) -> Result<SkillProperties>` — from M3 (`src/parser.rs`)
- `find_skill_md(dir) -> Option<PathBuf>` — from M1/M3 (`src/parser.rs`)
- `SkillProperties` — from M2 (`src/models.rs`)
- `xml_escape` — already implemented in `src/prompt.rs`

## Current State

`xml_escape` is already implemented (escapes `& < > "`). `to_prompt` is
a stub (`todo!()`) with signature `to_prompt(_dirs: &[&Path]) -> String`.

---

## Review Finding Resolutions

### Finding 1 (Medium): XML output format diverges from reference implementation

**Resolution**: Adhere to the Anthropic spec. The plan's indented inline
format matches the
[Anthropic spec examples](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
exactly (`<name>skill-name</name>` on one line, 2/4-space indentation).
The reference implementation (`skills-ref/prompt.py`) uses a flat
newline-delimited format that diverges from the spec. aigent follows the
spec; this divergence from the reference implementation will be noted in
a compliance table in a future README (M8: Docs).

### Finding 2 (Medium): Reference implementation resolves paths; plan does not

**Resolution**: Canonicalize paths. Before processing each directory,
resolve it to an absolute path via `std::fs::canonicalize(dir)`. This
ensures `<location>` always contains an unambiguous absolute path,
matching the reference implementation. If `canonicalize` fails (e.g.,
path doesn't exist), skip the directory — it would fail on
`read_properties` anyway.

### Finding 3 (Low): `xml_escape` does not escape `'` (single quote)

**Resolution**: Add `'` → `&apos;` to `xml_escape` for completeness.
The function is `pub` and may be used by future consumers for attribute
values in single quotes. The cost is one additional `.replace()` call.
This makes aigent's escaping cover all five XML predefined entities.

---

## Design Decisions

### Function Signature

Issue #16 specifies `to_prompt(dirs: &[PathBuf]) -> Result<String>`, but the
current stub uses `to_prompt(dirs: &[&Path]) -> String`. The `&[&Path]`
parameter is more flexible (accepts both `&Path` and `&PathBuf` via coercion
at the call site), and is already wired into `main.rs`.

For error handling: `to_prompt` should **not** fail as a whole when one
directory is unreadable. Instead, it skips directories where
`read_properties` fails, emitting no `<skill>` element for them. This makes
the function infallible (`-> String`), which matches the existing stub
signature and avoids forcing callers to handle errors for a prompt-assembly
function. The caller (`main.rs` `ToPrompt` command) already handles the
empty-list case naturally.

**Decision**: Keep `to_prompt(dirs: &[&Path]) -> String`. No signature change.

### XML Output Format

Per issue #16, the output is:

```xml
<available_skills>
  <skill>
    <name>skill-name</name>
    <description>what it does</description>
    <location>/path/to/SKILL.md</location>
  </skill>
</available_skills>
```

- Outer wrapper: `<available_skills>...</available_skills>`
- Each skill: indented 2 spaces, with `<name>`, `<description>`, `<location>`
  children indented 4 spaces
- `<location>` is the resolved SKILL.md path (from `find_skill_md`)
- All text content is XML-escaped via `xml_escape`
- Empty input → `<available_skills>\n</available_skills>` (single wrapper,
  no child elements)

### Skipping Invalid Directories

If `read_properties(dir)` fails (missing SKILL.md, parse error, etc.),
that directory is silently skipped — no `<skill>` element is emitted.
This is deliberate: `to_prompt` is a best-effort assembly function.
Callers who need validation should use `validate()` first.

### Location Path

The `<location>` element contains the absolute path to the SKILL.md file.
Each input directory is canonicalized via `std::fs::canonicalize()` before
processing — this resolves symlinks and relative paths to absolute paths,
matching the reference implementation's `Path.resolve()` behavior. After
canonicalization, `find_skill_md(dir)` locates the SKILL.md. The path is
converted to a string with `to_string_lossy()` and XML-escaped.

### XML Escaping

The `xml_escape` function handles all five XML predefined entities:
`& < > " '`. Ampersand is escaped first to avoid double-escaping.
The `'` → `&apos;` escape is added in M5 for completeness — the
function is `pub` and may be used by future consumers for attribute
values.

---

## Wave 1 — Implementation

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m05-prompt` | #16 | Implement `to_prompt` in `src/prompt.rs` |

**Merge**: A → `dev/m05`. Checkpoint with user.

### Agent A — Prompt Generator (#16)

#### Pre-requisite change

Add `'` → `&apos;` to `xml_escape` (after `&quot;` replacement).

#### `to_prompt(dirs: &[&Path]) -> String`

1. Initialize output with `<available_skills>\n`
2. For each directory in `dirs`:
   a. Canonicalize `dir` via `std::fs::canonicalize` — if `Err`, skip
   b. Call `read_properties(&canonical)` — if `Err`, skip
   c. Call `find_skill_md(&canonical)` — if `None`, skip (defensive)
   d. Append:
      ```
        <skill>\n
          <name>{xml_escape(&props.name)}</name>\n
          <description>{xml_escape(&props.description)}</description>\n
          <location>{xml_escape(&location)}</location>\n
        </skill>\n
      ```
3. Append `</available_skills>`
4. Return the assembled string

Implementation uses `String::push_str` or `write!` macro for efficient
string building. No heap allocations beyond the output string.

The function is already annotated `#[must_use]` in the stub.

---

## Wave 2 — Tests (depends on Wave 1)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| B | `task/m05-tests` | #17 | Write prompt tests in `src/prompt.rs` `#[cfg(test)]` |

**Merge**: B → `dev/m05`. Checkpoint with user.

### Agent B — Tests (#17)

`src/prompt.rs` — `#[cfg(test)] mod tests`

Test infrastructure:
- Use `tempfile::tempdir()` for temporary skill directories
- Helper function `make_skill_dir(name, content)` — creates a subdirectory
  with SKILL.md, returns `(TempDir, PathBuf)` (same pattern as M4 tests)

#### `xml_escape` tests

| # | Test | Type |
|---|------|------|
| 1 | Escapes `&` → `&amp;` | unit |
| 2 | Escapes `<` → `&lt;` | unit |
| 3 | Escapes `>` → `&gt;` | unit |
| 4 | Escapes `"` → `&quot;` | unit |
| 5 | Escapes `'` → `&apos;` | unit |
| 6 | String with no special characters → unchanged | unit |
| 7 | String with multiple special characters escaped in order | unit |
| 8 | Ampersand escaped first — no double-escaping (input `&lt;` → `&amp;lt;`) | unit |

#### `to_prompt` tests

| # | Test | Type |
|---|------|------|
| 9 | Empty directory list → `<available_skills>\n</available_skills>` | edge case |
| 10 | Single valid skill → correct XML with name, description, location | happy path |
| 11 | Multiple valid skills → aggregated XML with all skills | happy path |
| 12 | Name/description with special characters → XML-escaped in output | escaping |
| 13 | Invalid directory (no SKILL.md) skipped silently | error handling |
| 14 | Mix of valid and invalid directories → only valid skills emitted | error handling |
| 15 | Location contains absolute SKILL.md path (canonicalized) | path verification |

---

## Wave 3 — Verify (depends on Wave 2)

Single agent runs the full check suite on `dev/m05`.

| Agent | Branch | Task |
|-------|--------|------|
| C | `dev/m05` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Deliverables

- `src/prompt.rs` — `to_prompt` implemented, `xml_escape` updated with `&apos;`
- 15 tests inline in `#[cfg(test)] mod tests`
- PR: `M5: Prompt Generation`
