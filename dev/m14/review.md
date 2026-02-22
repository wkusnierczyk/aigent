# M14: SRE Review — Plan Review

Review of `dev/m14/plan.md` against `dev/m14/audit.md` and current codebase
(main at `2c2309d`).

---

## Verdict

The plan is accurate, thorough, and ready for execution. All code
location claims verified against the codebase. Audit coverage is complete.
Wave dependencies are sound. Two items need attention before execution:
merge conflict mitigation (see §5) and a minor design clarification on
`read_body` (see §3.1).

---

## 1. Code Location Accuracy

All 11 categories of code references verified against main at `2c2309d`.

| Claim | Status | Notes |
|-------|:------:|-------|
| SEC-1: `is_file()`/`is_dir()` in 6 files | ✅ | All locations confirmed |
| SEC-2: `check_references` unsanitized join | ✅ | `dir.join(clean_path)` at line 108 |
| SEC-3: unchecked `read_to_string` in 5 files | ✅ | All locations confirmed |
| REL-1: `read_body` returns `String` | ✅ | Line 253, returns empty on all errors |
| REL-2: `.flatten()` drops errors | ✅ | Line 413, plus `Err(_) => return` at 407 |
| REL-3: 3 `continue` paths in `collect_skills` | ✅ | Lines 59, 64, 69 in `prompt.rs` |
| REL-4: check-then-write in `build_skill`/`init_skill` | ✅ | Lines 164–177 and 260–299 |
| REL-5: hardcoded `close_pos + 5` | ✅ | Line 90 in `formatter.rs` |
| PERF-1: per-call allocation in O(n²) loop | ✅ | `jaccard_similarity` lines 150–177 |
| PERF-2: no depth limit in 2 functions | ✅ | `check_nesting_recursive` already has `MAX_NESTING_DEPTH = 2` |
| #103: `parse_yaml_blocks()` exists | ✅ | Lines 204–254 in `formatter.rs` |
| #102: `bump-version.sh` exists | ✅ | — |

No discrepancies found.

---

## 2. Audit Coverage

### Addressed (14 issues)

