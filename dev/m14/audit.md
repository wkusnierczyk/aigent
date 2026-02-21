# M14: SRE Audit Report

Security, reliability, and performance review of the aigent codebase.

**Scope:** `src/` (27 source files across library, CLI, and builder modules)
**Date:** 2026-02-21
**Branch:** `dev/m13` (pre-M14)

---

## Summary

| Category    | High | Medium | Low | Info |
|-------------|------|--------|-----|------|
| Security    | 1    | 2      | 2   | 2    |
| Reliability | 1    | 5      | 1   | 1    |
| Performance | 0    | 3      | 2   | 0    |
| **Total**   | **2**| **10** | **5**| **3**|

---

## Security

### SEC-1: Symlink following in file operations [HIGH] — #87

All uses of `is_file()` and `is_dir()` transparently follow symbolic links.
An attacker who can plant a symlink inside a skill directory could cause the
tool to read or copy files from outside the intended directory tree.

| File | Function | Detail |
|------|----------|--------|
| `parser.rs` | `find_skill_md` | `uppercase.is_file()`, `lowercase.is_file()` |
| `assembler.rs` | `copy_skill_files` | `src_path.is_file()`, `src_path.is_dir()` |
| `assembler.rs` | `copy_dir_recursive` | `src_path.is_file()`, `src_path.is_dir()` |
| `validator.rs` | `discover_skills_recursive` | `path.is_file()`, `path.is_dir()` |
| `structure.rs` | `check_script_permissions_impl`, `check_nesting_recursive` | `path.is_file()`, `path.is_dir()` |

Zero calls to `symlink_metadata()` or `read_link()` exist in the codebase.

**Fix:** Use `symlink_metadata()` in security-sensitive paths. Skip symlinks
in directory walks, or resolve and validate they stay within the skill tree.

---

### SEC-2: Path traversal via markdown reference links [MEDIUM] — #89

In `structure.rs` `check_references`, the `clean_path` extracted from markdown
links is joined directly to the skill directory without sanitizing `..`
components:

```rust
let full_path = dir.join(clean_path);
```

While this function only checks existence (not content), it can probe the
filesystem outside the skill directory.

**Fix:** Reject reference paths containing `..`, or canonicalize and verify
the resolved path starts with the skill directory.

---

### SEC-3: Uncapped file reads via `read_to_string` [MEDIUM] — #90

Every file read uses `std::fs::read_to_string` with no size limit. A
maliciously large SKILL.md, tests.yml, or reference file could exhaust memory.

| File | Function |
|------|----------|
| `parser.rs` | `read_properties`, `read_body` |
| `validator.rs` | `validate_with_target` |
| `fixer.rs` | `apply_fixes` |
| `formatter.rs` | `format_skill` |
| `test_runner.rs` | `run_test_suite` |

**Fix:** Add a `MAX_SKILL_FILE_SIZE` constant (e.g. 1 MiB). Check file size
via `metadata().len()` before reading.

---

### SEC-4: Unvalidated `plugin_name` from `AssembleOptions` [LOW]

The `plugin_name` used in `generate_plugin_json` comes from `opts.name`
(user-provided CLI argument) or defaults to the first skill name. While skill
names pass `is_unsafe_name`, the explicit `opts.name` override does not receive
the same validation. Impact is limited because `serde_json` handles escaping.

**Location:** `assembler.rs` line 113.

---

### SEC-5: Environment-variable-controlled base URLs [LOW]

LLM provider base URLs come directly from environment variables without URL
validation. An attacker with write access to the environment could redirect
API calls to arbitrary hosts. Risk is mitigated because environment variables
are a trusted input source.

| File | Variable |
|------|----------|
| `builder/providers/openai.rs` | `OPENAI_API_BASE` / `OPENAI_BASE_URL` |
| `builder/providers/ollama.rs` | `OLLAMA_HOST` |

---

### SEC-6: No shell injection vectors [INFO — clean]

Zero calls to `Command::new` or `process::Command` exist in the codebase.
The tool does not spawn subprocesses.

---

### SEC-7: YAML deserialization is typed [INFO — clean]

All YAML parsing uses `serde_yaml_ng` with strongly-typed target structs
(`SkillProperties`, `TestFixture`). No use of untyped deserialization from
untrusted external sources.

---

## Reliability

### REL-1: `read_body` silently swallows all errors [HIGH] — #88

`read_body()` in `parser.rs` silently returns an empty string on any error —
file not found, IO error, or parse error. Callers cannot distinguish between
"no body" and "failed to read."

All three error branches (`None` from `find_skill_md`, `Err` from
`read_to_string`, `Err` from `parse_frontmatter`) return `String::new()`.

**Impact:** `check`, `lint`, and `scorer` silently produce incomplete results.

**Fix:** Change signature to `Result<String>` and propagate errors, or return
`Option<String>` to distinguish "no body" from "failed to read."

---

### REL-2: `discover_skills_recursive` silently drops errors [MEDIUM] — #91

`read_dir` errors are silently ignored, and `.flatten()` silently drops
individual entry errors. Permission errors on subdirectories cause silent
skipping.

**Location:** `validator.rs`, `discover_skills_recursive`.

**Fix:** Collect IO errors as warnings/diagnostics or log to stderr.

---

### REL-3: `collect_skills` silently skips errors [MEDIUM] — #92

