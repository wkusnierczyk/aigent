# M12: Ecosystem & Workflow — Work Plan

## Overview

Higher-level workflow features built on the M10 diagnostic infrastructure and
M11 builder/prompt enhancements. Covers quality scoring, skill testing,
documentation generation, cross-skill analysis, watch mode, directory
validation, and skill upgrade/migration.

Issues: #54, #59, #61, #63, #64, #67, #68, #69, #70.

## Branch Strategy

- **Dev branch**: `dev/m12` (created from `main` after M11 merges)
- **Task branches**: `task/m12-<name>` (created from `dev/m12`)
- After each wave, task branches merge into `dev/m12`
- After all waves, PR from `dev/m12` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- **M10**: `Diagnostic`, `Severity`, `linter::lint()`, `ValidationTarget`,
  `--format`, `--recursive` — all M12 features consume these
- **M11**: `SkillEntry`, multi-format prompt, `template_files()`, token
  budget estimation, `apply_fixes()` — M12 builds on these
- M7: `src/builder/` — `SkillSpec`, `BuildResult`
- M9: `skills/`, `.claude-plugin/plugin.json` — plugin packaging

## Current State (after M11)

M11 adds:

- `src/prompt.rs`: `SkillEntry`, multi-format output, `estimate_tokens()`
- `src/builder/templates.rs`: 6 template variants
- `hooks/hooks.json`: PostToolUse hook for continuous validation
- `install.sh`: checksum verification
- `--interactive` build mode, `--output` for to-prompt

M12 introduces the ecosystem layer — tools for managing skill collections,
assessing quality at scale, and maintaining skills over time.

---

## Design Decisions

### Quality Scoring (#59)

A `score` subcommand that runs the Anthropic best-practices checklist:

```rust
Score {
    skill_dir: PathBuf,
    #[arg(long, value_enum, default_value = "text")]
    format: OutputFormat,
}
```

Scoring logic in `src/scorer.rs`:

- Run validation (structural checks)
- Run linting (semantic checks)
- Weight: structural pass = 60 points base, each lint pass = +8 points
  (5 checks = 40 max)
- Return score 0–100 with breakdown

Output:

```
Score: 76/100

Structural (60/60):
  [PASS] Name format
  [PASS] Description length
  ...

Quality (16/40):
  [PASS] Third-person description
  [FAIL] Missing trigger phrase
  [PASS] Gerund name form
  [FAIL] Description too vague
  [PASS] Specific name
```

JSON output when `--format json`.

### Skill Tester (#64)

The most ambitious item. The spec describes "evaluation-driven development"
but notes no built-in tooling exists. A `test` subcommand simulates skill
discovery given a test query:

1. Load all skills from specified directories
2. For each test query, rank skills by description relevance (keyword match
   + trigger phrase detection)
3. Report which skills would activate, potential conflicts, confidence scores

This addresses observation #3 from the review: evaluation-driven development
is the spec's recommended workflow but no tooling supports it.

Implementation approach:

- Simple keyword matching (no embedding model dependency)
- Score based on: query term overlap with description, trigger phrase match,
  name relevance
- Output: ranked list per query with activation confidence

### Scorer Skill (#63)

New skill at `skills/aigent-scorer/SKILL.md`:

```yaml
---
name: aigent-scorer
description: >-
  Scores AI agent skill definitions against the Anthropic best-practices
  checklist. Provides quality ratings and improvement suggestions. Use when
  reviewing skill quality, improving existing skills, or preparing skills
  for sharing.
allowed-tools: Bash(aigent *), Bash(command -v *), Read, Glob
argument-hint: "[skill-directory-or-file]"
---
```

Hybrid mode: CLI `aigent score` when available; prompt-only checklist
otherwise. Follows the pattern established in M9.

### Cross-Skill Conflict Detection (#69)

Depends on batch validation and token budget from M10/M11. For skill
collections:

- Detect description similarity (word overlap ratio)
- Flag potential activation conflicts (two skills triggering on same query)
- Estimate total token budget
- Check for name collisions across scopes

Implementation in `src/conflict.rs`:

```rust
pub fn detect_conflicts(entries: &[SkillEntry]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    diags.extend(check_name_collisions(entries));
    diags.extend(check_description_similarity(entries));
    diags.extend(check_token_budget(entries));
    diags
}
```

