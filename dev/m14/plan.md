# M14: SRE Review — Work Plan

## Table of Contents

- [Overview](#overview)
- [Baseline](#baseline)
- [Branch Strategy](#branch-strategy)
- [Dependencies](#dependencies)
- [Design Decisions](#design-decisions)
  - [SEC-1: Symlink Safety (#87)](#sec-1-symlink-safety-87)
  - [SEC-2: Path Traversal Guard (#89)](#sec-2-path-traversal-guard-89)
  - [SEC-3: File Size Cap (#90)](#sec-3-file-size-cap-90)
  - [REL-1: `read_body` Error Propagation (#88)](#rel-1-read_body-error-propagation-88)
  - [REL-2 / REL-3: Silent Error Collection (#91, #92)](#rel-2--rel-3-silent-error-collection-91-92)
  - [REL-4: TOCTOU Race (#93)](#rel-4-toctou-race-93)
  - [REL-5: CRLF Handling in Formatter (#94)](#rel-5-crlf-handling-in-formatter-94)
  - [PERF-1 / PERF-3: Pre-Tokenized Conflict Detection (#95)](#perf-1--perf-3-pre-tokenized-conflict-detection-95)
  - [PERF-2: Recursion Depth Limits (#96)](#perf-2-recursion-depth-limits-96)
  - [Formatter Comment Handling (#103)](#formatter-comment-handling-103)
  - [Unified Version Script (#102)](#unified-version-script-102)
  - [README Review (#105) and CLI Acceptance Testing (#106)](#readme-review-105-and-cli-acceptance-testing-106)
- [Wave Plan](#wave-plan)
  - [Wave 1: Security Hardening (#87, #89, #90)](#wave-1-security-hardening-87-89-90)
  - [Wave 2: Reliability Fixes (#88, #91, #92, #93, #94)](#wave-2-reliability-fixes-88-91-92-93-94)
  - [Wave 3: Performance + Cleanup (#95, #96, #102, #103)](#wave-3-performance--cleanup-95-96-102-103)
  - [Wave 4: Verification & Release Prep (#105, #106)](#wave-4-verification--release-prep-105-106)
- [Issue Summary](#issue-summary)
- [Risk Assessment](#risk-assessment)
- [Estimated Scope](#estimated-scope)
- [Worktree Setup](#worktree-setup)
- [Reconciled Plan (post-review amendments)](#reconciled-plan-post-review-amendments)
  - [A1. Design Clarification: `read_body` empty-body semantics (#88)](#a1-design-clarification-read_body-empty-body-semantics-88)
  - [A2. Design Clarification: `AlreadyExists` error message (#93)](#a2-design-clarification-alreadyexists-error-message-93)
  - [A3. Agent Note: Formatter header comments (#103)](#a3-agent-note-formatter-header-comments-103)
  - [A4. Merge Conflict Mitigation](#a4-merge-conflict-mitigation)
  - [A5. Scope Adjustments](#a5-scope-adjustments)
  - [A6. Out-of-Scope Items (confirmed)](#a6-out-of-scope-items-confirmed)


## Overview

Security, reliability, and performance hardening. Addresses findings from
the SRE audit (`dev/m14/audit.md`): symlink safety, path traversal guards,
file size caps, error propagation, CRLF handling, recursion depth limits,
O(n^2) conflict detection, TOCTOU races. Also includes cleanup items
(formatter comment handling, unified version script) and release-gating
items (README review, manual CLI acceptance testing).

Issues: #87, #88, #89, #90, #91, #92, #93, #94, #95, #96, #102, #103,
#105, #106.

## Baseline

Main at `2c2309d` (M13 merged). 481 tests (347 unit + 106 CLI + 27 plugin
+ 1 doc-test). ~19,200 lines across 27 source files.

Audit baseline: all 14 findings confirmed accurate against current main.
REL-5 clarification: CRLF causes `Parse` error (not panic) in formatter.
REL-6: `.lines()` already handles CRLF correctly in `extract_frontmatter_lines`.

## Branch Strategy

- **Dev branch**: `dev/m14` (from `main` at `2c2309d`)
- **Worktrees**: One per wave agent at `/Users/waku/dev/aigent-worktrees/<name>`
  branching from `dev/m14` as `task/m14-<name>`. Autonomous agents work in
  isolated worktrees and merge back to `dev/m14` after wave completion.
- After all waves, PR from `dev/m14` → `main`
- PR body uses `Closes #N` to auto-close issues on merge

## Dependencies

- **M13**: All merged (PR #86, commit `2c2309d`). M14 modifies M13 code
  (`formatter.rs`, `assembler.rs`, `test_runner.rs`).
- **M12**: `scorer.rs`, `conflict.rs`, `structure.rs`, `tester.rs` — all
  targets of M14 hardening.
- **M10**: `parser.rs`, `validator.rs`, `fixer.rs` — foundational modules
  receiving error propagation fixes.
- No new crate dependencies.

---

## Design Decisions

### SEC-1: Symlink Safety (#87)

Add a `is_symlink_safe()` utility in `parser.rs` (or new `src/fs_util.rs`):

```rust
/// Returns `true` if the path is a regular file/directory (not a symlink).
/// Uses `symlink_metadata()` to avoid following symlinks.
fn is_regular_file(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_file())
        .unwrap_or(false)
}

fn is_regular_dir(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_dir())
        .unwrap_or(false)
}
```

Replace all `path.is_file()` / `path.is_dir()` in security-sensitive paths
with these helpers. Affected locations:

| File | Function | Calls to replace |
|------|----------|------------------|
| `parser.rs` | `find_skill_md` | `uppercase.is_file()`, `lowercase.is_file()` |
| `validator.rs` | `discover_skills_recursive` | `path.is_file()`, `path.is_dir()` |
| `structure.rs` | `check_script_permissions_impl` | `path.is_file()` |
| `structure.rs` | `check_nesting_recursive` | `path.is_dir()` |
| `assembler.rs` | `copy_skill_files` | `src_path.is_file()`, `src_path.is_dir()` |
| `assembler.rs` | `copy_dir_recursive` | `src_path.is_file()`, `src_path.is_dir()` |

Symlinks are silently skipped (not followed, not errored). A new diagnostic
`S005: Symlink detected` (Info severity) is emitted by structure validation
when a symlink is found in a skill directory.

### SEC-2: Path Traversal Guard (#89)

In `structure.rs` `check_references`, reject reference paths containing `..`
or canonicalize and verify the resolved path is within the skill directory:

```rust
fn is_path_within(base: &Path, candidate: &Path) -> bool {
    match (base.canonicalize(), candidate.canonicalize()) {
        (Ok(b), Ok(c)) => c.starts_with(&b),
        _ => false,
    }
}
```

Emit `S006: Path traversal in reference link` (Warning) when `..` is
detected. Skip the existence check for traversal paths.

### SEC-3: File Size Cap (#90)

Add constant to `parser.rs`:

```rust
/// Maximum file size for SKILL.md and related files (1 MiB).
const MAX_FILE_SIZE: u64 = 1_048_576;
```

Add a `read_file_checked(path) -> Result<String>` helper that checks
`metadata().len()` before `read_to_string()`. Use it in:

- `parser.rs`: `read_properties`, `read_body`
- `validator.rs`: `validate_with_target`
- `fixer.rs`: `apply_fixes`
- `formatter.rs`: `format_skill`
- `test_runner.rs`: `run_test_suite`

Return `AigentError::Parse` with message "file exceeds 1 MiB size limit"
when exceeded.

### REL-1: `read_body` Error Propagation (#88)

Change signature from `pub fn read_body(dir: &Path) -> String` to
`pub fn read_body(dir: &Path) -> Result<String>`.

This is an API-breaking change. All callers must be updated:

| File | Caller | Current pattern | New pattern |
|------|--------|-----------------|-------------|
| `scorer.rs` | `score()` | `read_body(dir)` | `read_body(dir)?` |
| `structure.rs` | `validate_structure()` | `read_body(dir)` | `read_body(dir)?` |
| `main.rs` | `run_check()`, `run_upgrade()` | `read_body(&dir)` | `read_body(&dir)?` |
| `tester.rs` | `test_skill()` | Uses `read_properties()` directly | No change needed |

For `find_skill_md` returning `None`: return `Err(AigentError::Parse {
message: "no SKILL.md found" })`. For IO errors and parse errors: propagate
with `?`.

### REL-2 / REL-3: Silent Error Collection (#91, #92)

Add a `DiscoveryWarning` type:

```rust
pub struct DiscoveryWarning {
    pub path: PathBuf,
    pub message: String,
}
```

**`discover_skills_recursive`** (#91): Change internal API to accept a
`&mut Vec<DiscoveryWarning>` parameter. Collect IO errors as warnings
instead of silently skipping. The public API
`discover_skills(root) -> Vec<PathBuf>` gains a companion
`discover_skills_verbose(root) -> (Vec<PathBuf>, Vec<DiscoveryWarning>)`.
The original function remains unchanged for backward compatibility.

**`collect_skills`** (#92): Add
`collect_skills_verbose(dirs) -> (Vec<SkillEntry>, Vec<DiscoveryWarning>)`.
The original `collect_skills` stays unchanged. CLI commands (`prompt`,
`doc`, `check`) use the verbose variant and print warnings to stderr.

### REL-4: TOCTOU Race (#93)

Replace check-then-write with atomic exclusive creation:

```rust
// Before (TOCTOU):
if find_skill_md(&output_dir).is_some() {
    return Err(...);
}
std::fs::write(&skill_md_path, &content)?;

// After (atomic):
let file = OpenOptions::new()
    .write(true)
    .create_new(true)
    .open(&skill_md_path)?;
file.write_all(content.as_bytes())?;
```

The `create_new(true)` flag fails atomically if the file already exists.
Map the `AlreadyExists` error kind to `AigentError::Build`.

Apply to both `build_skill()` and `init_skill()` in `builder/mod.rs`.

### REL-5: CRLF Handling in Formatter (#94)

Normalize CRLF to LF at the entry point of `format_content`:

```rust
pub fn format_content(original: &str) -> Result<String> {
    let content = original.replace("\r\n", "\n");
    // ... rest of function operates on normalized content
```

This is a one-line fix at the top of the function. The byte-offset
arithmetic then works correctly because all newlines are `\n`.

Add tests with CRLF input to verify formatting produces correct LF output.

### PERF-1 / PERF-3: Pre-Tokenized Conflict Detection (#95)

Pre-tokenize descriptions before the O(n^2) loop:

```rust
fn check_description_similarity(entries: &[SkillEntry], threshold: f64) -> Vec<Diagnostic> {
    // Pre-tokenize once: O(n)
    let token_sets: Vec<HashSet<String>> = entries
        .iter()
        .map(|e| tokenize(&e.description))
        .collect();

    // Compare pairs: O(n^2) but no per-pair allocation
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let sim = jaccard_from_sets(&token_sets[i], &token_sets[j]);
```

Extract `tokenize(s: &str) -> HashSet<String>` and
`jaccard_from_sets(a: &HashSet<String>, b: &HashSet<String>) -> f64`.

The existing `jaccard_similarity(a: &str, b: &str) -> f64` stays for
backward compatibility and calls `tokenize` + `jaccard_from_sets`
internally.

### PERF-2: Recursion Depth Limits (#96)

Add a `MAX_RECURSION_DEPTH` constant (e.g. 10) and a `depth` parameter to:

| File | Function | Current depth limit |
|------|----------|---------------------|
| `assembler.rs` | `copy_dir_recursive` | None |
| `validator.rs` | `discover_skills_recursive` | None |

Pattern:

```rust
fn copy_dir_recursive(src: &Path, dest: &Path, depth: usize) -> Result<()> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(AigentError::Validate {
            message: format!("exceeded maximum directory depth ({MAX_RECURSION_DEPTH})")
        });
    }
    // ...
    copy_dir_recursive(&src_path, &dest_path, depth + 1)?;
}
```

`structure.rs` `check_nesting_recursive` already has `MAX_NESTING_DEPTH = 2`
— no change needed there.

### Formatter Comment Handling (#103)

In `formatter.rs`, modify `parse_yaml_blocks()` to flush the current key
block before emitting a standalone `YamlBlock::Comment` for non-indented
`#` lines. This keeps comments anchored to their original position rather
than traveling with the preceding key during reordering.

Current behavior:
```yaml
---
name: my-skill
# This comment is about description
description: Does things
---
```

If reordered, comment travels with `name`. After fix, comment stays in
its original relative position.

### Unified Version Script (#102)

Replace `scripts/bump-version.sh` with `scripts/version.sh`:

```
scripts/version.sh              # defaults to show
scripts/version.sh show         # print current version
scripts/version.sh set <x.y.z>  # sync all files
scripts/version.sh bump patch   # auto-increment
scripts/version.sh bump minor
scripts/version.sh bump major
```

Files synced by `set`: `Cargo.toml`, `.claude-plugin/plugin.json`,
`CHANGES.md` stub, `README.md` --about block, `Cargo.lock` via
`cargo check`.

### README Review (#105) and CLI Acceptance Testing (#106)

Manual tasks gated on implementation waves completing. These are
verification activities, not code changes.

---

## Wave Plan

### Wave 1: Security Hardening (#87, #89, #90)

The security fixes are independent of each other but share a common
pattern (input validation at boundaries). No API changes — these are
internal guards.

**Agent A** — Symlink safety (#87)

Worktree: `aigent-worktrees/symlink-safety`
Branch: `task/m14-symlink-safety`

- Files: new `src/fs_util.rs`, `src/parser.rs`, `src/validator.rs`,
  `src/structure.rs`, `src/assembler.rs`, `src/lib.rs`, `src/diagnostics.rs`
- Create `fs_util.rs` with `is_regular_file()`, `is_regular_dir()`,
  `is_symlink()` helpers using `symlink_metadata()`
- Replace all `path.is_file()` / `path.is_dir()` in affected locations
- Add `S005` diagnostic constant to `diagnostics.rs`
- Emit `S005` in structure validation when symlinks detected
- Add `pub mod fs_util` to `lib.rs`
- Tests:
  - `find_skill_md` ignores symlinked SKILL.md
  - `discover_skills_recursive` skips symlinked directories
  - `copy_dir_recursive` skips symlinks
  - Structure validation emits S005 for symlinks

**Agent B** — Path traversal guard (#89)

Worktree: `aigent-worktrees/path-traversal`
Branch: `task/m14-path-traversal`

- Files: `src/structure.rs`, `src/diagnostics.rs`
- Add `S006` diagnostic constant
- In `check_references`, reject paths containing `..` before joining
- Emit `S006: Path traversal in reference link` (Warning)
- Tests:
  - Reference `../../../etc/passwd` produces S006
  - Reference `./scripts/run.sh` passes (no traversal)
  - Reference `sub/../file.md` produces S006

**Agent C** — File size cap (#90)

Worktree: `aigent-worktrees/file-size-cap`
Branch: `task/m14-file-size-cap`

- Files: `src/parser.rs`, `src/validator.rs`, `src/fixer.rs`,
  `src/formatter.rs`, `src/test_runner.rs`
- Add `MAX_FILE_SIZE` constant and `read_file_checked()` helper to `parser.rs`
- Replace `read_to_string` with `read_file_checked` at all affected sites
- Re-export from `lib.rs` if needed by external consumers
- Tests:
  - File under 1 MiB reads successfully
  - File over 1 MiB returns error with descriptive message
  - All existing tests still pass (test fixtures are small)

### Wave 2: Reliability Fixes (#88, #91, #92, #93, #94)

These change function signatures and error handling patterns. They are
independent of each other but depend on Wave 1 (the `read_file_checked`
helper from #90 is used by the new `read_body` implementation in #88).

**Agent D** — `read_body` error propagation (#88)

Worktree: `aigent-worktrees/read-body-errors`
Branch: `task/m14-read-body-errors`

- Files: `src/parser.rs`, `src/scorer.rs`, `src/structure.rs`, `src/main.rs`
- Change `read_body` signature to `Result<String>`
- Propagate errors from `find_skill_md` (None → error), `read_file_checked`,
  `parse_frontmatter`
- Update all callers: `scorer.rs`, `structure.rs`, `main.rs`
- Tests:
  - `read_body` on missing directory returns error
  - `read_body` on valid skill returns body content
  - `scorer::score` propagates read_body errors
  - `structure::validate_structure` propagates read_body errors

**Agent E** — Discovery error collection (#91, #92)

Worktree: `aigent-worktrees/discovery-warnings`
Branch: `task/m14-discovery-warnings`

- Files: `src/validator.rs`, `src/prompt.rs`, `src/lib.rs`, `src/main.rs`
- Add `DiscoveryWarning` struct to `validator.rs`
- Add `discover_skills_verbose()` that collects warnings
- Add `collect_skills_verbose()` to `prompt.rs`
- Update CLI commands (`prompt`, `doc`, `check`) to use verbose variants
  and print warnings to stderr
- Keep original functions unchanged for backward compatibility
- Re-export new types from `lib.rs`
- Tests:
  - `discover_skills_verbose` on unreadable directory collects warning
  - `collect_skills_verbose` on unparseable skill collects warning
  - Original functions still work (backward compat)

**Agent F** — TOCTOU race fix (#93)

Worktree: `aigent-worktrees/toctou-fix`
Branch: `task/m14-toctou-fix`

- Files: `src/builder/mod.rs`
- Replace check-then-write in `build_skill()` and `init_skill()` with
  `OpenOptions::new().create_new(true)`
- Map `AlreadyExists` error to `AigentError::Build`
- Tests:
  - `build_skill` into existing SKILL.md returns error (not overwrite)
  - `init_skill` into existing SKILL.md returns error
  - Verify error message matches previous behavior

**Agent G** — CRLF handling in formatter (#94)

Worktree: `aigent-worktrees/crlf-handling`
Branch: `task/m14-crlf-handling`

- Files: `src/formatter.rs`
- Add `content.replace("\r\n", "\n")` at top of `format_content()`
- Also normalize in `extract_frontmatter_lines` in `main.rs` (for
  `upgrade --apply` frontmatter parsing) if applicable
- Tests:
  - Format a CRLF SKILL.md → produces valid LF output
  - Format a mixed LF/CRLF file → normalizes to LF
  - Existing LF files unchanged
  - Round-trip: format(format(crlf_input)) == format(crlf_input)

### Wave 3: Performance + Cleanup (#95, #96, #102, #103)

Independent optimizations and cleanup tasks. No API changes except
internal refactoring.

**Agent H** — Pre-tokenized conflict detection (#95)

Worktree: `aigent-worktrees/conflict-perf`
Branch: `task/m14-conflict-perf`

- Files: `src/conflict.rs`
- Extract `tokenize()` and `jaccard_from_sets()` functions
- Pre-tokenize in `check_description_similarity` before nested loop
- Keep `jaccard_similarity(a, b)` as a convenience wrapper
- Tests:
  - `tokenize` produces expected word sets
  - `jaccard_from_sets` matches `jaccard_similarity` results
  - Conflict detection results unchanged (same diagnostics)
  - Add benchmark note in doc comment

**Agent I** — Recursion depth limits (#96)

Worktree: `aigent-worktrees/recursion-depth`
Branch: `task/m14-recursion-depth`

- Files: `src/assembler.rs`, `src/validator.rs`
- Add `MAX_RECURSION_DEPTH = 10` constant
- Add `depth` parameter to `copy_dir_recursive` and
  `discover_skills_recursive`
- Return error when depth exceeded
- Tests:
  - Deeply nested directory (> 10 levels) produces error
  - Normal depth (< 10 levels) works correctly
  - `discover_skills_recursive` respects limit

**Agent J** — Unified version script (#102)

Worktree: `aigent-worktrees/version-script`
Branch: `task/m14-version-script`

- Files: `scripts/version.sh` (new), `scripts/bump-version.sh` (delete),
  `README.md`, `CLAUDE.md`
- Implement `show`, `set`, `bump` subcommands
- `show` as default (no args)
- `bump` reads current version, increments, calls `set` internally
- Update documentation references
- Tests: manual verification (shell script)

**Agent K** — Formatter comment handling (#103)

Worktree: `aigent-worktrees/formatter-comments`
Branch: `task/m14-formatter-comments`

- Files: `src/formatter.rs`
- Modify `parse_yaml_blocks()` to flush current key block before
  emitting standalone `YamlBlock::Comment`
- Comments anchored to original position during key reordering
- Tests:
  - Comment between keys stays in position after reorder
  - Inline comments (after values) stay with their key
  - Multiple consecutive comments preserved
  - Indented comments (continuation) stay with preceding key

### Wave 4: Verification & Release Prep (#105, #106)

Depends on all implementation waves being complete and merged to `dev/m14`.

**Agent L-readme** — README review (#105)

Manual task. No worktree needed (runs on `dev/m14` directly).

- Verify all command names match actual CLI output
- Run all code examples from README
- Verify `--about` block matches `cargo run -- --about`
- Check version references
- Verify milestones table
- Check internal anchor links resolve
- Check external links are reachable

**Agent L-cli** — CLI acceptance testing (#106)

Manual task. No worktree needed.

- Exercise every CLI command with each option combination
- Use bundled plugin skills as test fixtures
- Verify global flags: `--version`, `--about`, `--help`
- Test each subcommand: `validate`, `check`, `new`, `prompt`, `probe`,
  `score`, `doc`, `fmt`, `build`, `test`, `upgrade`, `init`,
  `read-properties`
- Test error paths: invalid input, missing files, wrong arguments
- Verify exit codes match documentation

**Agent L-verify** — Full verification

- `cargo fmt --check` — clean
- `cargo clippy -- -D warnings` — clean
- `cargo test` — all tests pass
- `cargo doc --no-deps` — no warnings
- `cargo build --release` — clean
- `cargo build --release --features watch` — clean

---

## Issue Summary

| Wave | Issue | Description | Complexity | Category |
|------|-------|-------------|------------|----------|
| 1 | #87 | Symlink safety | Medium | Security |
| 1 | #89 | Path traversal guard | Low | Security |
| 1 | #90 | File size cap | Low | Security |
| 2 | #88 | `read_body` error propagation | Medium | Reliability |
| 2 | #91 | `discover_skills_recursive` error collection | Medium | Reliability |
| 2 | #92 | `collect_skills` error collection | Medium | Reliability |
| 2 | #93 | TOCTOU race in build/init | Low | Reliability |
| 2 | #94 | CRLF handling in formatter | Low | Reliability |
| 3 | #95 | Pre-tokenized conflict detection | Low | Performance |
| 3 | #96 | Recursion depth limits | Low | Performance |
| 3 | #102 | Unified version script | Low | Cleanup |
| 3 | #103 | Formatter comment handling | Medium | Cleanup |
| 4 | #105 | README review for v0.3.0 | Low | Docs |
| 4 | #106 | Manual CLI acceptance testing | Medium | Test |

## Risk Assessment

- **#87 (SEC-1) is the widest-impact change**: Replacing `is_file()` /
  `is_dir()` across 6 files touches core file-system interactions. Risk
  of breaking existing behavior if symlinks are used legitimately (e.g.,
  development workflows). Mitigation: symlinks are silently skipped, not
  errored — callers that need to follow symlinks can still use the
  standard library functions directly.

- **#88 (REL-1) is the largest API change**: `read_body` signature changes
  from `String` to `Result<String>`. All callers must be updated. This is
  a library-breaking change (pre-1.0, acceptable). Risk: callers that
  previously relied on empty-string-on-error must now handle errors
  explicitly.

- **#91/#92 add new types**: `DiscoveryWarning` and verbose variants are
  additive (backward-compatible). Low risk.

- **#93 changes file creation semantics**: `create_new(true)` fails
  atomically instead of silently overwriting. Existing tests that create
  files in a pre-existing directory should still pass because the tests
  use temp dirs.

- **#103 (formatter comments) is the most algorithmically complex**: The
  YAML block parser must distinguish standalone comments from inline
  comments. Edge cases in deeply nested YAML with mixed comment styles.

- **#105/#106 are manual tasks**: No code changes, but they gate the
  release. Risk of not completing if implementation waves take longer
  than expected.

## Estimated Scope

- **New files**: `src/fs_util.rs` (~40 lines), `scripts/version.sh` (~80
  lines)
- **Modified files**: ~15 files
- **Deleted files**: `scripts/bump-version.sh`
- **New tests**: ~35–45
- **Agents**: 12 (A–K + L-verify, L-readme, L-cli)
- **Net line delta**: +400–600 lines
- **New dependencies**: None

## Worktree Setup

Before Wave 1, create worktrees:

```bash
# Create dev/m14 branch (if not already)
git branch dev/m14 main

# Wave 1 worktrees
git worktree add ../aigent-worktrees/symlink-safety -b task/m14-symlink-safety dev/m14
git worktree add ../aigent-worktrees/path-traversal -b task/m14-path-traversal dev/m14
git worktree add ../aigent-worktrees/file-size-cap -b task/m14-file-size-cap dev/m14

# Wave 2 worktrees (create after Wave 1 merges to dev/m14)
git worktree add ../aigent-worktrees/read-body-errors -b task/m14-read-body-errors dev/m14
git worktree add ../aigent-worktrees/discovery-warnings -b task/m14-discovery-warnings dev/m14
git worktree add ../aigent-worktrees/toctou-fix -b task/m14-toctou-fix dev/m14
git worktree add ../aigent-worktrees/crlf-handling -b task/m14-crlf-handling dev/m14

# Wave 3 worktrees (create after Wave 2 merges to dev/m14)
git worktree add ../aigent-worktrees/conflict-perf -b task/m14-conflict-perf dev/m14
git worktree add ../aigent-worktrees/recursion-depth -b task/m14-recursion-depth dev/m14
git worktree add ../aigent-worktrees/version-script -b task/m14-version-script dev/m14
git worktree add ../aigent-worktrees/formatter-comments -b task/m14-formatter-comments dev/m14
```

After each wave, merge task branches to `dev/m14` and remove worktrees:

```bash
# Example: merge Wave 1 agent A
git checkout dev/m14
git merge task/m14-symlink-safety --no-ff -m "M14: SEC-1 symlink safety (#87)"
git worktree remove ../aigent-worktrees/symlink-safety
git branch -d task/m14-symlink-safety
```

---

## Reconciled Plan (post-review amendments)

Review document: `dev/m14/review.md`. All 11 categories of code references
verified accurate. Audit coverage confirmed complete with no gaps. The
following amendments incorporate the review's recommendations.

### A1. Design Clarification: `read_body` empty-body semantics (#88)

A skill with valid frontmatter but no body after the closing `---` must
return `Ok(String::new())`. This preserves behavioral compatibility for
callers that check `body.is_empty()`. Agent D must implement this explicitly
and add a test for the empty-body case.

### A2. Design Clarification: `AlreadyExists` error message (#93)

When `create_new(true)` fails with `AlreadyExists`, the mapped
`AigentError::Build` message must include the target file path so the user
knows *which* file already exists. Example:

```rust
Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
    Err(AigentError::Build {
        message: format!("SKILL.md already exists: {}", skill_md_path.display()),
    })
}
```

Agent F must implement this and verify the message in tests.

### A3. Agent Note: Formatter header comments (#103)

YAML comments that appear *before the first key* (above `name:`) must be
preserved as header comments, not attached to the first sorted key. The
existing code already handles this (lines 217–220 in `formatter.rs`). Agent
K should add a test exercising this case to prevent regressions.

### A4. Merge Conflict Mitigation

The highest-risk files are `validator.rs` (4 agents across 3 waves) and
`main.rs` (3 agents in Wave 2). The following merge orders are mandatory.

**Wave 1 merge order** (narrowest-first):

1. Agent B (#89) — only `structure.rs` + `diagnostics.rs`
2. Agent A (#87) — widest change across 6 files
3. Agent C (#90) — `parser.rs` + 4 other files

**Wave 2 merge order** (narrowest-first):

1. Agent F (#93) — only `builder/mod.rs`
2. Agent G (#94) — only `formatter.rs`
3. Agent D (#88) — `parser.rs` + callers
4. Agent E (#91/#92) — `validator.rs` + `prompt.rs` + `main.rs`

**Wave 3 merge order** (narrowest-first):

1. Agent H (#95) — only `conflict.rs`
2. Agent J (#102) — only `scripts/`
3. Agent I (#96) — `assembler.rs` + `validator.rs`
4. Agent K (#103) — only `formatter.rs`

**`diagnostics.rs` coordination** (Wave 1): Agent A adds *both* `S005` and
`S006` constants to avoid a trivial merge conflict with Agent B. Agent B
then imports `S006` rather than defining it.

**`discover_skills_recursive` strategy**: Three agents (A, E, I) modify
this function across three waves. Each wave inherits the merged state from
the prior wave. Agent I (Wave 3, depth limits) should treat the function
holistically — reading the accumulated state from A + E before applying its
changes — rather than patching incrementally from the original baseline.

### A5. Scope Adjustments

| Metric | Original | Revised |
|--------|----------|---------|
| Modified files | ~15 | 17–18 (counting `lib.rs`, `diagnostics.rs` more carefully) |
| Net line delta | +400–600 | +500–700 (`DiscoveryWarning` types + verbose variants add more than estimated) |

All other metrics unchanged.

### A6. Out-of-Scope Items (confirmed)

These are explicitly **not** part of M14:

- **SEC-4 / SEC-5**: LOW severity, intentionally deferred.
- **Version bump to 0.3.0**: Separate release step after M14 PR merges.
- **CHANGES.md content**: The version script (#102) stubs an entry; actual
  changelog prose is a release step.
- **Tag push / crate publish**: Post-merge release workflow.
