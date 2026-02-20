# M10: Improvements and Extensions — Work Plan

## Overview

Product improvements and extensions beyond spec compliance. Organized into
five waves around dependency order and risk:

1. **Foundation** — Structured diagnostics (X1) and Claude Code field awareness
   (V2), which other features depend on
2. **Validator Power** — Semantic linting (V1), batch validation (V3), fix-it
   suggestions (V4)
3. **Builder & Prompt** — Template system (B1), token budget estimation (P1),
   multi-format output (P2)
4. **Plugin Depth** — Hooks (PL1), scorer skill (PL2), context:fork (PL4)
5. **Polish** — Interactive build (B3), quality assessment (B2), skill tester
   (PL3), remaining items

Issues: #49, #50, #51, #52, #53, #54, #55, #56, #57, #58, #59, #60, #61,
#62, #63, #64, #65, #66, #67, #68, #69, #70.

## Branch Strategy

- **Dev branch**: `dev/m10` (created from `main`)
- **Task branches**: `task/m10-<name>` (created from `dev/m10`)
- After each wave, task branches merge into `dev/m10`
- After all waves, PR from `dev/m10` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- M4/M6: `validate()` in `src/validator.rs` — the refactoring target
- M7: `src/builder/` — deterministic + LLM modules, `SkillSpec`, `BuildResult`
- M9: `skills/`, `.claude-plugin/plugin.json`, `install.sh` — plugin packaging
- M3: `src/parser.rs` — `KNOWN_KEYS`, `parse_frontmatter`, `read_properties`
- M5: `src/prompt.rs` — `to_prompt`, `xml_escape`
- M2: `src/errors.rs` — `AigentError`, `Result<T>` alias

## Current State

All M1–M9 milestones completed. The codebase has:

- `src/validator.rs`: 30+ validation rules, returns `Vec<String>` with
  ad-hoc "warning:" prefix for warnings
- `src/prompt.rs`: XML-only output, no budget estimation
- `src/builder/`: deterministic + LLM modes, 4 providers, template for `init`
  (single minimal template)
- `src/main.rs`: 5 subcommands (`validate`, `read-properties`, `to-prompt`,
  `build`, `init`), no `--format`, `--lint`, `--recursive`, or `--target` flags
- `src/errors.rs`: `AigentError` enum with `Parse`, `Validation`, `Io`,
  `Yaml`, `Build` variants; `Validation` stores `Vec<String>`
- `skills/`: two skills (`aigent-builder`, `aigent-validator`)
- `.claude-plugin/plugin.json`: plugin manifest
- `install.sh`: download + install script (no checksum verification)

Key refactoring surface: `validate()` returns `Vec<String>` — this permeates
the entire codebase. Wave 1 replaces it with a structured `Diagnostic` type,
which is the prerequisite for semantic linting (V1), fix-it suggestions (V4),
batch validation (V3), and JSON output.

---

## Design Decisions

### Structured Diagnostics (X1) — The Foundation

The most impactful cross-cutting change. Every other improvement benefits from
structured error output. The current `Vec<String>` approach has three problems:

1. No machine-readable output — CI scripts must parse human text
2. No stable identifiers — error messages are the only API contract
3. No fix suggestions — errors are descriptive but not actionable

#### New Types in `src/diagnostics.rs`

```rust
/// Severity of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A structured diagnostic message from validation or linting.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: Severity,
    /// Stable error code (e.g., "E001", "W001", "I001").
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Field that caused the diagnostic (e.g., "name", "description").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<&'static str>,
    /// Suggested fix (actionable text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}
```

The `Span` field (line/column) is intentionally deferred — it requires
propagating source positions through the parser, which is a larger refactor
better suited for an LSP integration milestone. The current design covers all
Wave 1–5 needs without it.

#### Error Code Registry

Codes are organized by component and severity:

| Code | Severity | Field | Message |
|------|----------|-------|---------|
| E001 | Error | name | name must not be empty |
| E002 | Error | name | name exceeds 64 characters |
| E003 | Error | name | name contains invalid character: '{c}' |
| E004 | Error | name | name starts with hyphen |
| E005 | Error | name | name ends with hyphen |
| E006 | Error | name | name contains consecutive hyphens |
| E007 | Error | name | name contains reserved word: '{word}' |
| E008 | Error | name | name contains XML tags |
| E009 | Error | name | name does not match directory name |
| E010 | Error | description | description must not be empty |
| E011 | Error | description | description exceeds 1024 characters |
| E012 | Error | description | description contains XML tags |
| E013 | Error | compatibility | compatibility exceeds 500 characters |
| W001 | Warning | metadata | unexpected metadata field: '{key}' |
| W002 | Warning | body | body exceeds 500 lines ({n} lines) |

Lint codes (added in Wave 2):

| Code | Severity | Field | Message |
|------|----------|-------|---------|
| I001 | Info | description | description uses first/second person |
| I002 | Info | description | description lacks trigger phrase ("Use when...") |
| I003 | Info | name | name does not use gerund form |
| I004 | Info | name | name is overly generic |
| I005 | Info | description | description is overly vague |

Codes are stable — removing a code is a breaking change. New codes are added
at the end. The registry lives as constants in `src/diagnostics.rs`.

#### Migration Strategy

1. Create `src/diagnostics.rs` with `Severity`, `Diagnostic`, and the code
   registry
2. Change `validate()` return type: `Vec<String>` → `Vec<Diagnostic>`
3. Change `validate_metadata()` return type: `Vec<String>` → `Vec<Diagnostic>`
4. Add `Diagnostic::is_error()`, `Diagnostic::is_warning()` helpers
5. Add `impl fmt::Display for Diagnostic` matching current text output format
   (backward-compatible human output)
6. Update `src/main.rs` to use `Diagnostic` — `is_error()` replaces
   `!m.starts_with("warning: ")`
7. Update `src/builder/mod.rs` — `build_skill` validation check uses
   `d.is_error()` instead of string prefix matching
8. Update `AigentError::Validation` to store `Vec<Diagnostic>` instead of
   `Vec<String>` — update `format_validation_errors` accordingly
9. Update all tests

The key constraint: the text output of `eprintln!("{d}")` must produce the
same strings as today, so existing users and CI scripts see no behavioral
change. Only the internal representation changes.

#### JSON Output

Add `--format` flag to `validate` subcommand:

```rust
/// Output format
#[arg(long, value_enum, default_value = "text")]
format: OutputFormat,

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}
```