### Directory Structure Validation (#54)

Extends the validator to check skill package structure:

- Referenced files actually exist (broken link detection via markdown
  link/image parsing in the body)
- Scripts have execute permissions
- References stay one level deep (spec recommendation)
- No deeply nested directory trees

Implementation: additional checks in `src/validator.rs` or a new
`src/structure.rs` module, returning `Vec<Diagnostic>` with new codes
(S001–S004).

### Skill Upgrade / Migration (#61)

An `upgrade` subcommand that reads an existing SKILL.md, identifies areas
not following current best practices, and applies fixes:

1. Run validation + linting
2. Identify fixable issues (reusing `src/fixer.rs` from M10)
3. Identify quality improvements (missing fields, outdated patterns)
4. Report with `--dry-run` (default) or apply with `--apply`

Difference from `validate --apply-fixes`: upgrade also adds missing
recommended fields (e.g., `compatibility`, trigger phrases) and restructures
content following current spec patterns.

### Documentation Generation (#67)

A `doc` subcommand generating markdown catalogs from skill collections:

```rust
Doc {
    /// Paths to skill directories
    dirs: Vec<PathBuf>,
    /// Output file (default: stdout)
    #[arg(long)]
    output: Option<PathBuf>,
    /// Recursive discovery
    #[arg(long)]
    recursive: bool,
}
```

Uses `SkillEntry` from M11's prompt refactor. Output:

```markdown
# Skill Catalog

## aigent-builder
> Generates AI agent skill definitions...

**Compatibility**: Claude Code
**Location**: `skills/aigent-builder/SKILL.md`

---

## aigent-validator
> Validates AI agent skill definitions...
```

### Watch Mode (#68)

A `--watch` flag on `validate` using the `notify` crate:

1. Run initial validation
2. Watch for filesystem changes in the skill directory
3. Re-run validation on change (debounced, 500ms)
4. Clear terminal between runs

### README Improvements (#70)

Update the project README with:

- Updated CLI surface showing all new subcommands and flags
- Feature matrix (M10/M11/M12 capabilities)
- Template catalog with examples
- Quick-start guide for common workflows
- Badge updates (version, CI, license)

Scope TBD based on user input — this issue tracks the work but specific
content will be determined during implementation.

---

