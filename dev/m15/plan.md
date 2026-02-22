# M15: Plugin Ecosystem Validation — Work Plan

> **Note**: v1 (below) is obsolete. See [v2 reconciliation](#v2-reconciled-plan-post-review-amendments) at end of file.

## Table of Contents

- [Overview](#overview)
- [Baseline](#baseline)
- [Dependencies](#dependencies)
- [Scope Boundaries](#scope-boundaries)
- [Architecture](#architecture)
  - [Module Structure](#module-structure)
  - [Diagnostic Code Namespaces](#diagnostic-code-namespaces)
  - [Shared Infrastructure](#shared-infrastructure)
  - [CLI Integration](#cli-integration)
- [Design Decisions](#design-decisions)
  - [P: Plugin Manifest Validation (#99)](#p-plugin-manifest-validation-99)
  - [H: Hook Validation (#97)](#h-hook-validation-97)
  - [A: Agent File Validation (#98)](#a-agent-file-validation-98)
  - [K: Command File Validation (#100)](#k-command-file-validation-100)
  - [X: Cross-Component Consistency (#101)](#x-cross-component-consistency-101)
  - [CLI Improvements (#110, #112, #113)](#cli-improvements-110-112-113)
  - [Scaffolding Enhancement (#111)](#scaffolding-enhancement-111)
  - [Test Runner Enhancement (#104)](#test-runner-enhancement-104)
- [Wave Plan](#wave-plan)
  - [Wave 1: Foundation — Manifest + Shared Infrastructure (#99)](#wave-1-foundation--manifest--shared-infrastructure-99)
  - [Wave 2: Component Validators (#97, #98, #100)](#wave-2-component-validators-97-98-100)
  - [Wave 3: Cross-Component + CLI Polish (#101, #110, #112, #113)](#wave-3-cross-component--cli-polish-101-110-112-113)
  - [Wave 4: Enhancements (#104, #111)](#wave-4-enhancements-104-111)
- [Issue Summary](#issue-summary)
- [Risk Assessment](#risk-assessment)
- [Estimated Scope](#estimated-scope)

## Overview

Extend aigent's deterministic validation from SKILL.md to the full Claude
Code plugin ecosystem: hooks, agents, commands, manifest, and cross-component
consistency. This is the milestone where aigent grows from a skill-only tool
to a plugin-wide validator.

Issues: #97, #98, #99, #100, #101, #104, #110, #111, #112, #113.

Milestone description: "Complements plugin-dev by mechanizing the rules it
teaches." See `dev/plugin-dev.md` for the full analysis of what rules are
mechanizable.

## Baseline

Main at `2c2309d` (M13 merged, M14 pending). 561 tests (413 unit + 120 CLI
+ 27 plugin + 1 doc-test). ~21,800 lines across source files.

Existing diagnostic codes: E000–E018, W001–W002, S001–S006, C001–C003.
Lint codes (string literals, not constants): I001–I005.

Existing infrastructure to build on:
- `Diagnostic` type with severity, code, field, suggestion
- `Vec<Diagnostic>` accumulation pattern (never fail-fast)
- `discover_skills_recursive` for filesystem traversal
- `parse_frontmatter` for YAML between `---` delimiters
- `--format json` output for all validation commands
- `ValidationTarget` for controlling strictness

## Dependencies

- **M14 (SRE Review)**: Should merge first — it hardens the validator
  infrastructure (symlink safety, file size caps, error propagation) that
  M15 builds on. However, M15 can start independently since it adds new
  modules rather than modifying existing ones. The shared touchpoints are
  `diagnostics.rs` (new code ranges) and `main.rs` (new CLI commands).

- **New crate dependencies**: `serde_json` is already a dependency.
  No new crates needed.

## Scope Boundaries

**In scope:**
- Deterministic validation of plugin.json, hooks.json, agent .md files,
  command .md files
- Cross-component consistency checks (reference resolution, orphan detection)
- CLI improvements (#110, #112, #113)
- Scaffolding enhancement for `init`/`new` (#111)
- Test runner `strength` field (#104)

**Out of scope:**
- MCP server validation (no deterministic rules to enforce — runtime-dependent)
- Plugin settings validation (`.local.md` files are per-project, not distributable)
- Auto-fix for non-skill components (future milestone)
- Formatting for non-skill components (future milestone)
- Scoring for non-skill components (future milestone)

## Architecture

### Module Structure

```
src/
├── plugin/                     # New: plugin ecosystem validation
│   ├── mod.rs                  # Re-exports, PluginComponent enum
│   ├── manifest.rs             # plugin.json validation (#99)
│   ├── hooks.rs                # hooks.json validation (#97)
│   ├── agent.rs                # Agent .md validation (#98)
│   ├── command.rs              # Command .md validation (#100)
│   └── cross.rs                # Cross-component checks (#101)
├── diagnostics.rs              # Extended with new code ranges
├── lib.rs                      # Re-export plugin module
└── main.rs                     # New validate-plugin command
```

A `src/plugin/` submodule keeps the new validators grouped and avoids
cluttering the top-level `src/`. Each file follows the same pattern as the
existing validators: takes a path, returns `Vec<Diagnostic>`.

### Diagnostic Code Namespaces

| Prefix | Component | Range |
|--------|-----------|-------|
| `E` | Skill metadata (existing) | E000–E018 |
| `W` | Skill warnings (existing) | W001–W002 |
| `S` | Skill structure (existing) | S001–S006 |
| `C` | Cross-skill conflicts (existing) | C001–C003 |
| `P` | Plugin manifest | P001–P010 |
| `H` | Hooks | H001–H010 |
| `A` | Agents | A001–A010 |
| `K` | Commands (slash commands) | K001–K010 |
| `X` | Cross-component consistency | X001–X010 |

Note: `C` is taken by conflict detection. Commands use `K` (for "Kommand"
or think of it as "command **K**ey").

### Shared Infrastructure

The existing `parse_frontmatter` in `parser.rs` already handles the YAML
frontmatter pattern used by both agent and command files. No new parsing
infrastructure is needed — agent/command validators call `parse_frontmatter`
directly.

For hooks and manifest, `serde_json::from_str` suffices.

**Discovery**: A generic `discover_component` helper or per-component
discovery functions following the `discover_skills` pattern. Plugin-level
validation starts from `plugin.json`, which declares component directories
(or uses defaults: `skills/`, `agents/`, `hooks/`, `commands/`).

### CLI Integration

Two options for exposing plugin-wide validation:

| Approach | Usage | Pros | Cons |
|----------|-------|------|------|
| **A. New subcommand** | `aigent validate-plugin .` | Clear intent, dedicated flags | New command to learn |
| **B. Extend `validate`** | `aigent validate --plugin .` | Familiar command | Overloaded semantics |

**Decision: Option A (`validate-plugin`).** Plugin-wide validation is
fundamentally different from skill validation — it takes a plugin root
directory (not skill directories), discovers components from `plugin.json`,
and runs cross-component checks. A dedicated subcommand avoids overloading
`validate` and keeps the CLI predictable.

Individual component validators can also be used standalone through the
existing `validate` command pattern, but the primary entry point for M15 is
`validate-plugin`.

```
aigent validate-plugin [<plugin-dir>] [--format text|json]
```

Defaults to `.` (current directory). Discovers components from `plugin.json`,
runs all validators, then cross-component checks. Output grouped by
component.

---

## Design Decisions

### P: Plugin Manifest Validation (#99)

**Input**: Path to `plugin.json` (or directory containing it).

**Model**: Parse into a `PluginManifest` struct with `serde_json`:

```rust
#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<AuthorField>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Option<Vec<String>>,
    // Component path overrides (if Claude Code supports them)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AuthorField {
    Simple(String),
    Detailed { name: String, url: Option<String> },
}
```

**Validation rules → diagnostic codes:**

| Code | Severity | Rule |
|------|----------|------|
| P001 | Error | JSON syntax error |
| P002 | Error | `name` field missing |
| P003 | Error | `name` not kebab-case or contains spaces |
| P004 | Warning | `version` not semver format (x.y.z) |
| P005 | Warning | `description` empty or missing |
| P006 | Error | Custom path uses absolute path (must start with `./`) |
| P007 | Error | Declared component path does not exist on filesystem |
| P008 | Error | Hardcoded credential/token detected in string values |
| P009 | Warning | MCP server URL uses HTTP/WS instead of HTTPS/WSS |
| P010 | Info | Missing recommended field (author, homepage, license) |

**Credential scanning (P008)**: Regex patterns for common secrets:
`(?i)(api[_-]?key|token|secret|password|credential)\s*[:=]\s*["'][^"']+["']`.
Scan all string values in the JSON, not just top-level fields.

### H: Hook Validation (#97)

**Input**: Path to `hooks.json` file.

**Model**: Parse into typed structs:

```rust
#[derive(Debug, Deserialize)]
pub struct HooksFile(pub HashMap<String, Vec<HookEntry>>);

#[derive(Debug, Deserialize)]
pub struct HookEntry {
    pub matcher: Option<String>,
    pub hooks: Vec<HookDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct HookDefinition {
    #[serde(rename = "type")]
    pub hook_type: String,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub timeout: Option<f64>,
}
```

**Validation rules → diagnostic codes:**

| Code | Severity | Rule |
|------|----------|------|
| H001 | Error | JSON syntax error |
| H002 | Error | Unknown event name (not one of 9 valid types) |
| H003 | Error | Hook entry missing `hooks` array |
| H004 | Error | Hook missing `type` field |
| H005 | Error | Unknown hook type (not `command` or `prompt`) |
| H006 | Error | Command hook missing `command` field |
| H007 | Error | Prompt hook missing `prompt` field |
| H008 | Warning | Timeout outside recommended range (5–600 seconds) |
| H009 | Warning | Hardcoded absolute path (should use `${CLAUDE_PLUGIN_ROOT}`) |
| H010 | Info | Prompt hook on suboptimal event (not Stop/SubagentStop/UserPromptSubmit/PreToolUse) |

**Valid event names** (constant array):
`PreToolUse`, `PostToolUse`, `Stop`, `SubagentStop`, `SessionStart`,
`SessionEnd`, `UserPromptSubmit`, `PreCompact`, `Notification`.

### A: Agent File Validation (#98)

**Input**: Path to agent `.md` file (YAML frontmatter + system prompt body).

**Parsing**: Reuses `parse_frontmatter` from `parser.rs`. The frontmatter
schema differs from SKILL.md but the parsing is identical.

**Validation rules → diagnostic codes:**

| Code | Severity | Rule |
|------|----------|------|
| A001 | Error | Frontmatter missing (no `---` delimiters) |
| A002 | Error | Required field missing (`name`, `description`, `model`, `color`) |
| A003 | Error | `name` not kebab-case |
| A004 | Warning | `name` is generic ("helper", "assistant", "agent", "tool") |
| A005 | Error | `name` length outside 3–50 chars |
| A006 | Error | `description` length outside 10–5000 chars |
| A007 | Error | `model` not one of: `inherit`, `sonnet`, `opus`, `haiku` |
| A008 | Error | `color` not one of: `blue`, `cyan`, `green`, `yellow`, `magenta`, `red` |
| A009 | Error | System prompt (body) missing or <20 chars |
| A010 | Warning | System prompt >10k chars |

**Reuse from skill validator**: Generic name detection (A004) can reuse the
pattern from linter's I004. Kebab-case checking (A003) can reuse the regex
from E003.

### K: Command File Validation (#100)

**Input**: Path to command `.md` file (optional YAML frontmatter + body).

**Parsing**: Reuses `parse_frontmatter`. If no frontmatter, the entire file
is the body — this is valid for commands (all frontmatter fields are optional).

**Validation rules → diagnostic codes:**

| Code | Severity | Rule |
|------|----------|------|
| K001 | Error | Frontmatter syntax error (if `---` present but invalid YAML) |
| K002 | Warning | `description` exceeds 60 chars |
| K003 | Error | `model` not one of: `sonnet`, `opus`, `haiku` |
| K004 | Warning | `description` does not start with a verb |
| K005 | Error | Body is empty (no content after frontmatter) |
| K006 | Warning | `allowed-tools` invalid format |
| K007 | Info | Missing `description` (recommended for discoverability) |

**Lighter validation**: Commands have all-optional frontmatter. The validator
only checks fields that are present — it doesn't require any specific fields.
This is intentionally lighter than skill or agent validation.

### X: Cross-Component Consistency (#101)

**Input**: Plugin root directory (containing `plugin.json`).

**Depends on**: P (#99) for manifest parsing, plus all per-component
validators for discovering components.

**Validation rules → diagnostic codes:**

| Code | Severity | Rule |
|------|----------|------|
| X001 | Error | Manifest declares path that doesn't exist |
| X002 | Error | Command hook references script that doesn't exist |
| X003 | Warning | Orphaned file in component directory (not referenced) |
| X004 | Warning | Naming inconsistency (mixed kebab-case violations across components) |
| X005 | Info | Total token budget across all skills exceeds threshold |
| X006 | Error | Duplicate component names across types (e.g., agent and skill with same name) |

**Implementation approach**: The `validate-plugin` command:

1. Parse `plugin.json` → `PluginManifest` (also runs P-series checks)
2. Discover component directories from manifest (or defaults)
3. Run per-component validators (skill, agent, hook, command)
4. Run cross-component checks (X-series) using collected component metadata
5. Return unified `Vec<Diagnostic>` with source component paths

**Hook script resolution (X002)**: For command hooks, expand
`${CLAUDE_PLUGIN_ROOT}` to the plugin directory, then check if the
referenced script path exists. Only applies to hooks with `type: "command"`.

**Orphan detection (X003)**: List files in each component directory, compare
against files referenced by `plugin.json` or discovered by validators.
Report files that exist but aren't referenced. Exclude common non-component
files (`.gitkeep`, `README.md`, etc.).

### CLI Improvements (#110, #112, #113)

**#110 — Confirmation on success**: When `validate`, `check`, or `fmt`
completes with zero diagnostics (single-dir, text format), print `ok` to
stderr. Multi-dir mode already prints a summary. JSON mode prints nothing
extra (machine consumers check the exit code).

**#112 — Misleading "build error" in init/new**: Map `AigentError::Build`
to a cleaner message at the CLI level for `init` and `new` commands. Either
strip the "build error:" prefix in the display, or add a dedicated
`AigentError::AlreadyExists` variant. The latter is cleaner since M14's
#93 (TOCTOU fix) already maps `io::ErrorKind::AlreadyExists` to
`AigentError::Build`.

**Decision**: Add `AigentError::AlreadyExists { path: PathBuf }` variant.
Display as `error: already exists: <path>`. Update `builder/mod.rs` to use
this variant instead of `AigentError::Build` for the `create_new(true)`
failure path.

**#113 — Probe output alignment**: Calculate the maximum label width across
all output fields, then pad values to a consistent column. Multiline values
(descriptions) wrap with continuation indentation. This is a display-only
change in `main.rs`'s `run_probe` function.

### Scaffolding Enhancement (#111)

**#111 — Scaffold supporting dirs**: When `init` or `new --no-llm` creates
a skill directory, also create `examples/` and `scripts/` subdirectories
with `.gitkeep` files. Add a `--minimal` flag to skip scaffolding.

For `new` (with LLM): The LLM builder already supports `extra_files` in
its output — no changes needed.

**Builder change**: In `builder/mod.rs`, after writing `SKILL.md`, create
the subdirectories:

```rust
if !minimal {
    fs::create_dir_all(dir.join("examples"))?;
    fs::write(dir.join("examples/.gitkeep"), "")?;
    fs::create_dir_all(dir.join("scripts"))?;
    fs::write(dir.join("scripts/.gitkeep"), "")?;
}
```

### Test Runner Enhancement (#104)

**#104 — Strength assertion**: Add optional `strength` field to `TestQuery`:

```rust
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MatchStrength {
    Strong,
    Weak,
    None,
}
```

Mapping: `Strong` → score >= 0.6, `Weak` → score >= 0.3, `None` → score < 0.3.
When both `strength` and `min_score` are present, `min_score` takes precedence.
Update `test --generate` to emit `strength` instead of `min_score`.

---

## Wave Plan

### Wave 1: Foundation — Manifest + Shared Infrastructure (#99)

The manifest validator is the foundation: `validate-plugin` needs
`plugin.json` parsing to discover components. This wave establishes the
`src/plugin/` module structure and the `validate-plugin` CLI command.

**Tasks:**

1. Create `src/plugin/mod.rs` with `PluginComponent` enum and re-exports
2. Create `src/plugin/manifest.rs` with `PluginManifest` struct and
   `validate_manifest(path) -> Vec<Diagnostic>`
3. Add diagnostic constants P001–P010 to `diagnostics.rs`
4. Add `pub mod plugin` to `lib.rs`
5. Add `validate-plugin` subcommand to `main.rs` (initially runs only
   manifest validation)
6. Tests: valid manifest, missing fields, invalid name, bad version,
   credential detection, absolute paths, JSON format output

**Files:**
- New: `src/plugin/mod.rs`, `src/plugin/manifest.rs`
- Modified: `src/diagnostics.rs`, `src/lib.rs`, `src/main.rs`

### Wave 2: Component Validators (#97, #98, #100)

Three independent validators, one per component type. Can be developed
in parallel since they share no code beyond the existing `parse_frontmatter`
and `Diagnostic` infrastructure.

**Agent A — Hook validation (#97)**

- New: `src/plugin/hooks.rs`
- Add constants H001–H010 to `diagnostics.rs`
- Implement `validate_hooks(path) -> Vec<Diagnostic>`
- Wire into `validate-plugin` command
- Tests: valid hooks, unknown events, missing fields, bad types, timeout
  range, absolute paths, prompt on wrong event

**Agent B — Agent file validation (#98)**

- New: `src/plugin/agent.rs`
- Add constants A001–A010 to `diagnostics.rs`
- Implement `validate_agent(path) -> Vec<Diagnostic>`
- Reuse `parse_frontmatter` for YAML extraction
- Wire into `validate-plugin` command
- Tests: valid agent, missing frontmatter, missing required fields, invalid
  model/color, generic name, short system prompt, long system prompt

**Agent C — Command file validation (#100)**

- New: `src/plugin/command.rs`
- Add constants K001–K007 to `diagnostics.rs`
- Implement `validate_command(path) -> Vec<Diagnostic>`
- Reuse `parse_frontmatter` for optional YAML extraction
- Wire into `validate-plugin` command
- Tests: valid command (with and without frontmatter), invalid YAML,
  long description, invalid model, empty body

### Wave 3: Cross-Component + CLI Polish (#101, #110, #112, #113)

Depends on Wave 2 (needs all per-component validators for discovery).

**Agent D — Cross-component checks (#101)**

- New: `src/plugin/cross.rs`
- Add constants X001–X006 to `diagnostics.rs`
- Implement `validate_cross_component(root, manifest, components) -> Vec<Diagnostic>`
- Integrate into `validate-plugin` as the final validation step
- Tests: missing declared paths, orphaned files, duplicate names across
  types, hook script references

**Agent E — CLI polish (#110, #112, #113)**

- `#110`: Add `ok` output to `run_validate`, `run_check`, `run_fmt` in
  `main.rs` when zero diagnostics in single-dir text mode
- `#112`: Add `AigentError::AlreadyExists` to `errors.rs`, update
  `builder/mod.rs` and display formatting
- `#113`: In `run_probe` in `main.rs`, calculate max label width and
  align all values; handle multiline description wrapping

Files modified: `src/main.rs`, `src/errors.rs`, `src/builder/mod.rs`

### Wave 4: Enhancements (#104, #111)

Independent improvements that don't affect the core validation pipeline.

**Agent F — Test runner strength (#104)**

- Modified: `src/test_runner.rs`, `src/tester.rs`
- Add `MatchStrength` enum, `strength` field to `TestQuery`
- Update assertion logic and `--generate` output
- Tests: strong/weak/none assertions, precedence with `min_score`

**Agent G — Scaffolding (#111)**

- Modified: `src/builder/mod.rs`, `src/main.rs`
- Add `--minimal` flag to `init` and `new` commands
- Create `examples/` and `scripts/` with `.gitkeep` after SKILL.md
- Tests: default scaffolding creates dirs, `--minimal` skips them

---

## Issue Summary

| Wave | Issue | Description | Complexity | Category |
|------|-------|-------------|------------|----------|
| 1 | #99 | Plugin manifest validation | Medium | Validator |
| 2 | #97 | Hook validation | Medium | Validator |
| 2 | #98 | Agent file validation | Medium | Validator |
| 2 | #100 | Command file validation | Low | Validator |
| 3 | #101 | Cross-component consistency | High | Validator |
| 3 | #110 | Success confirmation message | Low | CLI |
| 3 | #112 | Misleading "build error" message | Low | CLI |
| 3 | #113 | Probe output alignment | Low | CLI |
| 4 | #104 | Test runner strength assertion | Low | Test |
| 4 | #111 | Scaffold supporting directories | Low | Builder |

## Risk Assessment

- **#99 is the keystone**: Everything depends on `PluginManifest` parsing.
  If the manifest schema changes upstream (Claude Code updates), the model
  needs updating. Mitigation: parse permissively with `serde_json::Value`
  fallback for unknown fields, validate only fields we recognize.

- **#101 (cross-component) is the most complex**: It requires all four
  per-component validators to be complete and produces compound diagnostics.
  Risk of false positives in orphan detection (files that are intentionally
  present but not referenced). Mitigation: conservative detection — only
  flag files in recognized component directories, exclude common non-component
  files.

- **Diagnostic code conflicts with M14**: If M14 adds codes in the same
  session, there could be merge conflicts in `diagnostics.rs`. Mitigation:
  M15 uses entirely new code prefixes (P, H, A, K, X) that don't overlap
  with M14's changes to existing prefixes.

- **Claude Code spec stability**: The plugin format is not formally
  specified — rules are derived from plugin-dev's skills and scripts. If
  Claude Code changes its conventions, validators may produce false
  diagnostics. Mitigation: keep rules conservative (warn rather than error
  for anything not explicitly required).

- **API surface growth**: Adding `src/plugin/` increases the public API
  significantly. Risk of inconsistency with existing patterns. Mitigation:
  follow existing patterns exactly (same return types, same diagnostic
  builder pattern, same testing style).

## Estimated Scope

| Metric | Estimate |
|--------|----------|
| New files | 6 (`src/plugin/{mod,manifest,hooks,agent,command,cross}.rs`) |
| Modified files | 5 (`diagnostics.rs`, `lib.rs`, `main.rs`, `errors.rs`, `builder/mod.rs`) + 2 enhancement files |
| New diagnostic codes | ~43 (P: 10, H: 10, A: 10, K: 7, X: 6) |
| New tests | ~80–100 |
| Net line delta | +1500–2000 |
| New dependencies | 0 |

---

## v2: Reconciled Plan (post-review amendments)

Review document: `dev/m15/review.md`. All code locations verified accurate
(with one correction: I001–I005 are `pub const` in `linter.rs`, not string
literals). Validation rules cross-checked against `dev/plugin-dev.md`.

### Baseline Refresh (addendum finding 2)

v1 baseline was `2c2309d` (M13). Current `main` is `3e730ab` (v0.5.0).
M14 and several CLI/UX PRs have merged since:

```
3e730ab Bump version to 0.5.0
c9fede3 Move build matrix table from CI to release section in README (#127)
0ea6754 Add release subcommand to version.sh (#126)
0e36bcb Default to current directory when no skill path is given (#125)
d503c87 Fix version.sh: case-sensitive heading match (#123)
2e3340b Show diff in format --check output (#122)
8884129 Add properties as primary command, keep read-properties as alias (#121)
6ff3b05 Bump version to 0.4.1
5feaa2f M15: README — agent skills introduction and project description (#107)
d912310 M14: SRE Review — security, reliability, performance hardening (#109)
```

**Updated baseline**: `3e730ab`, 561 tests (413 + 120 + 27 + 1), ~21,856
lines. M14 dependency is **resolved** (merged). `AigentError` has 5
variants (Parse, Validation, Io, Yaml, Build) — no `AlreadyExists` yet.
Lint codes I001–I005 are `pub const` in `linter.rs` (same pattern as
diagnostics.rs constants).

**M14 merge-risk note from v1 is obsolete** — M14 is already on `main`.
No merge-risk coordination needed.

### R1. P007 / X001 Duplicate (§3.1)

**Problem**: P007 ("declared component path does not exist") and X001
("manifest declares path that doesn't exist") check the same condition.

**Resolution**: Keep P007 in the manifest validator (it checks that paths
declared in `plugin.json` resolve to existing directories). **Redefine
X001** to check a related but distinct condition: "declared component
directory exists but contains no valid component files" (e.g., `skills/`
exists but has no `SKILL.md` inside any subdirectory). This is a useful
cross-component check that P007 cannot perform.

Updated X001: "Component directory is empty (no valid files found)."

### R2. Plugin Root Path Resolution (§3.2)

**Problem**: The relationship between `<plugin-dir>` argument and
`plugin.json` was undefined.

**Resolution**: Define explicit resolution order:

1. If `<plugin-dir>/plugin.json` exists → use it (flat layout)
2. Else if `<plugin-dir>/.claude-plugin/plugin.json` exists → use it
   (assembled/installed layout)
3. Else → error: "No plugin.json found at `<dir>` or
   `<dir>/.claude-plugin/`. Use `aigent validate` for skill-only
   validation."

The **plugin root** (for resolving component paths) is the directory
*containing* `plugin.json`, not the input directory. This is important
because in layout (2), component directories like `skills/` are siblings
of `.claude-plugin/`, i.e., they live in `<plugin-dir>/skills/`, not
inside `.claude-plugin/`.

**Correction**: For layout (2), the plugin root is `<plugin-dir>` (the
parent of `.claude-plugin/`), since that's where component directories
sit. The manifest at `.claude-plugin/plugin.json` is just metadata.

### R3. Manifest Path Overrides — Confirmed Valid (§3.3)

**Problem**: The plan assumed Claude Code might not support custom
component path declarations in `plugin.json`.

**Resolution**: The Claude Code plugin spec **does** support path
override fields in `plugin.json`:

- `commands` — path to custom commands directory
- `agents` — path to agent definitions
- `skills` — path to skill components
- `hooks` — path to hooks configuration
- `mcpServers` — path to MCP server configuration
- `outputStyles` — path to custom output styles
- `lspServers` — path to LSP server configuration

These supplement (not replace) the default directories. P006 and P007
are valid checks. Update the `PluginManifest` struct:

```rust
#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<AuthorField>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Option<Vec<String>>,
    // Component path overrides (supplement defaults)
    pub commands: Option<String>,
    pub agents: Option<String>,
    pub skills: Option<String>,
    pub hooks: Option<String>,
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Option<String>,
    #[serde(rename = "outputStyles")]
    pub output_styles: Option<String>,
    #[serde(rename = "lspServers")]
    pub lsp_servers: Option<String>,
}
```

**Component discovery algorithm**: For each component type, collect
directories from:
1. The default location (`<root>/skills/`, `<root>/hooks/`, etc.)
2. Any custom path declared in `plugin.json`

Both are scanned if present.

### R4. HooksFile Parse Error Distinction (§3.4)

**Problem**: `serde_json` parse errors don't distinguish "not JSON" from
"JSON but wrong shape."

**Resolution**: Split H001 into two:
- **H001**: "Invalid JSON syntax" — `serde_json::from_str::<Value>` fails
- **H002** (renumbered): "Invalid hooks structure (expected object mapping
  event names to arrays)" — JSON parses but doesn't match
  `HashMap<String, Vec<HookEntry>>`

Implementation: first parse as `serde_json::Value` to check syntax, then
deserialize into the typed struct. If the first fails → H001. If the
second fails → H002. Original H002–H010 shift to H003–H011.

**Updated hook diagnostic codes:**

| Code | Severity | Rule |
|------|----------|------|
| H001 | Error | Invalid JSON syntax |
| H002 | Error | Invalid structure (not an object of event arrays) |
| H003 | Error | Unknown event name |
| H004 | Error | Hook entry missing `hooks` array |
| H005 | Error | Hook missing `type` field |
| H006 | Error | Unknown hook type (not `command` or `prompt`) |
| H007 | Error | Command hook missing `command` field |
| H008 | Error | Prompt hook missing `prompt` field |
| H009 | Warning | Timeout outside recommended range (5–600s) |
| H010 | Warning | Hardcoded absolute path |
| H011 | Info | Prompt hook on suboptimal event |

### R5. `validate-plugin` Feature Flags (§3.5)

**Problem**: Plan didn't specify whether `--watch` or `--dry-run` apply
to `validate-plugin`.

**Resolution**: `validate-plugin` supports `--format text|json` only.

- **`--watch`**: Not supported initially. Plugin-wide validation is heavier
  than single-skill validation and involves multiple file types. Watching
  all component directories for changes adds complexity for unclear benefit.
  Can be revisited in a future milestone.
- **`--dry-run`**: Not applicable — validation is read-only.
- **`--recursive`**: Not applicable — plugin-wide validation already
  discovers all components recursively from the plugin root.
- **`--apply-fixes`**: Not supported initially (future milestone).

### R6. Edge Case: Empty Component Directories (§4.1)

**Resolution**: Do **not** warn about empty component directories. They
are valid in scaffolded plugins (e.g., `aigent build` creates `agents/`
and `hooks/` even if none exist). The redefined X001 ("component
directory is empty") should be **Info** severity, not Warning or Error.

### R7. Edge Case: Missing `plugin.json` (§4.2)

**Resolution**: Handled by the path resolution in R2. If neither
`<dir>/plugin.json` nor `<dir>/.claude-plugin/plugin.json` exists, print
a clear error to stderr and exit 1:

```
error: no plugin.json found at <dir> or <dir>/.claude-plugin/
       Use `aigent validate` for skill-only validation.
```

### R8. Edge Case: Mixed Diagnostic Codes (§4.3)

**Resolution**: Yes, `validate-plugin` runs the existing skill validator
on discovered skills. Output mixes E/W/S codes (skill) with P/H/A/K/X
codes (plugin ecosystem). This is intentional and useful — it gives a
complete picture of the plugin's health. JSON output groups diagnostics
by component path, making the source clear:

```json
[
  { "path": "plugin.json", "diagnostics": [{"code": "P003", ...}] },
  { "path": "skills/my-skill", "diagnostics": [{"code": "E001", ...}] },
  { "path": "hooks/hooks.json", "diagnostics": [{"code": "H003", ...}] },
  { "path": "<cross-component>", "diagnostics": [{"code": "X001", ...}] }
]
```

### R9. Command Validator: `parse_optional_frontmatter` (addendum finding 1)

**Problem**: Command `.md` files may have no frontmatter at all (no `---`
delimiters). Current `parse_frontmatter` hard-requires leading `---` and
errors otherwise (`parser.rs:66–71`). The command validator can't reuse
it directly.

**Resolution**: Add a `parse_optional_frontmatter` helper to `parser.rs`:

```rust
/// Parse optional YAML frontmatter from markdown content.
///
/// If the content starts with `---`, delegates to `parse_frontmatter`.
/// Otherwise, returns an empty metadata map and the full content as body.
pub fn parse_optional_frontmatter(content: &str) -> Result<(HashMap<String, Value>, String)> {
    if content.trim_start().starts_with("---") {
        parse_frontmatter(content)
    } else {
        Ok((HashMap::new(), content.to_string()))
    }
}
```

This is a thin wrapper. The agent validator continues to use
`parse_frontmatter` directly (agents require frontmatter → A001 error
if missing). The command validator uses `parse_optional_frontmatter`
(commands have optional frontmatter → K001 only fires on malformed YAML
when `---` is present).

**Wave impact**: Add this helper to Wave 1 (shared infrastructure) so
it's available for Wave 2's command validator.

### R10. Scaffolding: Template Awareness (addendum finding 3)

**Problem**: The plan proposes always creating `examples/.gitkeep` and
`scripts/.gitkeep`, but templates already generate concrete files in
those locations (`scripts/run.sh` for CodeSkill, `EXAMPLES.md` for
ReferenceGuide, `reference/domain.md` for DomainSpecific).

**Resolution**: Only create `.gitkeep` scaffolding when the target
directory does not already exist or is empty. The logic becomes:

```rust
if !minimal {
    let examples_dir = dir.join("examples");
    if !examples_dir.exists() {
        fs::create_dir_all(&examples_dir)?;
        fs::write(examples_dir.join(".gitkeep"), "")?;
    }
    let scripts_dir = dir.join("scripts");
    if !scripts_dir.exists() {
        fs::create_dir_all(&scripts_dir)?;
        fs::write(scripts_dir.join(".gitkeep"), "")?;
    }
}
```

This runs **after** template files are written, so template-generated
files (`scripts/run.sh`, etc.) take precedence. `.gitkeep` is only added
to directories the template didn't populate.

**Scope**: This only applies to `init` and `new --no-llm`. The LLM
builder already handles `extra_files` independently.

### R11. Probe Alignment: No `run_probe` Function (addendum finding 4)

**Problem**: Plan references `run_probe` function in `main.rs` but probe
logic is inline in the `Commands::Probe` match arm (`main.rs:747+`).

**Resolution**: Implementation can either:
- Extract probe output into a `run_probe` helper (cleaner, consistent with
  other commands) and apply alignment there
- Apply alignment inline in the existing match arm

Either approach works. The plan's intent is clear regardless of whether a
helper is extracted. Agent E should decide based on code clarity.

### Updated Wave 1 Tasks

Wave 1 gains one additional task from R9:

1. Create `src/plugin/mod.rs` with `PluginComponent` enum and re-exports
2. Create `src/plugin/manifest.rs` with `PluginManifest` struct (updated
   per R3) and `validate_manifest(path) -> Vec<Diagnostic>`
3. Add `parse_optional_frontmatter` to `src/parser.rs` (R9)
4. Add diagnostic constants P001–P010 to `diagnostics.rs`
5. Add `pub mod plugin` to `lib.rs`
6. Add `validate-plugin` subcommand to `main.rs` with path resolution
   per R2 and `--format text|json` (no `--watch` per R5)
7. Tests: valid manifest, missing fields, invalid name, bad version,
   credential detection, absolute paths, path overrides, JSON output,
   path resolution (both layouts), missing plugin.json error

### Updated Diagnostic Codes

| Prefix | Range | Count | Notes |
|--------|-------|------:|-------|
| P | P001–P010 | 10 | Unchanged |
| H | H001–H011 | 11 | +1 from R4 (split H001/H002) |
| A | A001–A010 | 10 | Unchanged |
| K | K001–K007 | 7 | Unchanged |
| X | X001–X006 | 6 | X001 redefined per R1 (empty dir, Info severity) |
| **Total** | | **44** | Was 43 |

### Updated Scope

| Metric | v1 | v2 | Delta |
|--------|----|----|-------|
| New files | 6 | 6 | — |
| Modified files | 5–7 | 6–8 | +1 (parser.rs for `parse_optional_frontmatter`) |
| New diagnostic codes | 43 | 44 | +1 |
| New tests | 80–100 | 85–105 | +5 (path resolution, optional frontmatter) |
| Net line delta | +1500–2000 | +1600–2100 | +100 |
| New dependencies | 0 | 0 | — |

### Review Findings Addressed

| Review item | Resolution |
|-------------|------------|
| §3.1 P007/X001 duplicate | R1: X001 redefined as "empty component dir" (Info) |
| §3.2 Plugin root path unclear | R2: Resolution order defined (flat → `.claude-plugin/` → error) |
| §3.3 Manifest path overrides | R3: Confirmed — Claude Code supports 7 path override fields |
| §3.4 HooksFile parse distinction | R4: Split H001/H002 (syntax vs shape), codes renumbered |
| §3.5 --watch not mentioned | R5: Not supported initially, documented |
| §4.1 Empty component dirs | R6: Don't warn (valid for scaffolded plugins), X001 = Info |
| §4.2 No plugin.json | R7: Clear error with suggestion to use `aigent validate` |
| §4.3 Mixed diagnostic codes | R8: Intentional, JSON groups by component path |
| §4.4 MatchStrength boundaries | Confirmed: ≥ boundaries (0.3 = Weak, 0.6 = Strong) |
| Addendum 1: parse_frontmatter | R9: Add `parse_optional_frontmatter` wrapper |
| Addendum 2: Stale baseline | Baseline refresh: `3e730ab`, M14 merged |
| Addendum 3: Scaffolding conflict | R10: Only `.gitkeep` when template didn't populate |
| Addendum 4: `run_probe` missing | R11: Acknowledged, agent E decides on extraction |

---

## PR Review: Reconciled Feedback (Wave 5)

Sources:
- `dev/m15/review.md` — Branch review findings (3 items from first review, 5 from second)
- PR #130 — 13 Copilot review comments across 2 review rounds

### Reconciled Issues

**Must fix (correctness):**

| # | Source | File | Issue | Fix |
|---|--------|------|-------|-----|
| F1 | Review #1.3 + Copilot #13 | `cross.rs` | Skills discovery treats `skills/` as flat `.md` files but skills are subdirectories (`skills/<name>/SKILL.md`). X001 false positives, X004/X006 miss skills. | Handle `skills/` separately: iterate subdirs, use dir name as component name, check for `SKILL.md` inside each subdir |
| F2 | Review #1.2 + #2.3 + Copilot #11 | `main.rs` | `validate-plugin` doesn't validate skills | Discover skill subdirs under `skills/` and run existing `aigent::validate` on each |
| F3 | Copilot #4 + #8 | `parser.rs` | `parse_optional_frontmatter` uses `trim_start()` but `parse_frontmatter` requires `---` at byte 0. Leading whitespace causes mismatch. | Change to `content.starts_with("---")` |
| F4 | Copilot #9 | `command.rs` | Empty frontmatter (`---\n---`) parsed as no-frontmatter because `metadata.is_empty()`. K007 skipped. | Check `content.starts_with("---")` directly for presence |
| F5 | Copilot #5 + #7 | `manifest.rs` | P006 only detects Unix absolute paths (`/`). Should use `Path::is_absolute()`. | Replace `value.starts_with('/')` with `Path::new(value).is_absolute()` |
| F6 | Copilot #2 | `manifest.rs` | P009 URL scheme check is case-sensitive. RFC 3986 says schemes are case-insensitive. | Lowercase before comparison |

**Should fix (quality):**

| # | Source | File | Issue | Fix |
|---|--------|------|-------|-----|
| F7 | Copilot #1 | `manifest.rs` | P008 doesn't report JSON path of credential | Track path segments during recursive walk |
| F8 | Copilot #6 | `manifest.rs` | `path_overrides()` redundant `PATH_OVERRIDE_FIELDS` lookup | Simplify: return `(name, v)` directly |
| F9 | Copilot #10 | `tester.rs` | Comment says "Tokens at 6 chars" but widest label is "Description:" at 12 | Fix comment text |
| F10 | Copilot #3 | `manifest.rs` | Missing `ws://` test for P009 | Add unit test |

**Declined:**

| # | Source | Reason |
|---|--------|--------|
| D1 | Review #1.1 | `validate-plugin` ignoring manifest-declared paths: convention-based discovery is intentional for v1. Path overrides validated by P006/P007. Enhancement deferred. |
| D2 | Copilot #12 | `MatchStrength::None` as no-op: by design for YAML round-tripping symmetry. min_score=0.0 with should_match=true is a valid "any score accepted" semantic. |
| D3 | Review #2.1 | `KEBAB_CASE_RE` duplication: small, unlikely to drift. Not blocking. |
| D4 | Review #2.2 | Hook code renumbering: already documented in plan v2. |
| D5 | Review #2.4 | `init_skill` API change: pre-1.0 crate. Will note in CHANGES.md at release. |

### Files to Modify

| File | Changes |
|------|---------|
| `src/plugin/cross.rs` | F1: Handle skills as subdirectories |
| `src/main.rs` | F2: Add skill validation to `validate-plugin` |
| `src/parser.rs` | F3: Fix `parse_optional_frontmatter` trim check |
| `src/plugin/command.rs` | F4: Fix empty frontmatter detection |
| `src/plugin/manifest.rs` | F5: `Path::is_absolute()`, F6: case-insensitive URL, F7: JSON path in P008, F8: simplify `path_overrides()`, F10: add `ws://` test |
| `src/tester.rs` | F9: Fix comment |
| `tests/cli.rs` | Update affected CLI tests |

### Verification

```bash
cargo test                          # all tests pass
cargo clippy -- -D warnings         # no warnings
```