Three error paths in `prompt.rs` `collect_skills` are silently swallowed with
`continue`:

1. `canonicalize` failure
2. `read_properties` failure
3. `find_skill_md` returning `None`

**Fix:** Log skipped directories or return `(Vec<SkillEntry>, Vec<Warning>)`.

---

### REL-4: TOCTOU race in `build_skill` and `init_skill` [MEDIUM] — #93

Both functions check for existing SKILL.md then create the file. Between the
check and write, another process could create the file and it would be
silently overwritten.

**Location:** `builder/mod.rs`, `build_skill` and `init_skill`.

**Fix:** Use `OpenOptions::new().create_new(true)` for atomic exclusive
creation.

---

### REL-5: `format_content` byte slicing assumes LF line endings [MEDIUM] — #94

`format_content` in `formatter.rs` slices at byte offsets assuming single-byte
newlines (`\n`). CRLF line endings (`\r\n`) would cause incorrect offsets and
potential panics:

- `&after_first[1..close_pos]` — assumes single-byte after `---`
- `close_pos + 5` — hardcoded for `\n---\n`

**Fix:** Normalize CRLF to LF before parsing, or use line-based parsing.

---

### REL-6: Potential panic in `extract_frontmatter_lines` [MEDIUM]

In `main.rs`, `extract_frontmatter_lines` uses `.take_while(|l| l.trim_end() != "---")`
which relies on the delimiter matching after `trim_end()`. A closing delimiter
with different whitespace (tabs) could be missed, causing the function to
consume the entire file.

Not filed as a separate issue — covered by REL-5's CRLF/whitespace handling.

---

### REL-7: `unwrap()` in `main.rs` after `format_skill` [LOW]

`find_skill_md(dir).unwrap()` is called after `format_skill(dir)` succeeded.
The file could theoretically be deleted between the two calls (TOCTOU).
Per project conventions, `unwrap()` is allowed in `main.rs`.

**Location:** `main.rs` line 939.

---

### REL-8: `expect()` in static regex initialization [INFO — acceptable]

All `expect()` calls are in `LazyLock` regex compilations with hardcoded
patterns. These are compile-time-deterministic and will never fail at runtime.

---

## Performance

### PERF-1: O(n^2) conflict detection [MEDIUM] — #95

`check_description_similarity` in `conflict.rs` compares every pair of skills
using Jaccard similarity — O(n*(n-1)/2) comparisons. Each call allocates
fresh `Vec<String>` and `HashSet` instances.

Fine for tens of skills. Noticeable at hundreds.

**Fix:** Pre-tokenize descriptions into `Vec<HashSet<String>>` before the
nested loop.

---

### PERF-2: Unbounded recursion in directory walking [MEDIUM] — #96

`copy_dir_recursive` (assembler.rs) and `discover_skills_recursive`
(validator.rs) recurse with no depth limit or cycle detection. Deeply nested
or cyclic symlink structures could cause stack overflow.

Note: `check_nesting_recursive` in `structure.rs` already has
`MAX_NESTING_DEPTH` — the same pattern should be applied elsewhere.

**Fix:** Add a `max_depth` parameter and return error when exceeded.

---

### PERF-3: Repeated allocations in `jaccard_similarity` [LOW]

Each call creates fresh `Vec<String>` and `HashSet` allocations. Compounds
with PERF-1's O(n^2) loop. Low impact at current expected scale.

**Location:** `conflict.rs`, `jaccard_similarity`.

---

### PERF-4: Entire files read into memory [LOW]

Every SKILL.md and tests.yml file is read entirely into memory via
`read_to_string`. Intersects with SEC-3. Negligible for typical skill files
(kilobytes).

---

## Clean areas

- **No shell injection** — zero `Command::new` calls
- **YAML deserialization** is typed via `serde`
- **JSON generation** uses `serde_json` (no string formatting)
- All `expect()` calls are on compile-time-deterministic lazy regexes
- Path traversal in assembler is already guarded by `is_unsafe_name`

---

## Issue tracker

| ID | Issue | Severity | Category |
|----|-------|----------|----------|
| SEC-1 | [#87](https://github.com/wkusnierczyk/aigent/issues/87) | HIGH | Security |
| REL-1 | [#88](https://github.com/wkusnierczyk/aigent/issues/88) | HIGH | Reliability |
| SEC-2 | [#89](https://github.com/wkusnierczyk/aigent/issues/89) | MEDIUM | Security |
| SEC-3 | [#90](https://github.com/wkusnierczyk/aigent/issues/90) | MEDIUM | Security |
| REL-2 | [#91](https://github.com/wkusnierczyk/aigent/issues/91) | MEDIUM | Reliability |
| REL-3 | [#92](https://github.com/wkusnierczyk/aigent/issues/92) | MEDIUM | Reliability |
| REL-4 | [#93](https://github.com/wkusnierczyk/aigent/issues/93) | MEDIUM | Reliability |
| REL-5 | [#94](https://github.com/wkusnierczyk/aigent/issues/94) | MEDIUM | Reliability |
| PERF-1 | [#95](https://github.com/wkusnierczyk/aigent/issues/95) | MEDIUM | Performance |
| PERF-2 | [#96](https://github.com/wkusnierczyk/aigent/issues/96) | MEDIUM | Performance |