When `--format json`, output `serde_json::to_string_pretty(&diagnostics)`.

### Claude Code Extension Fields (V2)

Add a `--target` flag to `validate`:

```rust
/// Validation target profile
#[arg(long, value_enum, default_value = "standard")]
target: ValidationTarget,

#[derive(Clone, ValueEnum)]
enum ValidationTarget {
    Standard,
    ClaudeCode,
    Permissive,
}
```

Implementation: extend `KNOWN_KEYS` into a tiered system:

```rust
/// Open standard fields (always recognized).
pub const STANDARD_KEYS: &[&str] = &[
    "name", "description", "license", "compatibility", "allowed-tools",
];

/// Claude Code extension fields (recognized with --target claude-code).
pub const CLAUDE_CODE_KEYS: &[&str] = &[
    "disable-model-invocation", "user-invocable", "context", "agent",
    "model", "hooks", "argument-hint",
];

/// Returns the set of known keys for the given target.
pub fn known_keys_for(target: ValidationTarget) -> Vec<&'static str> {
    match target {
        ValidationTarget::Standard => STANDARD_KEYS.to_vec(),
        ValidationTarget::ClaudeCode => {
            let mut keys = STANDARD_KEYS.to_vec();
            keys.extend_from_slice(CLAUDE_CODE_KEYS);
            keys
        }
        ValidationTarget::Permissive => vec![], // no warnings for unknown fields
    }
}
```

The `validate()` function gains an optional `target` parameter. To maintain
backward compatibility, the public API uses a default:

```rust
pub fn validate(dir: &Path) -> Vec<Diagnostic> {
    validate_with_target(dir, ValidationTarget::Standard)
}

pub fn validate_with_target(dir: &Path, target: ValidationTarget) -> Vec<Diagnostic> {
    // ...
}
```

### Semantic Linting (V1)

Lint checks are Info-level diagnostics — they never cause validation failure.
They are emitted alongside structural validation when `--lint` is passed, or
via a standalone `lint` subcommand.

Implementation lives in a new `src/linter.rs` module:

```rust
/// Run semantic lint checks on parsed skill properties and body.
pub fn lint(properties: &SkillProperties, body: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    diags.extend(lint_description_person(&properties.description));
    diags.extend(lint_description_trigger(&properties.description));
    diags.extend(lint_name_gerund(&properties.name));
    diags.extend(lint_name_generic(&properties.name));
    diags.extend(lint_description_vague(&properties.description));
    diags
}
```

**Lint checks:**

1. **First/second person** (I001): Regex for `\b(I|me|my|you|your)\b`
   (case-insensitive) at word boundaries. Suggestion: "Rewrite description in
   third person — e.g., 'Processes PDFs' not 'I process PDFs'."

2. **Missing trigger phrase** (I002): Check if description contains any of:
   "use when", "use for", "use this", "invoke when", "activate when"
   (case-insensitive). Suggestion: "Add a trigger phrase — e.g., 'Use when
   working with PDF files.'"

3. **Non-gerund name** (I003): First segment of hyphen-split name; check if
   it ends in "ing". Suggestion: "Consider gerund form —
   e.g., 'processing-pdfs' instead of 'pdf-processor'."

4. **Generic name** (I004): Check first segment against a blocklist:
   `["helper", "utils", "tools", "stuff", "thing", "misc", "general"]`.
   Suggestion: "Use a specific, descriptive name."

5. **Vague description** (I005): Description has fewer than 20 characters, or
   word count fewer than 4. Suggestion: "Add detail about what the skill does
   and when to use it."

### Batch Validation (V3)

Extend the `Validate` subcommand:

```rust
Validate {
    /// Paths to skill directories or SKILL.md files
    skill_dirs: Vec<PathBuf>,    // changed from single PathBuf
    /// Recursively discover skills under each path
    #[arg(long)]
    recursive: bool,
    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    format: OutputFormat,
    /// Validation target profile
    #[arg(long, value_enum, default_value = "standard")]
    target: ValidationTarget,
    /// Run semantic lint checks
    #[arg(long)]
    lint: bool,
}
```

**Recursive discovery**: Walk directories, find all `SKILL.md` files, resolve
each to its parent directory. Uses `std::fs::read_dir` recursively — no new
dependency needed (`walkdir` is overkill for this use case).

**Summary output** (text mode):

```
skills/aigent-builder/   PASS
skills/aigent-validator/  PASS  (1 info)
skills/broken-skill/     FAIL  (2 errors, 1 warning)
---
3 skills checked: 2 passed, 1 failed
```

**JSON mode**: Array of `{ "path": "...", "diagnostics": [...] }` objects.

**Exit code**: Non-zero if any skill has errors (same as current behavior, but
across the batch).

### Fix-It Suggestions (V4)

Requires structured diagnostics (X1) — suggestions are stored in
`Diagnostic.suggestion`. This issue adds:

1. **Suggestion text** in each diagnostic where a fix is possible (populated
   during validation)
2. An `--apply-fixes` flag on `validate` that reads SKILL.md, applies fixable
   changes, and writes back

**Fixable issues:**

| Code | Fix |
|------|-----|
| E002 | Truncate name at hyphen boundary to ≤ 64 chars |
| E003 (uppercase) | Lowercase the name |
| E006 | Collapse consecutive hyphens |
| E012 | Strip XML tags from description |
| I002 | Append "Use when this capability is needed." |

**Non-fixable issues** (require human judgment):

| Code | Reason |
|------|--------|
| E001 | Can't invent a name |
| E007 | Reserved word removal might break semantics |
| E009 | Directory rename needed — out of scope |
| I001 | Person rewrite requires understanding intent |

The `--apply-fixes` handler:
1. Run validation
2. Collect fixable diagnostics
3. Parse frontmatter + body
4. Apply fixes to the parsed data
5. Re-serialize and write back
6. Re-validate to confirm fixes resolved the issues

### Template System (B1)

Extend `init` and `build` with a `--template` flag:

```rust
#[derive(Clone, ValueEnum)]
enum SkillTemplate {
    Minimal,
    ReferenceGuide,
    DomainSpecific,
    Workflow,
    CodeSkill,
    ClaudeCode,
}
```

Each template generates a different directory structure:

**`minimal`** (current, default):
```
<name>/
└── SKILL.md
```

**`reference-guide`**:
```
<name>/
├── SKILL.md          (overview + links to references)
├── REFERENCE.md      (detailed technical reference)
└── EXAMPLES.md       (usage examples)
```