All HIGH and MEDIUM findings tracked. Two HIGH (SEC-1, REL-1), eight
MEDIUM (SEC-2, SEC-3, REL-2–5, PERF-1–2), plus four non-audit items
(#102, #103, #105, #106).

### Deferred (safe to skip)

| Finding | Severity | Why safe |
|---------|----------|----------|
| SEC-4: unvalidated `plugin_name` | LOW | `serde_json` handles escaping; user-provided CLI arg, not untrusted file content |
| SEC-5: env-variable base URLs | LOW | Environment variables are a trusted input source |
| REL-7: `unwrap()` after `format_skill` | LOW | Allowed per project convention; negligible TOCTOU window in single-user CLI |
| PERF-3: repeated allocations in `jaccard_similarity` | LOW | Subsumed by PERF-1 (pre-tokenization eliminates this) |
| PERF-4: entire files read into memory | LOW | Subsumed by SEC-3 (1 MiB cap bounds memory) |

No gaps.

### Non-audit issues

| Issue | Scope | Fits M14? |
|-------|-------|:---------:|
| #102 (version script) | Replace `bump-version.sh` with unified `version.sh` | ✅ Release workflow cleanup |
| #103 (formatter comments) | Fix comment positioning during key reordering | ✅ Correctness fix, related to REL-5 |
| #105 (README review) | Manual pre-release verification | ✅ Release gate |
| #106 (CLI acceptance testing) | Manual end-to-end testing | ✅ Release gate |

All four are properly scoped and appropriately placed in their waves.

---

## 3. Design Decision Review

### 3.1 REL-1: `read_body` → `Result<String>` (#88)

The plan chooses `Result<String>` over `Option<String>`. This is the
right call — the current function conflates "no body" with "failed to
read", and callers need to distinguish IO errors from empty content.

**One question:** Should a skill with valid frontmatter but no body
after the closing `---` return `Ok(String::new())` or
`Ok("\n".to_string())`? The current implementation returns `""`. The
plan doesn't specify. Recommend keeping `Ok(String::new())` for
empty-body skills to maintain behavioral compatibility for callers that
check `body.is_empty()`.

### 3.2 SEC-1: Silent skip vs. diagnostic (#87)

The plan says symlinks are "silently skipped" during directory walks
but emit `S005` in structure validation. This is the right split:
callers doing discovery don't need to halt on symlinks, but users
running `--structure` checks get visibility.

### 3.3 REL-4: `create_new(true)` (#93)

Clean fix. The `AlreadyExists` → `AigentError::Build` mapping should
preserve the original message so the user sees which file already
exists. The plan implies this but doesn't spell it out.

### 3.4 REL-5: CRLF normalization (#94)

One-line fix at the top of `format_content`. Correct. The plan also
mentions normalizing in `extract_frontmatter_lines` in `main.rs` —
this is the function the audit flagged as REL-6. Good that it's
covered.

### 3.5 PERF-1: Pre-tokenization (#95)

The plan keeps `jaccard_similarity(a, b)` as a backward-compatible
wrapper. This is pragmatic — the function is `pub(crate)` so it has
no external API impact, but internal callers like `tester.rs` may
use it directly.

### 3.6 #103: Formatter comment anchoring

This is the most algorithmically subtle change. The plan correctly
identifies the flush-before-comment strategy. One edge case not
mentioned: YAML comments that appear *before the first key* (above
`name:`) should be preserved as header comments, not attached to
the first sorted key. The existing code already handles this (header
comments at lines 217–220), so no action needed — just noting for
the implementing agent.

---

## 4. Wave Dependencies

```
Wave 1: Security (A, B, C)     — independent of each other
    ↓ merge to dev/m14
Wave 2: Reliability (D, E, F, G) — D depends on C's read_file_checked
    ↓ merge to dev/m14
Wave 3: Perf + Cleanup (H, I, J, K) — independent of Wave 2
    ↓ merge to dev/m14
Wave 4: Verification (L-readme, L-cli, L-verify) — depends on all
```

**The dependency chain is correct.** The only inter-wave code dependency
is Wave 1 Agent C (#90) → Wave 2 Agent D (#88): the new `read_body`
calls `read_file_checked()` from #90.

Wave 3 is correctly marked independent — its issues touch different
code paths than Wave 2's error propagation changes.

---

## 5. Merge Conflict Risk

This is the highest-risk area of the plan. The worktree strategy
isolates agents within waves, but multiple agents touching the same
files creates merge friction.

### High risk

| File | Agents | Waves | Issue |
|------|--------|:-----:|-------|
| `validator.rs` | A, C, E, I | 1, 1, 2, 3 | 4 agents across 3 waves. A: symlink in `discover_skills_recursive`. C: `read_file_checked` in `validate_with_target`. E: verbose variant of `discover_skills_recursive`. I: depth limit in `discover_skills_recursive`. |
| `main.rs` | D, E, G | 2 | 3 agents in one wave. D: `read_body()` callers. E: verbose `collect_skills`. G: CRLF in `extract_frontmatter_lines`. Different functions but same file in parallel. |

### Medium risk

| File | Agents | Waves |
|------|--------|:-----:|
| `parser.rs` | A, C | 1 |
| `assembler.rs` | A, I | 1, 3 |
| `diagnostics.rs` | A, B | 1 |
| `structure.rs` | A, B | 1 |

### Mitigation recommendations

1. **Wave 1 merge order**: Merge Agent B (#89) first (smallest change,
   only `structure.rs` + `diagnostics.rs`). Then Agent A (#87) (widest
   change). Then Agent C (#90). This minimizes rebase surface.

2. **Wave 2 merge order**: Agent F (#93, only `builder/mod.rs`) →
   Agent G (#94, only `formatter.rs`) → Agent D (#88, `parser.rs` +
   callers) → Agent E (#91/#92, `validator.rs` + `main.rs`). Merge the
   narrowest-scope agents first.

3. **`discover_skills_recursive`**: Three agents (A, E, I) modify this
   function across three waves. Each wave inherits the previous wave's
   merged state, so conflicts are serial not parallel. But the function
   accumulates changes: symlink skip (Wave 1), error collection
   (Wave 2), depth limit (Wave 3). Consider having Agent I rewrite the
   function holistically rather than patching incrementally.

4. **`diagnostics.rs`**: Agents A and B both add new constants (`S005`,
   `S006`) in Wave 1. If diagnostics are defined sequentially, this
   creates a trivial merge conflict. Recommend one agent adds both
   constants, or pre-allocate code ranges.

---

## 6. Scope Assessment

| Metric | Plan estimate | Assessment |
|--------|---------------|------------|
| New files | 2 (`fs_util.rs`, `version.sh`) | Reasonable |
| Modified files | ~15 | Conservative — likely 17–18 counting `lib.rs`, `diagnostics.rs` |
| New tests | 35–45 | Reasonable for 14 issues |
| Net line delta | +400–600 | May be on the low side — verbose discovery variants (#91/#92) add type definitions, new functions, and CLI integration. Estimate +500–700. |
| Agents | 12 | Correct count |
| New dependencies | 0 | ✅ |

---

## 7. Items Not Covered

These are not problems — just noting things outside M14 scope:

- **SEC-4/SEC-5**: Intentionally deferred (LOW). No action needed.
- **Version bump to 0.3.0**: The plan references v0.3.0 in #105/#106
  descriptions, but the actual `cargo set-version 0.3.0` and tag push
  are not in the plan. Presumably a separate release step after M14 PR
  merges.
- **CHANGES.md**: The version script (#102) stubs a CHANGES.md entry
  but the actual changelog content for 0.3.0 isn't in scope. Also a
  release step.

---

## 8. Summary

| Dimension | Rating | Notes |
|-----------|:------:|-------|
| Accuracy | ✅ | All 11 code location claims verified |
| Completeness | ✅ | Full audit coverage, no gaps |
| Design | ✅ | One minor clarification on `read_body` empty-body semantics |
| Dependencies | ✅ | Sound wave ordering, single inter-wave code dependency |
| Conflict risk | ⚠️ | `validator.rs` (4 agents) and `main.rs` (3 agents) need merge order discipline |
| Scope | ✅ | Realistic estimates, no new dependencies |

**Recommendation:** Proceed with execution. Apply the merge order
recommendations from §5 to reduce conflict friction. Clarify the
`read_body` empty-body behavior (§3.1) before Agent D starts.

---
---

# M14: Code Review — Implementation Review

Review of the `dev/m14` branch (29 commits, `35bcd88`) against `main`
at `2c2309d`.

Baseline: 481 tests. Post-M14: 544 tests (410 unit + 106 CLI + 27 plugin
+ 1 doc-test). Net delta: +63 tests, +1268 lines (1433 added, 165 removed)
across 25 files.

---

## Verdict

The implementation is solid. All 14 issues addressed, all tests pass
(544/544), clippy clean, formatting clean. The code correctly follows
the plan and its reconciled amendments (A1–A6). Two bugs found (one in
`version.sh`, one missing depth limit). Several design observations worth
noting for future work.

| Dimension | Rating | Notes |
|-----------|:------:|-------|
| Correctness | ✅ | All tests pass, clippy/fmt clean |
| Plan adherence | ✅ | All 14 issues implemented per plan + amendments |
| Test coverage | ✅ | 63 new tests (plan estimated 35–45) |
| API design | ✅ | Backward-compatible verbose variants, clean error propagation |
| Security hardening | ✅ | Symlink safety, path traversal, file size cap all correct |
| Commit hygiene | ⚠️ | 11 duplicate commit messages from worktree merges |
| Shell portability | ⚠️ | `version.sh` CHANGES.md stub uses non-portable `\n` in sed |
| Depth limits | ⚠️ | `check_symlinks_recursive` missing depth limit |

---

## 1. Bugs

### 1.1 `version.sh`: Non-portable `\n` in sed replacement (LOW)

**File:** `scripts/version.sh:127–129`

```bash
STUB="## [$VERSION] — $TODAY\n\n_No changes yet._\n"
sedi "s/^# Changes$/# Changes\n\n$STUB/" "$CHANGES"
```

BSD `sed` (macOS) does not interpret `\n` as a newline in the replacement
string. This produces literal `\n` characters in the output on macOS.
The old `bump-version.sh` used a `head`/`tail`/temp-file approach that
was portable.

**Fix:** Use a temp-file approach (as the old script did), or use `printf`
with process substitution, or use `awk` for the insertion.

### 1.2 `check_symlinks_recursive`: No depth limit (LOW)

**File:** `src/structure.rs:277–311`

The function `check_symlinks_recursive` walks directories recursively
without a depth limit. While it only follows real directories (not
symlinks), a deeply nested directory structure could exhaust the stack.
Both `discover_skills_recursive` and `copy_dir_recursive` received depth
limits in this milestone (PERF-2, #96), but `check_symlinks_recursive`
was not included.

This is LOW severity because `check_nesting_recursive` (which shares the
same call site in `validate_structure`) already has `MAX_NESTING_DEPTH = 2`,
meaning overly deep structures would be flagged by S004 before anyone
would notice. However, the two functions are independent — `check_symlinks`
could be called separately.

**Fix:** Add the same `depth` parameter pattern used in the other recursive
functions, capping at `MAX_DISCOVERY_DEPTH` or similar.

---

## 2. Design Observations

### 2.1 Comment placement after key reordering (#103)

The new comment-handling logic in `format_frontmatter` collects all
interleaved comments (comments between keys) into a single list and emits
them between known keys and unknown keys. This means comments lose their
positional relationship to specific keys.

Example input:
```yaml
description: Does things
# About the name
name: my-skill
```

After reordering:
```yaml
name: my-skill
description: Does things
# About the name
```

The comment "About the name" now appears after `description`, not adjacent
to `name`. This is acceptable — the plan says comments should be "anchored
to position" not "attached to keys" — but it's a trade-off that may
surprise users. The idempotency test passes, which is the critical
property.

### 2.2 `read_file_checked` uses `metadata()` not `symlink_metadata()`

**File:** `src/parser.rs:18–29`

`read_file_checked` calls `std::fs::metadata(path)` to check the file
size. This follows symlinks, so a symlink pointing to a large file would
be checked against the target's size (correct behavior), but the function
itself doesn't reject symlinks. This is fine because callers reach
`read_file_checked` only through `find_skill_md` (which rejects symlinks
via `is_regular_file`) or through explicit path arguments in fixer/
formatter/test_runner where the user is providing the path directly.

No action needed — just documenting the design.

### 2.3 Duplicate TOCTOU error-mapping code in `builder/mod.rs`

**File:** `src/builder/mod.rs:172–190` and `src/builder/mod.rs:218–236`

The `build_skill` and `init_skill` functions both contain identical
`create_new(true)` + `map_err` blocks (~18 lines each). This could be
extracted into a helper like `write_skill_md_exclusive(path, content)`.
Not a bug — just notable duplication for future cleanup.

### 2.4 `discover_skills_recursive_verbose` silently stops at max depth

**File:** `src/validator.rs:470–471`

When `depth > MAX_DISCOVERY_DEPTH`, the verbose variant returns silently
without collecting a warning. Skills beyond depth 10 are invisible to
the user with no indication that the search stopped. Consider emitting
a `DiscoveryWarning` when the depth limit is hit.

### 2.5 `collect_skills_verbose` redundant `find_skill_md` call

**File:** `src/prompt.rs:102–108`

After `read_properties(&canonical)` succeeds (which internally calls
`find_skill_md`), the function calls `find_skill_md(&canonical)` again
to get the location path. This is a redundant filesystem call. Minor
perf impact; could be avoided by having `read_properties` return the
path, or by constructing the location from the canonical path directly.

### 2.6 Step numbering gap in `build_skill`

**File:** `src/builder/mod.rs:164–192`

After removing the check-then-write steps, comments jump from step 9 to
step 11 (step 10 was removed). Cosmetic only.

---

## 3. Test Quality

### 3.1 Coverage assessment

63 new tests added (plan estimated 35–45). All issue categories covered:

| Issue | New tests | Assessment |
|-------|:---------:|------------|
| #87 (symlink safety) | 13 | `fs_util` (10), `parser` (1), `structure` (2) |
| #88 (read_body errors) | 4 | Missing dir, valid body, empty body, error message |
| #89 (path traversal) | 6 | Parent traversal, dot-slash, embedded, multiple, skip S001 |
| #90 (file size cap) | 4 | Normal, oversized, exact 1 MiB, nonexistent |
| #91/#92 (discovery warnings) | 10 | Both `discover_skills_verbose` and `collect_skills_verbose` |
| #93 (TOCTOU) | 4 | Error contains path, does not overwrite existing |
| #94 (CRLF) | 4 | Pure CRLF, mixed, LF unchanged, idempotent |
| #95 (pre-tokenize) | 4 | Tokenize, jaccard_from_sets, match old results, empty sets |
| #96 (depth limits) | 5 | Normal depth, exceeds limit, error message (assembler + validator) |
| #103 (comments) | 6 | Between keys, inline, consecutive, indented, header, idempotent |
| Backward compat | 3 | `discover_skills`, `collect_skills`, `format_entries` |

### 3.2 Edge cases well covered

- Exact boundary test for 1 MiB (tests `>` not `>=`)
- Symlink tests gated with `#[cfg(unix)]`
- S006 verifies traversal paths don't also trigger S001
- CRLF idempotency: `format(format(crlf)) == format(crlf)`
- Comment idempotency: `format(format(comments)) == format(comments)`
- Backward compat tests compare old and new API results

### 3.3 Missing tests (non-blocking)

- `check_symlinks_recursive` with deeply nested symlinks (relates to §1.2)
- `version.sh` — no automated tests (per plan: "manual verification")
- `discover_skills_verbose` at exactly `MAX_DISCOVERY_DEPTH` (boundary)
- `read_file_checked` on a symlink to an oversized file

---

## 4. API Changes

### 4.1 Breaking: `read_body` signature change (#88)

`pub fn read_body(dir: &Path) -> String` → `pub fn read_body(dir: &Path) -> Result<String>`

All callers updated correctly. Three patterns used:

| Pattern | Where | Reason |
|---------|-------|--------|
| `.unwrap_or_default()` | `main.rs` (check, upgrade loops) | Graceful degradation in validation context |
| `?` operator | `main.rs` (body length check) | Must propagate to user |
| `.unwrap_or_default()` | `scorer.rs` | Graceful degradation in scoring context |
| `.unwrap_or_default()` | `structure.rs` | Graceful degradation in structure validation |

The `unwrap_or_default()` usages are reasonable — these callers previously
got an empty string on error, so the behavior is preserved.

### 4.2 Additive: New public types and functions

| Addition | Module | Notes |
|----------|--------|-------|
| `DiscoveryWarning` | `validator` | Struct with `path` and `message` fields |
| `discover_skills_verbose()` | `validator` | Returns `(Vec<PathBuf>, Vec<DiscoveryWarning>)` |
| `collect_skills_verbose()` | `prompt` | Returns `(Vec<SkillEntry>, Vec<DiscoveryWarning>)` |
| `format_entries()` | `prompt` | Formats pre-collected entries |

All re-exported from `lib.rs`. Backward-compatible — original functions
unchanged.

---

## 5. Security Hardening Assessment

### 5.1 SEC-1: Symlink safety (#87) — ✅

- `fs_util.rs` created with `is_regular_file`, `is_regular_dir`, `is_symlink`
- All `path.is_file()` / `path.is_dir()` in security-sensitive paths replaced
- `find_skill_md` rejects symlinked SKILL.md files
- `discover_skills_recursive` skips symlinked directories
- `copy_dir_recursive` skips symlinks
- `S005` diagnostic emitted by structure validation
- `S005` correctly uses `Severity::Info` (not Warning)

### 5.2 SEC-2: Path traversal guard (#89) — ✅

- `contains_path_traversal` uses `Path::components()` — robust against
  false positives (won't match `..` substrings in filenames)
- S006 emitted before existence check (correct: `continue` after S006
  prevents S001)
- Tests cover `../`, `sub/../`, and `./` (no false positive)

### 5.3 SEC-3: File size cap (#90) — ✅

- `MAX_FILE_SIZE = 1_048_576` (1 MiB)
- `read_file_checked` used in parser, validator, fixer, formatter, test_runner
- Boundary test confirms `>` (not `>=`): exactly 1 MiB passes

---

## 6. Commit Hygiene

29 commits on the branch, of which 11 have duplicate messages (each
appearing twice). This is an artifact of the worktree merge strategy:
task branches were merged into `dev/m14`, producing merge commits that
duplicate the original commit summaries.

**Recommendation:** Before merging to `main`, consider squashing the
duplicate pairs. The fixup commits (`fixup: fix rustdoc warning`,
`fixup: use symlink-safe helpers`) should also be squashed into their
parent commits. Target: ~15 clean commits for the final PR.

---

## 7. Scope Comparison

| Metric | Plan (revised) | Actual | Delta |
|--------|:--------------:|:------:|:-----:|
| New files | 2 | 2 (`fs_util.rs`, `version.sh`) | ✅ |
| Deleted files | 1 | 1 (`bump-version.sh`) | ✅ |
| Modified files | 17–18 | 18 | ✅ |
| New tests | 35–45 | 63 | ↑ exceeded |
| Net line delta | +500–700 | +1268 | ↑ exceeded |
| Version | 0.3.0 → 0.4.0 | 0.4.0 | ✅ |

The line delta is higher than estimated, primarily due to more thorough
test coverage (63 vs estimated 45) and the verbose discovery variants
adding more code than projected.

---

## 8. Summary

The M14 implementation is well-executed and ready for merge after
addressing the two bugs (§1.1 and §1.2). The security hardening is
correct and comprehensive. Test coverage exceeds estimates. API changes
are backward-compatible where possible and well-documented where breaking.

### Action items before merge

| Priority | Item | Effort |
|----------|------|--------|
| **Must fix** | §1.1: `version.sh` CHANGES.md sed portability | 10 min |
| **Should fix** | §1.2: Add depth limit to `check_symlinks_recursive` | 5 min |
| **Should fix** | §6: Squash duplicate commits before PR | 10 min |
| Nice to have | §2.3: Extract TOCTOU helper in `builder/mod.rs` | 5 min |
| Nice to have | §2.4: Emit warning at max discovery depth | 5 min |
| Nice to have | §2.6: Fix step numbering in `build_skill` | 1 min |

---

## M14 Code Review (Branch `dev/m14`)

Reviewed branch head `35bcd88` against `main` (`2c2309d`) with focus on
security hardening, reliability changes, formatter behavior, and release
script updates.

### Findings

No blocking defects found in the implemented M14 code.

### Validation Performed

- Ran full test suite: `cargo test` (all passing).
- Spot-checked high-risk changes in:
  - `src/parser.rs`
  - `src/validator.rs`
  - `src/structure.rs`
  - `src/builder/mod.rs`
  - `src/assembler.rs`
  - `src/formatter.rs`
  - `scripts/version.sh`

### Residual Risks / Testing Gaps

- `scripts/version.sh` behavior is not covered by automated tests (README/CHANGES rewrite paths are only manually verified).
- Symlink and permission behavior remains primarily Unix-tested (`#[cfg(unix)]` paths), so cross-platform edge cases still depend on manual verification.

---

## M14 README Review (Branch `dev/m14-105-readme`)

Reviewed `main...dev/m14-105-readme` (README-focused changes).

### Findings

1. High: documented `properties` command does not exist
   - `README.md` documents `properties` as the current command in multiple places (`README.md:68`, `README.md:176`, `README.md:438`, `README.md:471`, `README.md:787`, `README.md:793`).
   - The CLI subcommand is `read-properties` (`src/main.rs:129`), and there is no alias to `properties`.
   - Impact: copied commands fail with "unrecognized subcommand", breaking quick start and CLI reference usability.

2. Medium: exit code behavior for warnings is documented incorrectly for `validate`/`check`
   - README claims warnings cause exit 1 (`README.md:464`, `README.md:475`, `README.md:609`, `README.md:936`).
   - Implementation exits non-zero only when errors are present (`src/main.rs:426`, `src/main.rs:556`).
   - Impact: CI guidance is misleading; users may assume warning-only runs fail when they do not.

### Summary

The README rewrite is strong structurally, but it introduces two functional documentation regressions that should be corrected before merge.
