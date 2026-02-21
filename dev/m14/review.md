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