**`domain-specific`**:
```
<name>/
├── SKILL.md          (overview + navigation)
└── reference/
    └── domain.md     (placeholder domain reference)
```

**`workflow`**:
```
<name>/
└── SKILL.md          (includes checklist pattern from spec)
```

**`code-skill`**:
```
<name>/
├── SKILL.md          (instructions referencing scripts)
└── scripts/
    └── run.sh        (starter script with error handling)
```

**`claude-code`**:
```
<name>/
└── SKILL.md          (includes Claude Code extension fields)
```

Templates live in `src/builder/templates/` as functions returning
`HashMap<String, String>` (relative path → content). The `init_skill` and
`build_skill` functions accept an optional `SkillTemplate` parameter.

### Token Budget Estimation (P1)

Add `--budget` flag to `to-prompt`:

```rust
ToPrompt {
    skill_dirs: Vec<PathBuf>,
    /// Show estimated token budget
    #[arg(long)]
    budget: bool,
    /// Output format
    #[arg(long, value_enum, default_value = "xml")]
    format: PromptFormat,
}
```

**Estimation heuristic**: `chars / 4` (standard English approximation). No
external dependency — `tiktoken-rs` is heavy and unnecessary for estimates.

**Budget output** (appended after the prompt):

```
Token budget:
  aigent-builder     ~45 tokens
  aigent-validator   ~48 tokens
  ---
  Total:             ~93 tokens
  Context usage:     <0.1% of 200k
```

**Threshold warning**: If total exceeds 4000 tokens (~2% of 200k context),
emit a warning suggesting skill consolidation.

### Multi-Format Prompt Output (P2)

Add `--format` flag to `to-prompt`:

```rust
#[derive(Clone, ValueEnum)]
enum PromptFormat {
    Xml,
    Json,
    Yaml,
    Markdown,
}
```

**JSON format**:
```json
[
  {
    "name": "aigent-builder",
    "description": "...",
    "location": "/abs/path/to/SKILL.md"
  }
]
```

**YAML format**:
```yaml
- name: aigent-builder
  description: "..."
  location: /abs/path/to/SKILL.md
```

**Markdown format**:
```markdown
# Available Skills

## aigent-builder
> Generates AI agent skill definitions...

**Location**: `/abs/path/to/SKILL.md`
```

Implementation: the `to_prompt` function is refactored to first collect a
`Vec<SkillEntry>` struct, then format based on the chosen output format.

```rust
struct SkillEntry {
    name: String,
    description: String,
    location: String,
}
```

### Hooks for Continuous Validation (PL1)

Add `hooks/hooks.json` to the plugin:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "file=\"$(echo '$TOOL_INPUT' | jq -r '.file_path // .file_path // empty')\" && [ -n \"$file\" ] && echo \"$file\" | grep -q 'SKILL\\.md$' && command -v aigent >/dev/null 2>&1 && aigent validate \"$(dirname \"$file\")\" 2>&1 || true"
          }
        ]
      }
    ]
  }
}
```

The hook:
1. Extracts the file path from the tool input JSON
2. Checks if the file is a SKILL.md
3. If `aigent` is available, runs validation on the parent directory
4. Always succeeds (trailing `|| true`) — validation failures are informational

### Scorer Skill (PL2)

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

Body follows the hybrid pattern from M9 — CLI mode uses `aigent validate
--lint` for structural + semantic checks; prompt-only mode embeds the quality
checklist from the Anthropic best-practices document.

### context:fork for Builder (PL4)

Update `skills/aigent-builder/SKILL.md` frontmatter:

```yaml
context: fork
```

This enables the builder to explore the codebase before generating, avoiding
conflicts with existing skills.

### Interactive Build Mode (B3)

Add `--interactive` flag to `build`:

```rust
Build {
    purpose: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    dir: Option<PathBuf>,
    #[arg(long)]
    no_llm: bool,
    /// Interactive mode with preview and confirmation
    #[arg(long, short)]
    interactive: bool,
    /// Skill template
    #[arg(long, value_enum, default_value = "minimal")]
    template: SkillTemplate,
}
```

Interactive mode flow:
1. Assess clarity — if unclear, print questions and exit (user re-runs with
   more detail)
2. Derive name — print "Name: {name}" and confirm
3. Generate description — print and confirm
4. Generate body preview — print first 20 lines
5. Confirm write
6. Validate and report

This surfaces the existing `ClarityAssessment` struct, which currently has
no user-facing CLI exposure.

### Remaining Items

**B2: Quality Assessment** (#59) — A `score` subcommand that runs the
best-practices checklist. Depends on V1 (semantic linting) for the checks
and X1 (structured diagnostics) for the output format. Produces a summary
score (0–100) based on weighted checks.

**B4: Skill Upgrade** (#61) — An `upgrade` subcommand. Depends on V4 (fix-it
suggestions) for the fixing logic and V1 (semantic linting) for detecting
quality issues. Reads an existing SKILL.md, identifies areas not following
current best practices, and reports or applies fixes.

**V5: Directory Structure Validation** (#54) — Extends the validator to check
file references, script permissions, and nesting depth. Depends on X1
(structured diagnostics) for structured output.

**P3: Diff-Aware Prompt Updates** (#57) — Adds `--output <file>` flag.
Minimal scope — can be implemented standalone once P2 refactors `to_prompt`
internals.

**PL3: Skill Tester** (#64) — Most ambitious item. Simulates skill discovery
and activation given a test query. Depends on P1 (token budget) and V1
(semantic linting). May be deferred to a future milestone if M10 scope
becomes too large.

**X2: Documentation Generation** (#67) — A `doc` subcommand generating
markdown catalogs. Low priority, small scope once P2's `SkillEntry` struct
exists.

**X3: Watch Mode** (#68) — `--watch` flag using the `notify` crate. Low
priority, independent of other work.

**X4: Cross-Skill Conflict Detection** (#69) — Depends on V3 (batch
validation) for multi-directory scanning and P1 (token budget) for budget
analysis. Checks description similarity and name collisions.

**#49: Checksum Verification** — Update `install.sh` and `release.yml` to
generate and verify SHA256 checksums. Independent of other work.

**#70: README Improvements** — TBD, depends on user input.

---

## Wave 1 — Foundation: Structured Diagnostics + Claude Code Fields

Builds the infrastructure that all subsequent waves depend on.

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m10-diagnostics` | #66 | Create `Diagnostic` type, migrate `validate()`, update CLI |
| B | `task/m10-target` | #51 | Add `--target` flag and tiered `KNOWN_KEYS` |