## Wave 1 — Quality & Scoring

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m12-scorer` | #59 | Add `score` subcommand with quality scoring |
| B | `task/m12-scorer-skill` | #63 | Add aigent-scorer plugin skill |
| C | `task/m12-dir-validate` | #54 | Directory structure validation |

**Merge**: A first, then B (B's skill uses `aigent score`). C is independent.
Merge order: A → `dev/m12`, C → `dev/m12`, B → `dev/m12`.

### Agent A — Quality Scoring (#59)

1. Create `src/scorer.rs`:
   - `pub fn score(dir: &Path) -> ScoreResult`
   - `ScoreResult`: total score (0–100), structural breakdown, quality breakdown
   - Runs `validate()` + `lint()` internally
   - Weighted scoring: structural = 60 base, lint = 8 per check

2. Update `src/lib.rs`:
   - Add `pub mod scorer;`
   - Re-export `score`, `ScoreResult`

3. Update `src/main.rs`:
   - Add `Score` subcommand
   - Text output: table with [PASS]/[FAIL] per check
   - JSON output: structured `ScoreResult`

4. Tests:
   - Perfect skill scores 100/100
   - Skill with lint issues scores < 100
   - Skill with structural errors scores ≤ 60
   - JSON output is valid and matches text semantics

### Agent B — Scorer Skill (#63)

1. Create `skills/aigent-scorer/SKILL.md`:
   - Hybrid mode (CLI `aigent score` / prompt-only checklist)
   - Embeds the Anthropic best-practices checklist for prompt-only mode
2. Self-validation: `aigent validate skills/aigent-scorer/`
3. Test in `tests/plugin.rs`: scorer skill passes validation

### Agent C — Directory Structure Validation (#54)

1. Add structure validation in `src/validator.rs` or `src/structure.rs`:
   - S001: Referenced file does not exist
   - S002: Script missing execute permission
   - S003: Reference depth exceeds 1 level
   - S004: Excessive nesting depth

2. Enable with `--structure` flag on `validate`

3. Markdown link/image extraction: regex for `[text](path)` and `![alt](path)`

4. Tests:
   - Skill with broken file reference → S001
   - Script without +x → S002
   - Deeply nested reference → S003
   - Correct structure → no diagnostics

---

## Wave 2 — Ecosystem Features

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| D | `task/m12-conflict` | #69 | Cross-skill conflict detection |
| E | `task/m12-doc` | #67 | Documentation generation |
| F | `task/m12-watch` | #68 | Watch mode for validation |

**Merge**: D, E, F can all run in parallel. Merge order: any.

### Agent D — Cross-Skill Conflict Detection (#69)

1. Create `src/conflict.rs`:
   - `pub fn detect_conflicts(entries: &[SkillEntry]) -> Vec<Diagnostic>`
   - Name collision check (same name in different directories)
   - Description similarity (word overlap ratio > 0.7 threshold)
   - Token budget total (warning if > 4000 tokens)

2. Integrate with batch validation:
   - When `--recursive` or multiple dirs, run conflict detection
   - Conflict diagnostics use new codes: C001 (name collision),
     C002 (description overlap), C003 (token budget exceeded)

3. Tests:
   - Two skills with same name → C001
   - Two skills with similar descriptions → C002
   - Large collection exceeding budget → C003
   - No false positives on distinct skills

### Agent E — Documentation Generation (#67)

1. Add `Doc` subcommand to `src/main.rs`
2. Use `collect_skills()` from M11's prompt refactor
3. Generate markdown catalog with name, description, compatibility, location
4. Support `--output <file>` and `--recursive`

5. Tests:
   - Correct markdown structure
   - All skills included
   - Works with `--recursive`
   - `--output` writes to file

### Agent F — Watch Mode (#68)

1. Add `notify = "8"` dependency to `Cargo.toml`
2. Add `--watch` flag to `Validate` subcommand
3. Watch for file changes in specified directories
4. Re-run validation on change (debounced 500ms)
5. Clear terminal between runs (`\x1b[2J\x1b[H`)

6. Tests:
   - Watch flag accepted without error
   - (Integration test: write file, verify re-validation — may be flaky,
     consider skipping in CI)

---

## Wave 3 — Tester + Upgrade + README

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| G | `task/m12-tester` | #64 | Skill tester and previewer |
| H | `task/m12-upgrade` | #61 | Skill upgrade/migration command |
| I | `task/m12-readme` | #70 | README improvements |

**Merge**: G, H, I can all run in parallel. Merge order: any.

### Agent G — Skill Tester (#64)

1. Create `src/tester.rs`:
   - `pub fn test_queries(entries: &[SkillEntry], queries: &[&str]) -> Vec<TestResult>`
   - `TestResult`: query, ranked skills with scores, potential conflicts

2. Scoring algorithm:
   - Tokenize query and description
   - Score = weighted sum of: keyword overlap, trigger phrase match,
     name relevance
   - Normalize to 0.0–1.0 confidence

3. Add `Test` subcommand:
   ```rust
   Test {
       /// Skill directories
       skill_dirs: Vec<PathBuf>,
       /// Test queries
       #[arg(long, required = true)]
       query: Vec<String>,
       /// Recursive discovery
       #[arg(long)]
       recursive: bool,
   }
   ```

4. Output:
   ```
   Query: "process pdf files"
     1. processing-pdfs      0.85  ← likely match
     2. document-converter   0.42
     3. file-utils           0.21

   Query: "write unit tests"
     1. test-generator       0.91  ← likely match
     ⚠ No close second — good discrimination
   ```

5. Tests:
   - Exact name match scores highest
   - Description keyword overlap increases score
   - Unrelated queries score low
   - Multiple skills with similar descriptions flagged

### Agent H — Skill Upgrade (#61)

1. Add `Upgrade` subcommand:
   ```rust
   Upgrade {
       skill_dir: PathBuf,
       #[arg(long)]
       apply: bool,   // default: dry-run
   }
   ```

2. Upgrade logic:
   - Run `validate()` + `lint()` to identify issues
   - Check for missing recommended fields (`compatibility`)
   - Check for missing trigger phrase in description
   - If `--apply`: use `fixer::apply_fixes()` + add missing fields
   - If dry-run: report what would change

3. Tests:
   - Dry-run reports issues without modifying file
   - `--apply` fixes issues and modifies file
   - Already-good skill reports "no upgrades needed"

### Agent I — README Improvements (#70)

1. Update `README.md`:
   - Full CLI surface documentation
   - Feature matrix
   - Template catalog
   - Quick-start guide
   - Updated badges

2. Scope may be adjusted based on user input during implementation

---

## Wave 4 — Verify

Single agent runs the full check suite on `dev/m12`.

| Agent | Branch | Task |
|-------|--------|------|
| J | `dev/m12` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Cargo.toml Changes

```toml
[dependencies]
# existing...
notify = "8"       # Watch mode (#68)
```

No other new dependencies required.

---

## New Module Map

After M12, changes from M11 baseline:

```
src/
├── scorer.rs           # NEW: Quality scoring (0–100)
├── tester.rs           # NEW: Skill discovery simulation
├── conflict.rs         # NEW: Cross-skill conflict detection
├── structure.rs        # NEW: Directory structure validation (or in validator.rs)
└── main.rs             # UPDATED: new subcommands (score, test, doc, upgrade)

skills/
└── aigent-scorer/
    └── SKILL.md        # NEW: Quality assessment skill
```

---

## New CLI Surface

After M12 (additions to M11 surface):

```
aigent validate <dirs...> [--recursive] [--lint] [--structure]
                          [--target standard|claude-code|permissive]
                          [--format text|json] [--apply-fixes] [--watch]
aigent lint <skill-dir>
aigent score <skill-dir> [--format text|json]
aigent test <dirs...> --query <query> [--recursive]
aigent upgrade <skill-dir> [--apply]
aigent doc <dirs...> [--output <file>] [--recursive]
aigent read-properties <skill-dir>
aigent to-prompt <dirs...> [--format xml|json|yaml|markdown] [--budget] [--output <file>]
aigent build <purpose> [--name] [--dir] [--no-llm] [--interactive] [--template <template>]
aigent init [dir] [--template <template>]
aigent --about
```

---

## Deliverables

- `src/scorer.rs` — quality scoring with breakdown
- `src/tester.rs` — skill discovery simulation
- `src/conflict.rs` — cross-skill conflict detection
- `src/structure.rs` — directory structure validation
- `skills/aigent-scorer/SKILL.md` — quality assessment plugin skill
- `src/main.rs` — new subcommands (`score`, `test`, `doc`, `upgrade`),
  new flags (`--structure`, `--watch`, `--query`, `--apply`)
- `README.md` — comprehensive update
- Cargo.toml — `notify` dependency for watch mode
- Updated tests across all modules
- PR: `M12: Ecosystem & Workflow`

---

## Reconciliation (2026-02-20)

### M11 Absorbed Into M10

The M10 PR (#72, merged as `2c11167`) implemented **all 8 M11 issues** ahead
of schedule. The following M11 deliverables are already on `main`:

| M11 Issue | Deliverable | Location |
|-----------|-------------|----------|
| #58 Templates | 6 `SkillTemplate` variants | `src/builder/template.rs` |
| #55 Token budget | `estimate_tokens()`, `format_budget()` | `src/prompt.rs` |
| #56 Multi-format | `PromptFormat`, `to_prompt_format()` | `src/prompt.rs` |
| #57 Diff-aware | `--output` flag with change detection | `src/main.rs` |
| #60 Interactive | `interactive_build()` with `BufRead` | `src/builder/mod.rs` |
| #62 Hooks | `PostToolUse` hook for Write\|Edit | `hooks/hooks.json` |
| #65 context:fork | `context: fork` in builder skill | `skills/aigent-builder/SKILL.md` |
| #49 Checksum | SHA256 verification | `install.sh`, `release.yml` |

**Consequence for M12**: The plan's §Current State (after M11) is now the
**actual current state**. All M11 dependencies that M12 relies on are
available immediately:

- `SkillEntry` struct → used by `scorer.rs`, `tester.rs`, `conflict.rs`, `doc`
- `collect_skills()` → used by `doc` subcommand
- `estimate_tokens()` → used by conflict detection token budget check
- `apply_fixes()` → used by `upgrade` subcommand
- `SkillTemplate` → no M12 dependency, but confirms template infrastructure

### Stale Assumptions

1. Plan §Dependencies says M12 depends on M11 merging first — **no longer
   required**. M12 can branch from current `main` directly.
2. Plan §Branch Strategy says `dev/m12` created "after M11 merges" — M11 work
   is already on `main`, so `dev/m12` can be created immediately.

### M11 Reconciliation Findings Inherited

The M11 reconciliation section identified these deferred items relevant to M12:

| Finding | M12 Coverage | Resolution |
|---------|-------------|------------|
| F3: `--apply-fixes` dry-run | Agent H (#61, upgrade) | **Address**: `upgrade --apply` vs dry-run default covers this |
| F5: Hook `jq` dependency | Already resolved in M10 PR | **No action needed** |
| F8: `template.rs` kept as filename | N/A for M12 | **No action needed** |

### Additional M10 Artifacts Available

The M10 PR also delivered items not in the original M10 plan that M12 can use:

- `src/builder/util.rs`: `to_title_case()` utility — reusable in doc generation
- `read_body()` helper in `main.rs` — pattern for reading SKILL.md body
- `resolve_dirs()` with `--recursive` — reusable for `doc`, `test`, `conflict`
- Consistent `Format` + `PromptOutputFormat` enum pattern in `main.rs` —
  follow same pattern for `Score`, `Test`, `Doc` output formats

### Ordering Adjustments

No ordering changes needed. The wave structure remains valid:

1. **Wave 1** (Quality & Scoring): `scorer.rs` + scorer skill + dir validation
   — no cross-dependencies with M11
2. **Wave 2** (Ecosystem): conflict detection + doc gen + watch mode — all
   consume `SkillEntry`/`collect_skills()` which are now available
3. **Wave 3** (Tester + Upgrade + README): highest-level features
4. **Wave 4** (Verify): unchanged

### Implementation Notes

1. **`lib.rs` re-exports**: Add `score`, `ScoreResult`, `detect_conflicts`,
   `test_queries`, `TestResult` as they are implemented.
2. **Error code registry**: New diagnostic code ranges for M12:
   - S001–S004 (structure validation, Wave 1)
   - C001–C003 (conflict detection, Wave 2)
   - Follow the pattern in `src/diagnostics.rs` constants.
3. **`--format` pattern**: Reuse the `Format` enum (text/json) from `main.rs`
   for `score`, `test`, and `doc` subcommands.
4. **`notify` crate version**: Verified — `notify = "8"` resolves to 8.2.0
   (latest stable as of 2026-02-20). Gate behind feature flag per review F6.

---

## Pre-Implementation Reconciliation (2026-02-20, session 2)

### Baseline

Main at `7cd1aa7` (M11 merged via squash). 332 tests (256 unit + 54 CLI +
21 plugin + 1 doc). All M11 review fixes included in squash merge.

### M11 Review Findings — Resolved

All M11 findings that the M12 review (F11) flagged are resolved:

- F1 (build --template): Removed from `build` subcommand
- F5 (interactive deterministic): Documented
- F6 (--output overwrites): Fixed to write-only-on-change

Agent E (`doc --output`) should follow the same write-only-on-change pattern.
Agent H (upgrade) is safe — no `build_skill()` with templates.

### Review Findings — Implementation Decisions

| Finding | Decision |
|---------|----------|
| F2 (tester formula) | Use `score = 0.5*jaccard(query,desc) + 0.3*trigger_match + 0.2*name_match` |
| F3 (S002 platform) | Gate behind `#[cfg(unix)]`, document as Unix-only |
| F4 (upgrade overlap) | `upgrade` calls `validate --apply-fixes` internally + adds recommended fields |
| F5 (0.7 threshold) | Document as heuristic, add `--similarity-threshold` flag |
| F6 (notify weight) | Gate behind `[features] watch = ["notify"]` |
| F7 (scorer tools) | Use `Bash(aigent score *), Bash(aigent validate *)` |
| F8 (code registry) | Define S001-S004, C001-C003 in `diagnostics.rs` |
| F9 (doc format) | Omit missing fields, sort alphabetically |
| F10 (integration tests) | Add cross-module smoke tests in Wave 4 |

### CLI Surface Correction

Plan shows `build` with `--template` — removed in M11 review. Corrected surface:

```
aigent build <purpose> [--name] [--dir] [--no-llm] [--interactive]
```
