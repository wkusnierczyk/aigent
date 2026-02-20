## Table of Contents

- [M13: Enhancements — Work Plan (OBSOLETE)](#m13-enhancements--work-plan-obsolete)
  - [Overview](#overview)
  - [Branch Strategy](#branch-strategy)
  - [Dependencies](#dependencies)
  - [Current State (after M12 + hotfixes)](#current-state-after-m12--hotfixes)
  - [Design Decisions](#design-decisions)
  - [Wave Plan](#wave-plan)
  - [Issue Summary](#issue-summary)
  - [Risk Assessment](#risk-assessment)
  - [Estimated Scope](#estimated-scope)
  - [Reconciliation (2026-02-20)](#reconciliation-2026-02-20)
- [M13: Enhancements — Work Plan (Consolidated)](#m13-enhancements--work-plan-consolidated)
  - [Overview](#overview-1)
  - [Baseline](#baseline)
  - [Branch Strategy](#branch-strategy-1)
  - [Dependencies](#dependencies-1)
  - [CLI Naming Decisions (#76)](#cli-naming-decisions-76)
  - [Design Decisions](#design-decisions-1)
  - [Wave Plan](#wave-plan-1)
  - [Issue Summary](#issue-summary-1)
  - [Risk Assessment](#risk-assessment-1)
  - [Estimated Scope](#estimated-scope-1)


# M13: Enhancements — Work Plan (OBSOLETE)

> **⚠ This version of the plan is obsolete.** It was written incrementally
> over multiple review rounds and accumulated inconsistencies. The
> consolidated plan follows below, after the `---` separator.
>
> Retained for historical record of the decision-making process.

## Overview

Enhancements across the CLI surface and library internals. Covers CLI naming
alignment, scorer output polish, tester algorithm upgrade, upgrade command
robustness, YAML handling, a new formatting command, new `build` (skill→plugin)
and `test` (fixture-based) subcommands, version management, structured warnings,
and minor doc/hook audits.

Issues: #45, #74, #75, #76, #78, #79, #80, #81, #82, #83, #84, #85.

## Branch Strategy

- **Dev branch**: `dev/m13` (created from `main` after v0.2.3 release)
- **Task branches**: `task/m13-<name>` (created from `dev/m13`)
- After each wave, task branches merge into `dev/m13`
- After all waves, PR from `dev/m13` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- **M12**: All 9 issues merged (PR #77). M13 modifies code introduced in M12
  (`scorer.rs`, `tester.rs`, `main.rs` upgrade logic).
- **M11**: Template system, prompt generation — unchanged by M13.
- **M10**: Diagnostic infrastructure — unchanged by M13.
- **#83, #84**: New features that depend on Wave 3 renames (#76) to free up
  the `build` and `test` names. Placed in Wave 4.
- No new external crate dependencies anticipated (except potentially for #81
  if a YAML AST-preserving library is chosen).

## Current State (after M12 + hotfixes)

Main at `e991b69` (M12 merged + version hotfixes). 416 tests (314 unit +
75 CLI + 26 plugin + 1 doc-test). ~15,500 lines across 22 source files.

CLI has 10 subcommands: `validate`, `lint`, `read-properties`, `to-prompt`,
`build`, `score`, `doc`, `test`, `upgrade`, `init`. After M13 #76, primary
names become: `validate`, `check`, `prompt`, `new`, `probe`, `score`, `doc`,
`fmt`, `upgrade`, `init`, `read-properties`.

### M12 Review Residual Bugs — Status

The M12 code review identified 4 bugs. Three were fixed in the hotfix
commits after M12 merge (PR #77, commit `0cb0713`). One remains:

| Bug | Status | Resolution |
|-----|--------|------------|
| `jaccard_similarity()` case-sensitive | **Fixed** on main | `.to_lowercase()` added (commit `0cb0713`) |
| `format_doc_catalog()` passes file path to `read_properties()` | **Fixed** on main | Resolves to parent dir (commit `0cb0713`) |
| `upgrade --apply` silently skips partial metadata | **Fixed** on main | Line-level insertion under existing `metadata:` block (commit `0cb0713`) |
| `read_body()` duplicated in 3 files | **Open** | 3 identical copies in `scorer.rs`, `structure.rs`, `main.rs` |

### Doc-Comments — Status

Issue #75 lists 5 public items missing doc-comments. All 5 already have
`///` doc-comments on the current `main`:

| Item | File | Status |
|------|------|--------|
| `AnthropicProvider` struct | `src/builder/providers/anthropic.rs` | ✅ Has `///` |
| `GoogleProvider` struct | `src/builder/providers/google.rs` | ✅ Has `///` |
| `OllamaProvider` struct | `src/builder/providers/ollama.rs` | ✅ Has `///` |
| `capitalize_first` fn | `src/builder/util.rs` | ✅ Has `///` |
| `to_title_case` fn | `src/builder/util.rs` | ✅ Has `///` |

The remaining work for #75 is adding `#![warn(missing_docs)]` to `src/lib.rs`
as a permanent quality gate.

---

## Design Decisions

### Doc-Comments Quality Gate (#75)

Add `#![warn(missing_docs)]` to `src/lib.rs`. This makes future missing
doc-comments a compile warning (and an error under `cargo clippy -- -D
warnings`). All existing public items already have doc-comments — this is
purely a gating change.

### Score Check Labels (#78)

Add a `fail_label` field to `CheckResult`:

```rust
pub struct CheckResult {
    /// Label shown when the check passes.
    pub label: String,
    /// Label shown when the check fails (if different from pass label).
    pub fail_label: Option<String>,
    pub passed: bool,
    pub message: Option<String>,
}
```

`format_text()` uses `fail_label.as_deref().unwrap_or(&label)` when
`passed == false`. This keeps the `CheckResult` backward-compatible (JSON
serialization unchanged — `fail_label` is `#[serde(skip_serializing_if = "Option::is_none")]`).

Label pairs:

| Check | Pass | Fail |
|-------|------|------|
| Unknown fields | No unknown fields | Unknown fields found |
| Third-person | Third-person description | Not third-person description |
| Trigger phrase | Trigger phrase present | Trigger phrase missing |
| Gerund name | Gerund name form | Non-gerund name form |
| Specific name | Specific name | Generic name |
| Detailed description | Detailed description | Description too short |
| Name format | Name format valid | Name format invalid |
| Description valid | Description valid | Description invalid |
| Required fields | Required fields present | Required fields missing |
| SKILL.md | SKILL.md exists and is parseable | SKILL.md missing or unparseable |
| Body size | Body within size limits | Body exceeds size limits |

### Tester Weighted Scoring (#79)

Replace the single word-overlap ratio in `compute_query_match()` with a
weighted formula returning a numeric score (0.0–1.0):

```
score = 0.5 * jaccard(query_tokens, desc_tokens)
      + 0.3 * trigger_match(query, triggers)
      + 0.2 * name_match(query_tokens, name_tokens)
```

Where:
- `jaccard(A, B) = |A ∩ B| / |A ∪ B|` (lowercase word tokens, stopwords
  removed)
- `trigger_match = 1.0` if any query token appears in a "Use when..." trigger
  phrase, `0.0` otherwise
- `name_match = 1.0` if any query token is a substring of the skill name,
  `0.0` otherwise

Stopword list (defined inline in `tester.rs`):
```rust
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "of", "to", "in",
    "for", "on", "with", "and", "or", "but", "not", "it", "this", "that",
];
```

The `QueryMatch` enum remains (Strong ≥ 0.5, Weak ≥ 0.2, None < 0.2) but
`TestResult` gains a `pub score: f64` field for granular feedback.

The trigger phrase is extracted from the description by scanning for lines
starting with "Use when" or "Use this when" (case-insensitive). If no trigger
phrase exists, the trigger component scores 0.0 (the 30% weight is
effectively lost, penalizing skills without trigger phrases).

### Upgrade --full (#80)

Add a `--full` flag to the `Upgrade` subcommand:

```rust
Upgrade {
    skill_dir: PathBuf,
    #[arg(long)]
    apply: bool,
    #[arg(long)]
    full: bool,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}
```

When `--full` is set:
1. Run `validate()` + `lint()` diagnostics
2. If `--apply`: call `apply_fixes()` first (fix invalid metadata)
3. Re-read the file (fixes may have changed content)
4. Run the upgrade pipeline (add recommended fields)
5. Report results in two sections:

```
Validation fixes applied (--full):
  ✓ Fixed name casing: MySkill → my-skill

Upgrade suggestions:
  ⚠ Missing 'compatibility' field
  ⚠ Missing 'metadata.version'
```

Without `--full`, behavior is unchanged.

### YAML AST-Preserving Parser (#81)

Replace string-append in `upgrade --apply` with structured YAML manipulation.

The partial metadata bug is already fixed on main (line-level insertion under
existing `metadata:` block). This issue focuses on improving that approach:

Options to evaluate:

1. **Improved line-level manipulation** (preferred): Enhance the existing
   line-level approach with better edge-case handling — comments between keys,
   unusual indentation, metadata blocks at different positions. No new
   dependency.

2. **`yaml-rust2`**: YAML library with AST access. Can parse to a `Yaml`
   tree, modify, and re-emit, but strips comments.

3. **Custom comment-aware parser**: Pre/post-processing layer that extracts
   comments, modifies the tree via `serde_yaml_ng`, and reinserts comments.

Decision deferred to implementation. If no clean solution exists, keep the
improved line-level approach with better edge-case tests. This should not
block the rest of M13.

### Structured Warning Channel (#45)

Replace `eprintln!` calls in `src/builder/mod.rs` with a structured warning
mechanism. Currently 11 `eprintln!` calls emit warnings when LLM calls fail
and fall back to deterministic generation. These work for CLI usage but are
invisible to library consumers.

Options:
1. **`BuildResult` warnings field**: Add `pub warnings: Vec<String>` to
   `BuildResult`. The CLI prints them, library consumers can inspect them.
2. **`tracing` or `log` crate**: Standard Rust logging. More conventional but
   adds a dependency.
3. **Callback function**: Pass a `Box<dyn Fn(&str)>` for warning output.

Option 1 is preferred — no new dependency, matches the pattern of
`ScoreResult` carrying diagnostic information.

### Hook Variable Audit (#74)

Audit `hooks/hooks.json` quoting conventions against Claude Code's variable
substitution model. Document findings as comments in the hook file and/or
README. Low effort — purely documentary.

### Version Management (#82)

Add `scripts/bump-version.sh` that updates all version locations atomically:

1. `Cargo.toml` — `version = "x.y.z"`
2. `Cargo.lock` — regenerated by `cargo check`
3. `.claude-plugin/plugin.json` — `"version": "x.y.z"`
4. `CHANGES.md` — adds `## [x.y.z] — YYYY-MM-DD` stub

Document `cargo-edit` (`cargo set-version`) as optional tooling in README.

### CLI Naming Alignment (#76)

All renames decided. Old names preserved as hidden aliases for backward
compatibility during transition.

1. **`build` → `new`**: Like `cargo new`. The current `build` creates new
   source from a description — that's `new`, not `build`. Reserve `build`
   for future skill→plugin conversion (#83).

2. **`to-prompt` → `prompt`**: Bare noun, idiomatic CLI naming. `prompt`
   describes what you get (like `cargo doc`, `git diff`).

3. **`test` → `probe`**: Current `test` simulates activation matching —
   that's probing, not testing. Reserve `test` for fixture-based testing
   with assertions (#84).

4. **`validate` and `check` — differentiated roles** (not a rename):
   - `validate` = spec conformance (syntax, schema, required fields, types).
     Stays as-is — structural, deterministic, fast.
   - `check` = superset: validate + semantic quality. Runs validate first,
     then semantic analysis (description quality, trigger phrases, naming).
     Absorbs `lint` (which was always the semantic part). `lint` preserved
     as hidden alias for `check`.
   - `check --no-validate` = semantic quality only (skip conformance).
   - Quality spectrum: `validate` → `check` → `score` (spec → sense → quality).

5. **Add `fmt`**: New subcommand that normalizes YAML frontmatter (key
   ordering, quoting style, indentation) and markdown body (heading levels,
   trailing newlines). Idempotent. Distinct from `validate --apply-fixes`
   (which fixes errors, not style). New `src/formatter.rs` module (~200
   lines). This is a **new feature**, not cleanup — included here because
   it's part of the CLI naming alignment story.

**Ripple effects:** Renaming subcommands requires updating:
- `skills/aigent-scorer/SKILL.md` — `allowed-tools` references
- `skills/aigent-builder/SKILL.md` — `allowed-tools` and description references
- `skills/aigent-validator/SKILL.md` — if applicable
- `hooks/hooks.json` — hook command references
- `README.md` — CLI documentation
- `CHANGES.md` — only for new version entry (historical entries stay as-is)
- `tests/cli.rs` — CLI integration tests
- `tests/plugin.rs` — plugin tests referencing subcommand names

Historical documents (`dev/m*/plan.md`, `dev/m*/review.md`) are **not
updated** — they're historical records of past decisions.

### `fmt` Subcommand Design (part of #76)

```rust
Fmt {
    /// Paths to skill directories or SKILL.md files
    skill_dirs: Vec<PathBuf>,
    /// Check formatting without modifying files (exit 1 if unformatted)
    #[arg(long)]
    check: bool,
    /// Discover skills recursively
    #[arg(long)]
    recursive: bool,
}
```

Formatting rules (idempotent):

**YAML frontmatter:**
- Canonical key order: `name`, `description`, `instructions`, `compatibility`,
  `context`, `allowed-tools`, `metadata` (alphabetical within `metadata`)
- Consistent quoting: bare values for simple strings, double-quoted for
  strings containing YAML special characters
- 2-space indentation for nested keys
- No trailing whitespace on YAML lines

**Markdown body:**
- Single `#` for top-level heading (if present)
- Consistent list markers (`-` for unordered)
- Single blank line between sections
- No trailing whitespace
- File ends with exactly one newline

Implementation: new `src/formatter.rs` module with `format_skill(path) ->
Result<FormattedResult>` where `FormattedResult` contains `changed: bool` and
`content: String`. The CLI writes the file only if `changed` is true.

---

## Wave Plan

### Wave 1: Low-Risk Fixes (#75, #78, #74, #82, `read_body()` dedup)

Minimal-scope changes with no API surface modifications. Safe to implement
and verify independently.

#### Agent A: Doc-Comments Gate + Hook Audit (#75, #74)

**Files**: `src/lib.rs`, `hooks/hooks.json`

Steps:
1. Add `#![warn(missing_docs)]` to `src/lib.rs`
2. Verify `cargo doc --no-deps` produces no warnings
3. Verify `cargo clippy -- -D warnings` passes
4. Audit `hooks/hooks.json` quoting — verify `$TOOL_INPUT` substitution
   model against Claude Code docs. Add clarifying comments to hook file.

Tests: No new tests needed — `cargo doc` and `cargo clippy` serve as the
verification for #75. Hook audit is documentary.

#### Agent B: Score Check Labels (#78)

**Files**: `src/scorer.rs`, `tests/cli.rs`

Steps:
1. Add `fail_label: Option<String>` to `CheckResult` struct with
   `#[serde(skip_serializing_if = "Option::is_none")]`
2. Populate `fail_label` for all 11 checks in `score_structural()` and
   `score_quality()`
3. Update `format_text()` to use `fail_label` when `passed == false`
4. Update existing tests that assert on label text
5. Add test: score a broken skill, verify `[FAIL]` lines use fail labels

#### Agent C: `read_body()` Deduplication + Version Management (#82)

**Files**: `src/parser.rs`, `src/scorer.rs`, `src/structure.rs`, `src/main.rs`,
`README.md`

Steps:
1. **Extract `read_body()`**: Move the duplicated `read_body()` function from
   `scorer.rs`, `structure.rs`, and `main.rs` into `parser.rs` as
   `pub fn read_body(dir: &Path) -> String`. Update all 3 call sites.
   Existing tests remain unchanged (they test through higher-level APIs).
2. **Version management**: Create `scripts/bump-version.sh` that updates
   `Cargo.toml`, `.claude-plugin/plugin.json`, `CHANGES.md` stub, and runs
   `cargo check` to regenerate `Cargo.lock`. Document `cargo-edit` in
   README's "Optional tooling" subsection.

Tests:
- Existing tests verify `read_body()` through higher-level APIs — no new
  tests needed for the extraction.
- Version script is verified manually.

### Wave 2: Algorithm Improvements (#79, #80, #81, #45)

Changes to tester scoring, upgrade pipeline, YAML handling, and builder
warnings. Agents D, E, and F are independent. Agent E depends on Agent F
being deferred or completed (see note).

#### Agent D: Tester Weighted Scoring (#79)

**Files**: `src/tester.rs`, `tests/cli.rs`

Steps:
1. Add `pub score: f64` field to `TestResult`
2. Define `STOPWORDS` constant (minimal list: articles, prepositions,
   conjunctions)
3. Replace `compute_query_match()` internals with the weighted formula:
   - Extract trigger phrase from description ("Use when..." lines)
   - Compute Jaccard similarity on lowercase word tokens (stopwords removed)
   - Compute name token overlap
   - Weight: 0.5 * jaccard + 0.3 * trigger + 0.2 * name
4. Map score to `QueryMatch` enum (Strong ≥ 0.5, Weak ≥ 0.2, None < 0.2)
5. Update `format_test_result()` to show numeric score alongside
   Strong/Weak/None label
6. Update `test_skill()` to populate the new `score` field
7. Update existing tests — scores may change with new formula, recalibrate
   expected `QueryMatch` values
8. Add tests:
   - Trigger phrase match boosts score vs. same skill without trigger
   - Name match boosts score vs. same description with unrelated name
   - All-zero inputs produce score 0.0

#### Agent E: Upgrade --full (#80)

**Files**: `src/main.rs`, `tests/cli.rs`

Steps:
1. Add `--full` flag to `Upgrade` subcommand in CLI enum
2. In `run_upgrade()`, when `full` is true:
   a. Run `validate()` + `lint()` to collect diagnostics
   b. If `apply`: call `fixer::apply_fixes()` on fixable diagnostics
   c. Re-read the file (fixes may have changed content)
   d. Continue with upgrade pipeline
3. Separate output into "Validation fixes" and "Upgrade suggestions" sections
   when `--full` is used
4. Fix `upgrade --apply` write error handling: replace `unwrap_or_else` with
   proper `?` propagation that returns `Err`
5. Add tests:
   - `upgrade --full` applies validation fixes then upgrade suggestions
   - `upgrade --full` output has separate sections
   - `upgrade --apply` write failure propagates error

Note: The partial metadata fix is already on main. Agent E focuses on the
`--full` flag and error handling only.

#### Agent F: YAML AST-Preserving Parser (#81)

**Files**: `src/main.rs` (or new `src/yaml_utils.rs`)

Steps:
1. Evaluate approach: improved line-level manipulation vs. library-based
2. If line-level (preferred for minimal dependency):
   a. Enhance the existing line-level insertion in `run_upgrade()` to handle
      edge cases: comments between keys, unusual indentation, metadata blocks
      at different positions in the frontmatter
   b. Preserve comments and quoting style
3. If library-based: add dependency, implement parse-modify-serialize
4. Add tests:
   - Comments in frontmatter are preserved after upgrade --apply
   - Existing key values are not modified
   - Edge case: metadata block with unusual indentation
   - Edge case: metadata block followed by comments

Note: Agent F is independent of Agent E — the partial metadata fix is already
on main, so F refines the existing implementation rather than depending on E.
If #81 is deferred, the current line-level approach on main is sufficient.

#### Agent G-alt: Structured Warning Channel (#45)

**Files**: `src/builder/mod.rs`, `src/builder/providers/*.rs`

Steps:
1. Add `pub warnings: Vec<String>` field to `BuildResult`
2. Replace `eprintln!` calls in builder with `warnings.push(...)` collection
3. Update CLI (`main.rs`) to print collected warnings after build completes
4. Library consumers now get structured access to warnings
5. Add test: build with unavailable LLM provider → warnings contain fallback
   message

### Wave 3: CLI Surface Redesign (#76)

The largest change. Depends on Waves 1-2 being complete so that all tests
pass before restructuring the CLI surface.

#### Agent G: CLI Renames and Aliases

**Files**: `src/main.rs`, `tests/cli.rs`

Steps:
1. Rename `Build` → `New` in `Commands` enum. Update help text to "Create a
   new skill from a natural language description". Add `#[command(alias = "build")]`
   for backward compatibility.
2. Rename `ToPrompt` → `Prompt` in `Commands` enum. Update help text. Add
   `#[command(alias = "to-prompt")]` for backward compatibility.
3. Rename `Test` → `Probe` in `Commands` enum. Update help text to "Probe a
   skill's activation surface with a sample query". Add
   `#[command(alias = "test")]` for backward compatibility.
4. Keep `Validate` as-is (spec conformance). Add new `Check` variant that
   runs validate + semantic quality (superset). Add `--no-validate` flag
   to skip conformance. Add `#[command(alias = "lint")]` on `Check` for
   backward compatibility. Remove the standalone `Lint` variant (absorbed
   into `Check`).
5. Update all CLI tests that reference old subcommand names
6. Update `README.md` CLI documentation
7. Add tests:
   - `new`/`build` alias produces same output
   - `prompt`/`to-prompt` alias produces same output
   - `probe`/`test` alias produces same output
   - `check` runs validate + lint (superset behavior)
   - `check --no-validate` skips conformance checks
   - `validate` still works standalone (spec conformance only)
   - `lint` alias maps to `check`

#### Agent H: `fmt` Subcommand

**Files**: new `src/formatter.rs`, `src/main.rs`, `src/lib.rs`, `tests/cli.rs`

Steps:
1. Create `src/formatter.rs`:
   - `pub fn format_skill(path: &Path) -> Result<FormatResult>`
   - `FormatResult { changed: bool, content: String }`
   - `fn format_frontmatter(yaml: &str) -> String` — canonical key order,
     consistent quoting, 2-space indent
   - `fn format_body(body: &str) -> String` — list markers, blank lines,
     trailing whitespace, final newline
2. Add `pub mod formatter;` to `src/lib.rs` with re-exports
3. Add `Fmt` subcommand to CLI:
   ```rust
   Fmt {
       skill_dirs: Vec<PathBuf>,
       #[arg(long)]
       check: bool,
       #[arg(long)]
       recursive: bool,
   }
   ```
   Add `#[command(alias = "format")]` for the alias.
4. Implement `run_fmt()`: discover skills, format each, write if changed.
   With `--check`: report unformatted files and exit 1.
5. Add tests:
   - Format an already-formatted skill → no change
   - Format a skill with wrong key order → keys reordered
   - Format with `--check` on unformatted skill → exit code 1
   - Format preserves frontmatter values (no data loss)
   - Recursive discovery works

#### Agent I: Ripple Effect Updates

**Files**: `skills/aigent-scorer/SKILL.md`, `skills/aigent-builder/SKILL.md`,
`skills/aigent-validator/SKILL.md`, `hooks/hooks.json`, `tests/plugin.rs`,
`CHANGES.md`, `README.md`

Steps:
1. Run `grep -r "aigent build\|aigent validate\|aigent lint\|aigent test\|aigent to-prompt" --include="*.md" --include="*.json"`
   to enumerate all files with old subcommand references
2. Update skill files: `allowed-tools` references to match renamed subcommands
   (`build`→`new`, `test`→`probe`, `to-prompt`→`prompt`, `lint`→`check`).
   `validate` stays as-is.
3. Update `hooks/hooks.json`: hook command references (if any reference `lint`→`check`)
4. Update `tests/plugin.rs`: assertions on subcommand names in skill content
5. Update `CHANGES.md`: add M13 entry (not historical entries)
6. Do NOT update historical documents (`dev/m*/plan.md`, `dev/m*/review.md`)
7. Verify all plugin tests pass

### Wave 4: New Feature Commands (#83, #84)

Depends on Wave 3 (the renames must land first so `build` and `test` are
freed up as primary names).

#### Agent J: `build` Subcommand — Skill→Plugin Assembly (#83)

**Files**: new `src/assembler.rs`, `src/main.rs`, `src/lib.rs`, `tests/cli.rs`

Steps:
1. Create `src/assembler.rs`:
   - `pub fn assemble_plugin(skills: &[&Path], opts: AssembleOptions) -> Result<AssembleResult>`
   - `AssembleOptions { output_dir: PathBuf, name: Option<String>, validate: bool }`
   - `AssembleResult { plugin_dir: PathBuf, skills_count: usize }`
   - Generate `plugin.json` manifest from skill frontmatter
   - Copy skill directories into `skills/` subdirectory
   - Scaffold empty `agents/`, `hooks/` directories
2. Add `pub mod assembler;` to `src/lib.rs` with re-exports
3. Add `Build` subcommand to CLI:
   ```rust
   Build {
       skill_dirs: Vec<PathBuf>,
       #[arg(long, default_value = "./dist")]
       output: PathBuf,
       #[arg(long)]
       name: Option<String>,
       #[arg(long)]
       validate: bool,
   }
   ```
4. Implement `run_build()`: collect skills, assemble plugin, optionally
   validate the result with existing `validate()` infrastructure.
5. Add tests:
   - Single skill → valid plugin directory with plugin.json
   - Multiple skills → bundled into one plugin
   - `--validate` flag runs validation on output
   - Generated plugin.json has correct structure
   - Skill files are copied (not moved)

#### Agent K: `test` Subcommand — Fixture-Based Testing (#84)

**Files**: new `src/test_runner.rs`, `src/main.rs`, `src/lib.rs`, `tests/cli.rs`

Steps:
1. Create `src/test_runner.rs`:
   - `pub fn run_test_suite(skill_dir: &Path) -> Result<TestSuiteResult>`
   - `TestSuiteResult { passed: usize, failed: usize, results: Vec<TestCaseResult> }`
   - `TestCaseResult { input: String, expected_match: bool, actual_match: bool, score: f64 }`
   - Parse `tests.yml` from skill directory
   - Run each test case through `probe` (née `test_skill()`)
   - Compare actual result against expected
2. Define `tests.yml` schema:
   ```yaml
   queries:
     - input: "help me write a skill"
       should_match: true
       min_score: 0.5          # optional
     - input: "what's the weather"
       should_match: false
   ```
3. Add `pub mod test_runner;` to `src/lib.rs` with re-exports
4. Add `Test` subcommand to CLI:
   ```rust
   Test {
       skill_dirs: Vec<PathBuf>,
       #[arg(long, value_enum, default_value_t = Format::Text)]
       format: Format,
       #[arg(long)]
       recursive: bool,
       #[arg(long)]
       generate: bool,
   }
   ```
5. Implement `run_test()`: discover skills, load fixtures, run suite, report.
   With `--generate`: create a starter `tests.yml` from skill metadata.
6. Add tests:
   - All-pass suite → exit code 0
   - Suite with failure → exit code 1
   - Missing `tests.yml` → helpful error (exit code 2)
   - JSON output format
   - `--generate` creates valid fixture file

### Wave 5: Documentation & Verification

#### Agent L-doc: README Rewrite (#85)

**Files**: `README.md`

Steps:
1. Rewrite the CLI command reference section to use new primary names
   (`new`, `prompt`, `probe`, `validate`, `check`, `fmt`, `build`, `test`)
2. Update all examples to use new command names
3. Add documentation for new commands: `check`, `fmt`, `build` (#83), `test` (#84)
4. Document the validate/check/score quality spectrum
5. Document behavioral changes: `check` as superset of `validate`, `probe`
   numeric scores, `score` pass/fail labels, `upgrade --full`
5. Add a brief note about hidden aliases for backward compatibility
6. Update quick start / getting started section

#### Agent L: Full Verification

Steps:
1. `cargo fmt --check` — clean
2. `cargo clippy -- -D warnings` — clean (including `#![warn(missing_docs)]`)
3. `cargo test` — all tests pass
4. `cargo doc --no-deps` — no warnings
5. `cargo build --release` — clean
6. Manual smoke test (using both old and new names where aliased):
   - `aigent new "test skill" --no-llm` works
   - `aigent build skills/aigent-builder/ --output /tmp/plugin` works
   - `aigent prompt skills/` works
   - `aigent to-prompt skills/` works (alias)
   - `aigent probe skills/aigent-builder/ "generate a skill"` shows numeric
     score
   - `aigent test skills/aigent-builder/` runs fixture suite (if tests.yml
     exists)
   - `aigent validate skills/` works (spec conformance only)
   - `aigent check skills/` works (validate + semantic quality)
   - `aigent check skills/ --no-validate` works (semantic only)
   - `aigent lint skills/` works (alias for `check`)
   - `aigent fmt skills/ --check` reports status
   - `aigent score skills/aigent-builder/` shows correct fail labels
   - `aigent upgrade skills/aigent-builder/ --full` composes validate +
     upgrade
   - `./scripts/bump-version.sh` updates all version files

---

## Issue Summary

| Wave | Issue | Description | Complexity |
|------|-------|-------------|------------|
| 1 | #75 | Add `#![warn(missing_docs)]` quality gate | Low |
| 1 | #74 | Audit hook variable quoting conventions | Low |
| 1 | #78 | Separate pass/fail labels in scorer output | Low |
| 1 | #82 | Version management script + docs | Low |
| 1 | — | `read_body()` deduplication (M12 residual) | Low |
| 2 | #79 | Weighted scoring formula for tester | Medium |
| 2 | #80 | `upgrade --full` flag + error handling | Medium |
| 2 | #81 | YAML AST-preserving manipulation for upgrade | Medium–High |
| 2 | #45 | Replace `eprintln!` with structured warnings | Medium |
| 3 | #76 | CLI renames (`new`, `prompt`, `probe`) + new `check`/`fmt` + ripple effects | Medium |
| 4 | #83 | `build` subcommand: skill→plugin assembly | Medium |
| 4 | #84 | `test` subcommand: fixture-based testing | Medium |
| 5 | #85 | README rewrite for M13 CLI surface changes | Low |
| 5 | — | Full verification pass | Low |

## Risk Assessment

- **#81 is the highest-risk item**: YAML comment preservation is a known hard
  problem. If no clean solution emerges, the fallback is improved line-level
  manipulation with better edge-case tests. This should not block the rest of
  M13.

- **#76 is the widest-impact item**: Renaming subcommands touches CLI, tests,
  skills, hooks, and README. Wave 3 positioning ensures all other changes are
  stable before restructuring the surface. The `fmt` subcommand within #76 is
  a **new feature** (~200 lines, new module) — the largest single deliverable
  in this milestone.

- **#79 changes test expectations**: The new scoring formula will produce
  different `QueryMatch` classifications for some inputs. Existing tests must
  be recalibrated, not just updated.

- **#45 changes the builder API**: Adding `warnings: Vec<String>` to
  `BuildResult` is a minor API addition. Library consumers gain access to
  warning information they previously couldn't see.

- **#83 introduces a new module** (`assembler.rs`): Skill→plugin assembly
  is a new capability. The main risk is getting the `plugin.json` generation
  right — must match the Anthropic plugin spec exactly. Low-medium risk since
  the spec is well-documented.

- **#84 introduces a new module** (`test_runner.rs`): Fixture-based testing
  depends on `probe` (the renamed `test_skill()`). Risk is in the YAML schema
  design for `tests.yml` — keep it simple and extensible. Low-medium risk.

## Estimated Scope

- **New files**: `src/formatter.rs` (~200 lines), `src/assembler.rs` (~150
  lines), `src/test_runner.rs` (~200 lines), `scripts/bump-version.sh`
  (~30 lines)
- **Modified files**: ~16 files
- **New tests**: ~45–55
- **Agents**: 13 (A–L + L-doc)
- **Net line delta**: +900–1200 lines
- **New dependencies**: None anticipated (unless #81 adopts a YAML library)

---

## Reconciliation (2026-02-20)

### Baseline Update

The plan was originally written against M12 merge commit `0d6de3d`. The
actual baseline is `e991b69` (after version hotfixes `v0.2.1` → `v0.2.3`).
Key changes on the updated baseline:

1. **Three M12 residual bugs already fixed** (commit `0cb0713`):
   - Jaccard case-sensitivity → `.to_lowercase()` added
   - Doc catalog path → resolves to parent directory
   - Partial metadata → line-level insertion under existing `metadata:` block

2. **All 5 doc-comments already present**: The items listed in the original
   plan for #75 already have `///` doc-comments. Only the
   `#![warn(missing_docs)]` gate remains.

3. **Version sync tests added** (commits `367bb2b`, `42fda84`):
   - `plugin_version_matches_cargo_version` (existing)
   - `changes_md_has_entry_for_current_version` (new)
   - Test count is now 416 (314 + 75 + 26 + 1)

4. **Plugin.json version**: Updated to `0.2.3`.
5. **CHANGES.md**: Has `[0.2.3]` entry.
6. **README.md**: Has `cargo-edit` in Optional tooling section.

### Issues Added to Plan

Three M13 milestone issues were missing from the original plan:

| Issue | Wave | Rationale |
|-------|------|-----------|
| #45 | Wave 2 | `eprintln!` cleanup fits cleanup milestone; medium effort |
| #74 | Wave 1 | Hook audit is low effort, documentary only |
| #82 | Wave 1 | Version management prevents release failures; low effort |

### Agent Scope Changes

| Agent | Original Scope | Updated Scope |
|-------|---------------|---------------|
| A | Add 5 doc-comments + `warn(missing_docs)` | `warn(missing_docs)` only + hook audit (#74) |
| C | Jaccard fix + doc path fix + `read_body()` dedup | `read_body()` dedup only + version management (#82) |
| E | `--full` flag + partial metadata fix + error handling | `--full` flag + error handling only (partial metadata already fixed) |
| F | Depends on Agent E | Independent (partial metadata already on main) |
| G-alt | (new) | Structured warning channel (#45) |

### Wave 2 Dependency Correction

The original plan stated Wave 2 agents are "independent of each other" but
then noted Agent F depends on Agent E. With the partial metadata fix already
on main, Agent F is now truly independent — it refines an existing
implementation rather than depending on Agent E to create it.

---
---

# M13: Enhancements — Work Plan (Consolidated)

Consolidated 2026-02-21. Supersedes the incremental plan above.

## Overview

Enhancements across the CLI surface and library internals: CLI naming
alignment, new `check`/`fmt`/`build`/`test` subcommands, scorer polish,
tester algorithm upgrade, upgrade robustness, YAML handling, version
management, structured warnings, and doc/hook audits.

Issues: #45, #74, #76, #78, #79, #80, #81, #82, #83, #84, #85.

Note: #75 (doc-comments) closed — already resolved on main (`lib.rs:22`
has `#![warn(missing_docs)]`, all public items documented).

## Baseline

Main at `e991b69` (M12 merged + v0.2.3 hotfixes). 416 tests (314 unit +
75 CLI + 26 plugin + 1 doc-test). ~15,500 lines across 22 source files.

Current CLI: `validate`, `lint`, `read-properties`, `to-prompt`, `build`,
`score`, `doc`, `test`, `upgrade`, `init`.

Post-M13 CLI: `validate`, `check`, `new`, `prompt`, `probe`, `score`,
`doc`, `fmt`, `build`, `test`, `upgrade`, `init`, `read-properties`.

## Branch Strategy

- **Dev branch**: `dev/m13` (from `main` after v0.2.3)
- **Task branches**: `task/m13-<name>` (from `dev/m13`)
- After each wave, task branches merge into `dev/m13`
- After all waves, PR from `dev/m13` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` to auto-close issues on merge

## Dependencies

- **M12**: All merged (PR #77). M13 modifies M12 code (`scorer.rs`,
  `tester.rs`, `main.rs`).
- **#83, #84**: Depend on Wave 3 renames (#76) to free up `build`/`test`.
- No new crate dependencies anticipated (except possibly #81).

## CLI Naming Decisions (#76)

### Renames (old → new, old preserved as hidden alias)

| Old | New | Rationale |
|-----|-----|-----------|
| `build` | `new` | Like `cargo new` — creates from scratch |
| `to-prompt` | `prompt` | Bare noun, idiomatic (`cargo doc`, `git diff`) |
| `test` | `probe` | Activation matching is probing, not testing |
| `lint` | `check` | Absorbed into `check` (see below) |

### Differentiated commands (not renames)

**`validate`** = spec conformance (syntax, schema, required fields, types).
Stays as-is. Structural, deterministic, fast.

**`check`** = superset: validate + semantic quality. Runs validate first,
then semantic analysis (description quality, trigger phrases, naming).
Absorbs `lint`. `lint` preserved as hidden alias.

- `aigent validate skills/` — conformance only
- `aigent check skills/` — validate + semantic (default: run everything)
- `aigent check skills/ --no-validate` — semantic only

**Quality spectrum**: `validate` → `check` → `score` (spec → sense → quality).

### New commands

| Command | Purpose |
|---------|---------|
| `fmt` | YAML/markdown normalization (idempotent, like `cargo fmt`) |
| `build` | Skill→plugin assembly (#83, uses freed name) |
| `test` | Fixture-based skill testing (#84, uses freed name) |

## Design Decisions

### Score Check Labels (#78)

Add `fail_label: Option<String>` to `CheckResult` with
`#[serde(skip_serializing_if = "Option::is_none")]`. `format_text()` uses
`fail_label` when `passed == false`. 11 label pairs defined (e.g.,
"Trigger phrase present" / "Trigger phrase missing").

### Tester Weighted Scoring (#79)

Replace single word-overlap with weighted formula (0.0–1.0):

```
score = 0.5 * jaccard(query, description)
      + 0.3 * trigger_match(query, triggers)
      + 0.2 * name_match(query, name)
```

Stopwords removed. `QueryMatch` thresholds: Strong ≥ 0.5, Weak ≥ 0.2,
None < 0.2. `TestResult` gains `pub score: f64`.

### Upgrade --full (#80)

Add `--full` flag: runs `validate()` + `lint()` first, applies fixes if
`--apply`, then runs upgrade pipeline. Two-section output.

### YAML AST-Preserving Parser (#81)

Improve line-level YAML manipulation for `upgrade --apply`. Handle edge
cases: comments between keys, unusual indentation, metadata blocks at
different positions. Prefer no new dependency; defer if no clean solution.

### Structured Warning Channel (#45)

Add `pub warnings: Vec<String>` to `BuildResult`. Replace the 3 LLM
fallback `eprintln!` calls in `builder/mod.rs` (lines ~88, ~107, ~135)
with `warnings.push(...)`. The remaining 10 `eprintln!` calls in
`interactive_build()` are intentional user-facing output, not warnings —
leave them as-is. CLI prints collected warnings; library consumers inspect.

### Hook Variable Audit (#74)

Audit `hooks/hooks.json` quoting against Claude Code's variable substitution
model. Documentary — add clarifying comments.

### Version Management (#82)

Create `scripts/bump-version.sh` updating `Cargo.toml`, `plugin.json`,
`CHANGES.md` stub, and `Cargo.lock` atomically. Document `cargo-edit`.

### `fmt` Subcommand (part of #76)

New `src/formatter.rs` (~200 lines). `format_skill(path) → Result<FormatResult>`.
YAML: canonical key order, consistent quoting, 2-space indent, no trailing
whitespace. Markdown: consistent list markers, single blank line between
sections, file ends with one newline. `--check` flag for CI (exit 1 if
unformatted).

### `build` Subcommand (#83)

New `src/assembler.rs` (~150 lines). Takes skill directories, generates
`plugin.json` manifest, copies skills into plugin `skills/` directory,
scaffolds empty `agents/`/`hooks/`. `--validate` flag runs plugin validation.

### `test` Subcommand (#84)

New `src/test_runner.rs` (~200 lines). Reads `tests.yml` from skill
directory, runs each query through `probe` (`test_skill()`), compares
against expected results. `--generate` creates starter fixture from skill
metadata. Exit 0 = all pass, 1 = failure, 2 = fixture error.

```yaml
queries:
  - input: "help me write a skill"
    should_match: true
    min_score: 0.5
  - input: "what's the weather"
    should_match: false
```

### README Rewrite (#85)

Comprehensive rewrite of CLI documentation: new command names, quality
spectrum, new commands, behavioral changes, hidden alias note.

## Wave Plan

### Wave 1: Low-Risk Fixes (#78, #74, #82, read_body dedup)

No API surface changes. Safe to implement independently.

**Agent A** — Hook variable audit (#74)
- Files: `hooks/hooks.json`
- Audit `$TOOL_INPUT` quoting against Claude Code variable substitution model
- Add clarifying comments to hook file

**Agent B** — Score check labels (#78)
- Files: `src/scorer.rs`, `tests/cli.rs`
- Add `fail_label` to `CheckResult`, populate for 11 checks
- Update `format_text()`, update/add tests

**Agent C** — `read_body()` dedup + version management (#82)
- Files: `src/parser.rs`, `src/scorer.rs`, `src/structure.rs`, `src/main.rs`, `README.md`
- Extract `read_body()` to `parser.rs`, update 3 call sites
- Create `scripts/bump-version.sh`

### Wave 2: Algorithm Improvements (#79, #80, #81, #45)

All agents independent.

**Agent D** — Tester weighted scoring (#79)
- Files: `src/tester.rs`, `tests/cli.rs`
- Weighted formula, stopwords, `score` field, recalibrate tests

**Agent E** — Upgrade --full (#80)
- Files: `src/main.rs`, `tests/cli.rs`
- `--full` flag, two-section output, error handling fix

**Agent F** — YAML AST-preserving parser (#81)
- Files: `src/main.rs` (or new `src/yaml_utils.rs`)
- Improve line-level manipulation edge cases, preserve comments
- Defer if no clean solution

**Agent G-alt** — Structured warning channel (#45)
- Files: `src/builder/mod.rs`
- `warnings: Vec<String>` on `BuildResult`
- Replace only the 3 LLM fallback `eprintln!` calls (not interactive output)

### Wave 3: CLI Surface Redesign (#76)

Depends on Waves 1-2 being stable.

**Agent G** — CLI renames and aliases
- Files: `src/main.rs`, `tests/cli.rs`
- `Build` → `New` (alias `build`)
- `ToPrompt` → `Prompt` (alias `to-prompt`)
- `Test` → `Probe` (alias `test`)
- Keep `Validate` as-is (spec conformance)
- Add new `Check` variant (validate + semantic, alias `lint`)
  - `--no-validate` flag to skip conformance
  - Remove standalone `Lint` variant
- Tests for all aliases and `check` superset behavior

**Agent H** — `fmt` subcommand
- Files: new `src/formatter.rs`, `src/main.rs`, `src/lib.rs`, `tests/cli.rs`
- `format_skill()`, `format_frontmatter()`, `format_body()`
- `Fmt` CLI variant with `--check`, `--recursive`
- Alias `format`

**Agent I** — Ripple effect updates
- Files: skill SKILL.md files, `hooks/hooks.json`, `tests/plugin.rs`,
  `CHANGES.md`
- Update `allowed-tools` references (`build`→`new`, `test`→`probe`,
  `to-prompt`→`prompt`, `lint`→`check`). `validate` stays as-is.
- Do NOT update historical docs

### Wave 4: New Feature Commands (#83, #84)

Depends on Wave 3 (renames must land to free up `build` and `test`).

**Agent J** — `build` subcommand: skill→plugin assembly (#83)
- Files: new `src/assembler.rs`, `src/main.rs`, `src/lib.rs`, `tests/cli.rs`
- `assemble_plugin()`, `plugin.json` generation, directory scaffolding
- `Build` CLI variant with `--output`, `--name`, `--validate`

**Agent K** — `test` subcommand: fixture-based testing (#84)
- Files: new `src/test_runner.rs`, `src/main.rs`, `src/lib.rs`, `tests/cli.rs`
- `run_test_suite()`, `tests.yml` parsing, `--generate` flag
- `Test` CLI variant with `--format`, `--recursive`, `--generate`

### Wave 5: Documentation & Verification

**Agent L-doc** — README rewrite (#85)
- Files: `README.md`
- New command reference, quality spectrum, behavioral changes
- Hidden aliases note, quick start update

**Agent L** — Full verification
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- `cargo doc --no-deps`, `cargo build --release`
- Smoke test all commands:
  - `aigent new "test skill" --no-llm`
  - `aigent build skills/aigent-builder/ --output /tmp/plugin`
  - `aigent prompt skills/`
  - `aigent to-prompt skills/` (alias)
  - `aigent probe skills/aigent-builder/ "generate a skill"`
  - `aigent test skills/aigent-builder/` (fixture suite)
  - `aigent validate skills/` (spec conformance)
  - `aigent check skills/` (validate + semantic)
  - `aigent check skills/ --no-validate` (semantic only)
  - `aigent lint skills/` (alias for `check`)
  - `aigent fmt skills/ --check`
  - `aigent score skills/aigent-builder/`
  - `aigent upgrade skills/aigent-builder/ --full`
  - `./scripts/bump-version.sh`

## Issue Summary

| Wave | Issue | Description | Complexity |
|------|-------|-------------|------------|
| 1 | #74 | Hook variable quoting audit | Low |
| 1 | #78 | Score pass/fail labels | Low |
| 1 | #82 | Version management script | Low |
| 1 | — | `read_body()` deduplication | Low |
| 2 | #79 | Weighted scoring formula | Medium |
| 2 | #80 | `upgrade --full` flag | Medium |
| 2 | #81 | YAML AST-preserving parser | Medium–High |
| 2 | #45 | Structured warning channel | Medium |
| 3 | #76 | CLI renames + new `check`/`fmt` | Medium |
| 4 | #83 | `build`: skill→plugin assembly | Medium |
| 4 | #84 | `test`: fixture-based testing | Medium |
| 5 | #85 | README rewrite | Low |
| 5 | — | Full verification | Low |

## Risk Assessment

- **#81** (highest risk): YAML comment preservation is hard. Fallback:
  improved line-level with better edge-case tests.
- **#76** (widest impact): Touches CLI, tests, skills, hooks, README.
  Wave 3 ensures stability first. `fmt` is ~200 lines of new code.
- **#79**: New scoring formula changes `QueryMatch` classifications.
  Tests must be recalibrated.
- **#83**: `plugin.json` generation must match Anthropic spec exactly.
- **#84**: `tests.yml` schema design — keep simple and extensible.

## Estimated Scope

- **New files**: `src/formatter.rs` (~200), `src/assembler.rs` (~150),
  `src/test_runner.rs` (~200), `scripts/bump-version.sh` (~30)
- **Modified files**: ~16
- **New tests**: ~30–45
- **Agents**: 13 (A–L + L-doc; Agent A reduced to hook audit only)
- **Net line delta**: +900–1200
- **New dependencies**: None anticipated (unless #81)

## Review Reconciliation (2026-02-21)

Pre-implementation checklist resolved:
- ✅ #75 closed (already done on main)
- ✅ Agent A reduced to hook audit (#74) only
- ✅ Agent G-alt scope corrected: 3 LLM fallback warnings only (not all 13 `eprintln!`)
- ✅ Issue #85 body fixed (validate/check distinction corrected)
- ✅ Issue #83 body fixed (`--multi` flag removed, implicit multi via `Vec<PathBuf>`)
- ✅ Labels added to #83 (medium), #84 (medium), #85 (low)
- ✅ Project fields already set (P2, feature/task)
- ✅ Test estimate recalibrated to 30–45 (realistic lower bound)
- Decision: `aigent lint` alias defaults to `check` behavior (superset) — documented in README
- Decision: `fmt` is target-agnostic (formats all fields present, doesn't judge validity)
- Decision: Waves 1 and 2 will run in parallel where possible