**Merge**: A → `dev/m10`, then B → `dev/m10`. A must merge first (B depends
on the new `Diagnostic` type).

### Agent A — Structured Diagnostics (#66)

1. Create `src/diagnostics.rs`:
   - `Severity` enum (Error, Warning, Info)
   - `Diagnostic` struct (severity, code, message, field, suggestion)
   - `impl fmt::Display for Diagnostic` producing current text format
   - `Diagnostic::is_error()`, `Diagnostic::is_warning()`, `Diagnostic::new()`
   - Error code constants: `E001`–`E013`, `W001`–`W002`

2. Update `src/lib.rs`:
   - Add `pub mod diagnostics;`
   - Re-export `Diagnostic`, `Severity`

3. Refactor `src/validator.rs`:
   - Change `validate()` return type: `Vec<String>` → `Vec<Diagnostic>`
   - Change `validate_metadata()` return type: `Vec<String>` → `Vec<Diagnostic>`
   - Change `validate_name()`: return `Vec<Diagnostic>` with codes E001–E009
   - Change `validate_description()`: return `Vec<Diagnostic>` with codes
     E010–E012
   - Change `validate_compatibility()`: return `Vec<Diagnostic>` with E013
   - Replace string-prefix "warning: " with `Severity::Warning`
   - Each diagnostic includes the relevant `field` value

4. Update `src/errors.rs`:
   - Change `AigentError::Validation { errors: Vec<String> }` to
     `AigentError::Validation { errors: Vec<Diagnostic> }`
   - Update `format_validation_errors` to use `Display` on `Diagnostic`

5. Update `src/main.rs`:
   - `Validate` subcommand gains `--format text|json` flag
   - Error detection uses `d.is_error()` instead of `!m.starts_with("warning:")`
   - Text mode: `eprintln!("{d}")` (same output as before)
   - JSON mode: `serde_json::to_string_pretty(&diagnostics)`

6. Update `src/builder/mod.rs`:
   - `build_skill` validation check uses `d.is_error()`
   - Warning reporting uses `d` directly

7. Update all tests in `src/validator.rs`, `tests/cli.rs`, `src/errors.rs`
   to work with `Diagnostic` instead of `String`

**Verification**: `cargo fmt --check && cargo clippy -- -D warnings &&
cargo test` — all existing tests pass with identical behavior.

### Agent B — Claude Code Field Awareness (#51)

Depends on Agent A completing first.

1. In `src/parser.rs`:
   - Rename `KNOWN_KEYS` → `STANDARD_KEYS`
   - Add `CLAUDE_CODE_KEYS` constant
   - Keep `KNOWN_KEYS` as a deprecated alias for backward compatibility
   - Export `known_keys_for(target)` function

2. In `src/validator.rs`:
   - Add `validate_with_target(dir, target)` function
   - `validate(dir)` delegates to `validate_with_target(dir, Standard)`
   - Unknown-field warning (W001) checks against `known_keys_for(target)`
   - Permissive mode suppresses W001 entirely

3. In `src/main.rs`:
   - Add `--target standard|claude-code|permissive` to `Validate` subcommand
   - Pass target to `validate_with_target`

4. In `src/lib.rs`:
   - Export `validate_with_target`, `ValidationTarget`
   - Export `STANDARD_KEYS`, `CLAUDE_CODE_KEYS`

5. Tests:
   - Skill with `argument-hint` + `--target standard` → W001 warning
   - Skill with `argument-hint` + `--target claude-code` → no warning
   - Skill with `argument-hint` + `--target permissive` → no warning
   - Skill with truly unknown field + `--target claude-code` → W001 warning
   - Backward compat: `validate(dir)` behaves identically to before

---

## Wave 2 — Validator Power: Linting + Batch + Fix-It

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| C | `task/m10-linter` | #50 | Create `src/linter.rs`, add `--lint` flag and `lint` subcommand |
| D | `task/m10-batch` | #52 | Add multi-dir + `--recursive` to validate |
| E | `task/m10-fix` | #53 | Add `--apply-fixes` to validate |

**Merge**: C, D can run in parallel; E depends on C (uses lint codes for
some fixes). Merge order: C → `dev/m10`, D → `dev/m10`, E → `dev/m10`.

### Agent C — Semantic Linting (#50)

1. Create `src/linter.rs`:
   - `pub fn lint(properties: &SkillProperties, body: &str) -> Vec<Diagnostic>`
   - Five lint checks (I001–I005) as described in Design Decisions
   - Each check is a separate function for testability

2. Update `src/lib.rs`:
   - Add `pub mod linter;`
   - Re-export `lint`

3. Update `src/main.rs`:
   - Add `--lint` flag to `Validate` subcommand
   - When `--lint`, append `linter::lint()` results to diagnostics
   - Add standalone `Lint` subcommand (same as `validate --lint`)

4. Tests (in `src/linter.rs`):
   - I001: "I can help you process files" → triggers
   - I001: "Processes files and generates reports" → does not trigger
   - I002: "Processes files" (no trigger) → triggers
   - I002: "Processes files. Use when working with data." → does not trigger
   - I003: "pdf-processor" → triggers
   - I003: "processing-pdfs" → does not trigger
   - I004: "helper" → triggers
   - I004: "processing-pdfs" → does not trigger
   - I005: "Helps" → triggers
   - I005: "Processes PDF files and generates reports..." → does not trigger
   - Lint results have `Severity::Info`

### Agent D — Batch Validation (#52)

1. Update `src/main.rs`:
   - `Validate` accepts `Vec<PathBuf>` instead of single `PathBuf`
   - Add `--recursive` flag
   - Add recursive SKILL.md discovery function in `src/validator.rs` or
     a utility module

2. Recursive discovery:
   - Walk `read_dir` recursively
   - Find all files named `SKILL.md` or `skill.md`
   - Return parent directories
   - Skip hidden directories (starting with `.`)

3. Summary output (text mode):
   ```
   skills/aigent-builder/   PASS
   skills/broken-skill/     FAIL  (2 errors)
   ---
   2 skills checked: 1 passed, 1 failed
   ```

4. JSON mode: Array of per-skill results

5. Exit code: non-zero if any skill has errors

6. Tests:
   - Single directory (backward compatible)
   - Multiple directories on command line
   - `--recursive` discovers nested skills
   - Summary counts are correct
   - Exit code reflects worst result

### Agent E — Fix-It Suggestions (#53)

Depends on Agent C (uses lint codes for suggestions).

1. Add suggestion text to existing diagnostics in `src/validator.rs`:
   - E002: "Truncate to: '{truncated}'"
   - E003 (uppercase): "Use lowercase: '{lowered}'"
   - E006: "Remove consecutive hyphens: '{collapsed}'"
   - E012: "Remove XML tags from description"
   - I002: "Append: 'Use when this capability is needed.'"

2. Add `--apply-fixes` flag to `Validate` subcommand

3. Implement fix application in `src/fixer.rs`:
   - `pub fn apply_fixes(path: &Path, diagnostics: &[Diagnostic]) -> Result<usize>`
   - Parse SKILL.md
   - Apply each fixable diagnostic
   - Re-serialize and write back
   - Return count of fixes applied
   - Re-validate and report remaining issues

4. Tests:
   - Name with uppercase → lowercased after fix
   - Name with consecutive hyphens → collapsed after fix
   - Description with XML tags → tags removed after fix
   - Non-fixable issues remain unchanged
   - Fix count reported correctly

---

## Wave 3 — Builder & Prompt Enhancements

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| F | `task/m10-templates` | #58 | Template system for init/build |
| G | `task/m10-prompt` | #55, #56 | Token budget + multi-format prompt |
| H | `task/m10-checksum` | #49 | Checksum verification for install script |

**Merge**: F, G, H can all run in parallel. Merge order: any.

### Agent F — Template System (#58)

1. Create `src/builder/templates.rs` (replace existing `template.rs`):
   - `pub fn template_files(template: SkillTemplate, name: &str) -> HashMap<String, String>`
   - Each template returns relative path → content pairs
   - `SkillTemplate` enum with 6 variants

2. Update `src/builder/mod.rs`:
   - `init_skill` accepts optional `SkillTemplate` parameter
   - `build_skill` accepts template via `SkillSpec`

3. Update `src/main.rs`:
   - Add `--template` flag to `Init` and `Build` subcommands

4. Template content:
   - `minimal`: current behavior (backward compatible default)
   - `reference-guide`: SKILL.md + REFERENCE.md + EXAMPLES.md
   - `domain-specific`: SKILL.md + reference/domain.md
   - `workflow`: SKILL.md with checklist pattern (from spec)
   - `code-skill`: SKILL.md + scripts/run.sh (with shebang, error handling)
   - `claude-code`: SKILL.md with extension fields in frontmatter

5. Add `SkillTemplate` and `template` field to `SkillSpec` struct

6. Tests:
   - Each template generates expected file set
   - Minimal template matches current behavior
   - Generated files pass validation
   - Template names match `SkillTemplate` variants

### Agent G — Token Budget + Multi-Format Prompt (#55, #56)

1. Refactor `src/prompt.rs`:
   - Extract `SkillEntry` struct: `{ name, description, location }`
   - New `collect_skills(dirs: &[&Path]) -> Vec<SkillEntry>`
   - `to_prompt(dirs)` becomes a wrapper: collect + format as XML
   - New `to_prompt_format(dirs, format)` function

2. Add format implementations:
   - `format_xml(entries: &[SkillEntry]) -> String` (current behavior)
   - `format_json(entries: &[SkillEntry]) -> String`
   - `format_yaml(entries: &[SkillEntry]) -> String`
   - `format_markdown(entries: &[SkillEntry]) -> String`

3. Token budget:
   - `estimate_tokens(s: &str) -> usize` — `s.len() / 4`
   - `format_budget(entries: &[SkillEntry]) -> String` — per-skill + total
   - Warning if total > 4000

4. Update `src/main.rs`:
   - Add `--format xml|json|yaml|markdown` to `ToPrompt`
   - Add `--budget` flag to `ToPrompt`

5. Tests:
   - XML output matches current behavior exactly
   - JSON output is valid JSON array
   - YAML output is valid YAML
   - Markdown output has expected headings
   - Budget estimates are reasonable
   - Budget warning triggers at threshold

### Agent H — Checksum Verification (#49)

1. Update `.github/workflows/release.yml`:
   - Add step after artifact upload: compute SHA256 for each archive
   - Upload `checksums.txt` as release asset

2. Update `install.sh`:
   - Download `checksums.txt` from release
   - Compute SHA256 of downloaded archive (`sha256sum` or `shasum -a 256`)
   - Compare against published checksum
   - Fail with clear error on mismatch

3. Tests:
   - `install.sh` contains `sha256` or `shasum` reference
   - `release.yml` contains checksum generation step

---

## Wave 4 — Plugin Depth

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| I | `task/m10-hooks` | #62 | Add PostToolUse hooks for SKILL.md validation |
| J | `task/m10-scorer` | #63 | Add aigent-scorer skill |
| K | `task/m10-fork` | #65 | Add context:fork to builder skill |

**Merge**: I, J, K can all run in parallel. Merge order: any.

### Agent I — Hooks (#62)

1. Create `hooks/hooks.json` with PostToolUse hook
2. Hook detects SKILL.md writes and runs `aigent validate`
3. Test: `hooks/hooks.json` is valid JSON, matches expected structure

### Agent J — Scorer Skill (#63)

1. Create `skills/aigent-scorer/SKILL.md`:
   - Hybrid mode (CLI `aigent validate --lint` / prompt-only checklist)
   - Embeds the Anthropic best-practices checklist for prompt-only mode
2. Self-validation: `aigent validate skills/aigent-scorer/`
3. Test in `tests/plugin.rs`: scorer skill passes validation

### Agent K — context:fork (#65)

1. Update `skills/aigent-builder/SKILL.md` frontmatter:
   - Add `context: fork`
2. Verify skill still passes validation with `--target claude-code`
3. No test change needed — existing `--target standard` tests will flag
   `context` as W001 warning (expected; plugin tests should use
   `--target claude-code`)

---

## Wave 5 — Polish: Interactive Build + Quality + Remaining

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| L | `task/m10-interactive` | #60 | Add `--interactive` flag to build |
| M | `task/m10-score` | #59 | Add `score` subcommand |
| N | `task/m10-remaining` | #57, #67, #68 | Diff output, doc gen, watch mode |

**Merge**: L, M, N can run in parallel. Merge order: any.

### Agent L — Interactive Build (#60)

1. Add `--interactive` / `-i` flag to `Build` subcommand
2. Interactive flow:
   - `assess_clarity(purpose)` — print questions if unclear, exit
   - Print "Name: {name}" — prompt to continue
   - Print "Description: {description}" — prompt to continue
   - Print body preview (first 20 lines)
   - Prompt for confirmation before writing
   - After write, run validation and report
3. Read confirmation from stdin (`std::io::stdin().read_line()`)
4. Tests:
   - Non-interactive mode unchanged (backward compatible)
   - Interactive mode tested with piped stdin

### Agent M — Quality Assessment (#59)

1. Add `Score` subcommand:
   ```rust
   Score {
       skill_dir: PathBuf,
       #[arg(long, value_enum, default_value = "text")]
       format: OutputFormat,
   }
   ```
2. Scoring logic in `src/scorer.rs`:
   - Run validation (structural checks)
   - Run linting (semantic checks)
   - Weight checks: structural pass = 60 points base, each lint pass = +8
     points (5 checks = 40 max)
   - Return score 0–100 with breakdown
3. Output:
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
4. JSON output when `--format json`

### Agent N — Remaining Items (#57, #67, #68)

**P3: Diff-Aware Prompt (#57)**:
1. Add `--output <file>` flag to `ToPrompt`
2. Write prompt output to file instead of stdout
3. If file exists, compare and report changes

**X2: Documentation Generation (#67)**:
1. Add `Doc` subcommand accepting one or more directories
2. Use `SkillEntry` from P2 refactor
3. Generate markdown catalog
4. Write to stdout or `--output <file>`

**X3: Watch Mode (#68)**:
1. Add `notify` dependency to Cargo.toml
2. Add `--watch` flag to `Validate`
3. Watch for file changes in skill directory
4. Re-run validation on change
5. Clear terminal between runs

---

## Wave 6 — Deferred / Lower Priority

These items are tracked but intentionally deferred. They may be implemented in
M10 if time permits, or deferred to M11.

| Issue | Title | Reason for deferral |
|-------|-------|---------------------|
| #54 | Directory structure validation | Depends on X1; lower priority |
| #61 | Skill upgrade command | Depends on V4; overlaps with fix-it |
| #64 | Skill tester and previewer | Highest effort; needs design work |
| #69 | Cross-skill conflict detection | Depends on V3, P1; needs design |
| #70 | README improvements | Awaiting user input |

---

## Wave 7 — Verify

Single agent runs the full check suite on `dev/m10`.

| Agent | Branch | Task |
|-------|--------|------|
| O | `dev/m10` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Cargo.toml Changes

```toml
[dependencies]
# existing...
notify = "8"       # Watch mode (X3) — only if X3 is implemented
```

No other new dependencies required. All new features use existing deps
(`serde`, `serde_json`, `regex`, `clap`).

---

## New Module Map

After M10, the source tree grows to:

```
src/
├── lib.rs              # Library root — re-exports
├── errors.rs           # AigentError (updated: Vec<Diagnostic>)
├── models.rs           # SkillProperties
├── parser.rs           # Frontmatter parser (STANDARD_KEYS, CLAUDE_CODE_KEYS)
├── validator.rs        # Structural validation (returns Vec<Diagnostic>)
├── diagnostics.rs      # NEW: Severity, Diagnostic, error codes
├── linter.rs           # NEW: Semantic lint checks (I001–I005)
├── fixer.rs            # NEW: Auto-fix application
├── scorer.rs           # NEW: Quality scoring
├── prompt.rs           # Prompt generation (SkillEntry, multi-format)
├── builder/
│   ├── mod.rs          # build_skill, init_skill
│   ├── deterministic.rs
│   ├── llm.rs
│   ├── providers/
│   └── templates.rs    # UPDATED: 6 template variants
└── main.rs             # CLI (updated with new subcommands + flags)
```

---

## New CLI Surface

After M10:

```
aigent validate <dirs...> [--recursive] [--lint] [--target standard|claude-code|permissive]
                          [--format text|json] [--apply-fixes] [--watch]
aigent lint <skill-dir>
aigent score <skill-dir> [--format text|json]
aigent read-properties <skill-dir>
aigent to-prompt <dirs...> [--format xml|json|yaml|markdown] [--budget] [--output <file>]
aigent build <purpose> [--name] [--dir] [--no-llm] [--interactive] [--template <template>]
aigent init [dir] [--template <template>]
aigent doc <dirs...> [--output <file>]
aigent --about
```

---

## Deliverables

- `src/diagnostics.rs` — `Severity`, `Diagnostic`, error code registry
- `src/linter.rs` — 5 semantic lint checks (I001–I005)
- `src/fixer.rs` — auto-fix application for fixable issues
- `src/scorer.rs` — quality scoring (0–100) with breakdown
- `src/validator.rs` — refactored to return `Vec<Diagnostic>`, tiered
  `KNOWN_KEYS`, batch + recursive validation
- `src/prompt.rs` — `SkillEntry`, multi-format output, token budget
- `src/builder/templates.rs` — 6 template variants
- `src/main.rs` — new subcommands (`lint`, `score`, `doc`), new flags
  (`--format`, `--target`, `--lint`, `--recursive`, `--apply-fixes`,
  `--interactive`, `--template`, `--budget`, `--output`, `--watch`)
- `hooks/hooks.json` — PostToolUse SKILL.md validation hook
- `skills/aigent-scorer/SKILL.md` — quality assessment skill
- `install.sh` — updated with checksum verification
- `.github/workflows/release.yml` — SHA256 checksum generation
- Updated tests across all modules
- PR: `M10: Improvements and Extensions`

---

> **⚠ Plan Update — Original Plan Obsolete**
>
> The plan above (Waves 1–7, 22 issues) has been superseded. The scope was
> too large for a single milestone. Issues have been redistributed:
>
> - **M10** (this milestone): Foundation — 5 issues (#50, #51, #52, #53, #66)
> - **M11**: Builder & Prompt Enhancements — 8 issues (#49, #55, #56, #57, #58, #60, #62, #65)
> - **M12**: Ecosystem & Workflow — 9 issues (#54, #59, #61, #63, #64, #67, #68, #69, #70)
>
> The revised plan for M10 follows below. See `dev/m11/plan.md` and
> `dev/m12/plan.md` for the other two milestones.

---

# M10: Validator Foundation — Revised Plan

## Overview

Core validator infrastructure overhaul. Introduces structured diagnostics,
Claude Code field awareness, semantic linting, batch validation, and fix-it
suggestions. Establishes the typed `Diagnostic` infrastructure that M11 and
M12 build upon.

Issues: #50, #51, #52, #53, #66.

## Branch Strategy

- **Dev branch**: `dev/m10` (created from `main`)
- **Task branches**: `task/m10-<name>` (created from `dev/m10`)
- After each wave, task branches merge into `dev/m10`
- After all waves, PR from `dev/m10` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- M4/M6: `validate()` in `src/validator.rs` — the refactoring target
- M3: `src/parser.rs` — `KNOWN_KEYS`, `parse_frontmatter`, `read_properties`
- M2: `src/errors.rs` — `AigentError`, `Result<T>` alias

## Current State

All M1–M9 milestones completed. The key refactoring surfaces:

- `src/validator.rs`: 30+ validation rules, returns `Vec<String>` with ad-hoc
  `"warning: "` prefix convention
- `src/errors.rs`: `AigentError::Validation { errors: Vec<String> }`
- `src/main.rs`: error detection via `!m.starts_with("warning: ")`
- `src/builder/mod.rs`: validation check uses the same string prefix

The `Vec<String>` representation permeates the codebase. This milestone
replaces it with structured `Diagnostic` types — the prerequisite for
semantic linting, fix-it suggestions, batch validation, and JSON output.

---

## Design Decisions

All design decisions from the original plan remain valid. The key designs are:

- **Structured Diagnostics** (X1/#66): `Diagnostic` struct with `Severity`,
  stable error codes (E001–E013, W001–W002), optional field/suggestion.
  `impl Display` produces backward-compatible text. See original plan for
  the full Error Code Registry and migration strategy.

- **Claude Code Extension Fields** (V2/#51): Tiered `KNOWN_KEYS` →
  `STANDARD_KEYS` + `CLAUDE_CODE_KEYS` with `--target` flag. See original
  plan for `known_keys_for(target)` design.

- **Semantic Linting** (V1/#50): New `src/linter.rs` with 5 Info-level
  checks (I001–I005). See original plan for lint check specifications.

- **Batch Validation** (V3/#52): `Vec<PathBuf>` + `--recursive` + summary
  output. See original plan for discovery algorithm and output format.

- **Fix-It Suggestions** (V4/#53): Suggestion text in `Diagnostic` +
  `--apply-fixes` flag + `src/fixer.rs`. See original plan for fixable/
  non-fixable issue table.

---

## Wave 1 — Foundation: Structured Diagnostics + Claude Code Fields

Builds the infrastructure that all subsequent work depends on.

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m10-diagnostics` | #66 | Create `Diagnostic` type, migrate `validate()`, update CLI |
| B | `task/m10-target` | #51 | Add `--target` flag and tiered `KNOWN_KEYS` |

**Merge**: A → `dev/m10`, then B → `dev/m10`. A must merge first (B depends
on the new `Diagnostic` type).

### Agent A — Structured Diagnostics (#66)

1. Create `src/diagnostics.rs`:
   - `Severity` enum (Error, Warning, Info)
   - `Diagnostic` struct (severity, code, message, field, suggestion)
   - `impl fmt::Display for Diagnostic` producing current text format
   - `Diagnostic::is_error()`, `Diagnostic::is_warning()`, `Diagnostic::new()`
   - Error code constants: `E001`–`E013`, `W001`–`W002`

2. Update `src/lib.rs`:
   - Add `pub mod diagnostics;`
   - Re-export `Diagnostic`, `Severity`

3. Refactor `src/validator.rs`:
   - Change `validate()` return type: `Vec<String>` → `Vec<Diagnostic>`
   - Change `validate_metadata()` return type: `Vec<String>` → `Vec<Diagnostic>`
   - Change `validate_name()`: return `Vec<Diagnostic>` with codes E001–E009
   - Change `validate_description()`: return `Vec<Diagnostic>` with codes
     E010–E012
   - Change `validate_compatibility()`: return `Vec<Diagnostic>` with E013
   - Replace string-prefix `"warning: "` with `Severity::Warning`
   - Each diagnostic includes the relevant `field` value

4. Update `src/errors.rs`:
   - Change `AigentError::Validation { errors: Vec<String> }` to
     `AigentError::Validation { errors: Vec<Diagnostic> }`
   - Update `format_validation_errors` to use `Display` on `Diagnostic`

5. Update `src/main.rs`:
   - `Validate` subcommand gains `--format text|json` flag
   - Error detection uses `d.is_error()` instead of `!m.starts_with("warning:")`
   - Text mode: `eprintln!("{d}")` (same output as before)
   - JSON mode: `serde_json::to_string_pretty(&diagnostics)`

6. Update `src/builder/mod.rs`:
   - `build_skill` validation check uses `d.is_error()`
   - Warning reporting uses `d` directly

7. Update all tests in `src/validator.rs`, `tests/cli.rs`, `src/errors.rs`
   to work with `Diagnostic` instead of `String`

**Verification**: `cargo fmt --check && cargo clippy -- -D warnings &&
cargo test` — all existing tests pass with identical behavior.

### Agent B — Claude Code Field Awareness (#51)

Depends on Agent A completing first.

1. In `src/parser.rs`:
   - Rename `KNOWN_KEYS` → `STANDARD_KEYS`
   - Add `CLAUDE_CODE_KEYS` constant
   - Keep `KNOWN_KEYS` as a deprecated alias for backward compatibility
   - Export `known_keys_for(target)` function

2. In `src/validator.rs`:
   - Add `validate_with_target(dir, target)` function
   - `validate(dir)` delegates to `validate_with_target(dir, Standard)`
   - Unknown-field warning (W001) checks against `known_keys_for(target)`
   - Permissive mode suppresses W001 entirely

3. In `src/main.rs`:
   - Add `--target standard|claude-code|permissive` to `Validate` subcommand
   - Pass target to `validate_with_target`

4. In `src/lib.rs`:
   - Export `validate_with_target`, `ValidationTarget`
   - Export `STANDARD_KEYS`, `CLAUDE_CODE_KEYS`

5. Tests:
   - Skill with `argument-hint` + `--target standard` → W001 warning
   - Skill with `argument-hint` + `--target claude-code` → no warning
   - Skill with `argument-hint` + `--target permissive` → no warning
   - Skill with truly unknown field + `--target claude-code` → W001 warning
   - Backward compat: `validate(dir)` behaves identically to before

---

## Wave 2 — Validator Power: Linting + Batch + Fix-It

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| C | `task/m10-linter` | #50 | Create `src/linter.rs`, add `--lint` flag and `lint` subcommand |
| D | `task/m10-batch` | #52 | Add multi-dir + `--recursive` to validate |
| E | `task/m10-fix` | #53 | Add `--apply-fixes` to validate |

**Merge**: C, D can run in parallel; E depends on C (uses lint codes for
some fixes). Merge order: C → `dev/m10`, D → `dev/m10`, E → `dev/m10`.

### Agent C — Semantic Linting (#50)

1. Create `src/linter.rs`:
   - `pub fn lint(properties: &SkillProperties, body: &str) -> Vec<Diagnostic>`
   - Five lint checks (I001–I005) as described in Design Decisions
   - Each check is a separate function for testability

2. Update `src/lib.rs`:
   - Add `pub mod linter;`
   - Re-export `lint`

3. Update `src/main.rs`:
   - Add `--lint` flag to `Validate` subcommand
   - When `--lint`, append `linter::lint()` results to diagnostics
   - Add standalone `Lint` subcommand (same as `validate --lint`)

4. Tests (in `src/linter.rs`):
   - I001: "I can help you process files" → triggers
   - I001: "Processes files and generates reports" → does not trigger
   - I002: "Processes files" (no trigger) → triggers
   - I002: "Processes files. Use when working with data." → does not trigger
   - I003: "pdf-processor" → triggers
   - I003: "processing-pdfs" → does not trigger
   - I004: "helper" → triggers
   - I004: "processing-pdfs" → does not trigger
   - I005: "Helps" → triggers
   - I005: "Processes PDF files and generates reports..." → does not trigger
   - Lint results have `Severity::Info`

### Agent D — Batch Validation (#52)

1. Update `src/main.rs`:
   - `Validate` accepts `Vec<PathBuf>` instead of single `PathBuf`
   - Add `--recursive` flag
   - Add recursive SKILL.md discovery function in `src/validator.rs`

2. Recursive discovery:
   - Walk `read_dir` recursively
   - Find all files named `SKILL.md` or `skill.md`
   - Return parent directories
   - Skip hidden directories (starting with `.`)

3. Summary output (text mode):
   ```
   skills/aigent-builder/   PASS
   skills/broken-skill/     FAIL  (2 errors)
   ---
   2 skills checked: 1 passed, 1 failed
   ```

4. JSON mode: Array of per-skill results

5. Exit code: non-zero if any skill has errors

6. Tests:
   - Single directory (backward compatible)
   - Multiple directories on command line
   - `--recursive` discovers nested skills
   - Summary counts are correct
   - Exit code reflects worst result

### Agent E — Fix-It Suggestions (#53)

Depends on Agent C (uses lint codes for suggestions).

1. Add suggestion text to existing diagnostics in `src/validator.rs`:
   - E002: "Truncate to: '{truncated}'"
   - E003 (uppercase): "Use lowercase: '{lowered}'"
   - E006: "Remove consecutive hyphens: '{collapsed}'"
   - E012: "Remove XML tags from description"
   - I002: "Append: 'Use when this capability is needed.'"

2. Add `--apply-fixes` flag to `Validate` subcommand

3. Implement fix application in `src/fixer.rs`:
   - `pub fn apply_fixes(path: &Path, diagnostics: &[Diagnostic]) -> Result<usize>`
   - Parse SKILL.md
   - Apply each fixable diagnostic
   - Re-serialize and write back
   - Return count of fixes applied
   - Re-validate and report remaining issues

4. Tests:
   - Name with uppercase → lowercased after fix
   - Name with consecutive hyphens → collapsed after fix
   - Description with XML tags → tags removed after fix
   - Non-fixable issues remain unchanged
   - Fix count reported correctly

---

## Wave 3 — Verify

Single agent runs the full check suite on `dev/m10`.

| Agent | Branch | Task |
|-------|--------|------|
| F | `dev/m10` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Cargo.toml Changes

No new dependencies required. All features use existing deps
(`serde`, `serde_json`, `regex`, `clap`).

---

## New Module Map

After M10, the source tree grows to:

```
src/
├── lib.rs              # Library root — re-exports
├── errors.rs           # AigentError (updated: Vec<Diagnostic>)
├── models.rs           # SkillProperties
├── parser.rs           # Frontmatter parser (STANDARD_KEYS, CLAUDE_CODE_KEYS)
├── validator.rs        # Structural validation (returns Vec<Diagnostic>)
├── diagnostics.rs      # NEW: Severity, Diagnostic, error codes
├── linter.rs           # NEW: Semantic lint checks (I001–I005)
├── fixer.rs            # NEW: Auto-fix application
├── prompt.rs           # Prompt generation (unchanged)
├── builder/
│   ├── mod.rs          # build_skill, init_skill (updated for Diagnostic)
│   ├── deterministic.rs
│   ├── llm.rs
│   ├── providers/
│   └── templates.rs    # Unchanged (templates deferred to M11)
└── main.rs             # CLI (updated with new flags)
```

---

## New CLI Surface

After M10:

```
aigent validate <dirs...> [--recursive] [--lint] [--target standard|claude-code|permissive]
                          [--format text|json] [--apply-fixes]
aigent lint <skill-dir>
aigent read-properties <skill-dir>
aigent to-prompt <dirs...>
aigent build <purpose> [--name] [--dir] [--no-llm]
aigent init [dir]
aigent --about
```

---

## Deliverables

- `src/diagnostics.rs` — `Severity`, `Diagnostic`, error code registry
- `src/linter.rs` — 5 semantic lint checks (I001–I005)
- `src/fixer.rs` — auto-fix application for fixable issues
- `src/validator.rs` — refactored to return `Vec<Diagnostic>`, tiered
  `KNOWN_KEYS`, batch + recursive validation
- `src/main.rs` — new subcommand (`lint`), new flags (`--format`, `--target`,
  `--lint`, `--recursive`, `--apply-fixes`)
- `src/errors.rs` — updated `Validation` variant with `Vec<Diagnostic>`
- `src/parser.rs` — `STANDARD_KEYS`, `CLAUDE_CODE_KEYS`, `known_keys_for()`
- Updated tests across all modules
- PR: `M10: Validator Foundation`
