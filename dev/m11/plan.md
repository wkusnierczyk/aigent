# M11: Builder & Prompt Enhancements — Work Plan

## Overview

Extends the builder and prompt generator with new capabilities that consume
the structured diagnostics infrastructure from M10. Covers template systems,
multi-format prompt output, token budgets, interactive build mode, hooks,
and diff-aware output.

Issues: #49, #55, #56, #57, #58, #60, #62, #65.

## Branch Strategy

- **Dev branch**: `dev/m11` (created from `main` after M10 merges)
- **Task branches**: `task/m11-<name>` (created from `dev/m11`)
- After each wave, task branches merge into `dev/m11`
- After all waves, PR from `dev/m11` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- **M10**: Structured `Diagnostic` type, `ValidationTarget`, `linter::lint()`,
  `--format` flag infrastructure — all M11 features consume these
- M7: `src/builder/` — deterministic + LLM modules, `SkillSpec`, `BuildResult`
- M9: `skills/`, `.claude-plugin/plugin.json`, `install.sh` — plugin packaging
- M5: `src/prompt.rs` — `to_prompt`, `xml_escape`

## Current State (after M10)

M10 introduces:

- `src/diagnostics.rs`: `Severity`, `Diagnostic`, error code registry
- `src/linter.rs`: 5 semantic lint checks (I001–I005)
- `src/fixer.rs`: auto-fix application
- Tiered `KNOWN_KEYS` with `--target` flag
- `--format text|json` on `validate`
- Batch validation with `--recursive`

M11 builds on this foundation to enhance the builder and prompt subsystems.

---

## Design Decisions

### Template System (#58)

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

- **`minimal`** (current, default): `<name>/SKILL.md`
- **`reference-guide`**: SKILL.md + REFERENCE.md + EXAMPLES.md
- **`domain-specific`**: SKILL.md + `reference/domain.md`
- **`workflow`**: SKILL.md with checklist pattern (from spec)
- **`code-skill`**: SKILL.md + `scripts/run.sh` (with shebang, error handling)
- **`claude-code`**: SKILL.md with Claude Code extension fields in frontmatter

Templates live in `src/builder/templates.rs` as functions returning
`HashMap<String, String>` (relative path → content). The `init_skill` and
`build_skill` functions accept an optional `SkillTemplate` parameter.

### Multi-Format Prompt Output (#56)

Refactor `src/prompt.rs` to first collect a `Vec<SkillEntry>` struct, then
format based on the chosen output format:

```rust
struct SkillEntry {
    name: String,
    description: String,
    location: String,
}
```

Four output formats: XML (current), JSON, YAML, Markdown. The existing
`to_prompt(dirs)` function becomes a backward-compatible wrapper.

### Token Budget Estimation (#55)

Estimation heuristic: `chars / 4` (standard English approximation). No
external dependency — `tiktoken-rs` is heavy and unnecessary for estimates.

Budget output appended after the prompt:

```
Token budget:
  aigent-builder     ~45 tokens
  aigent-validator   ~48 tokens
  ---
  Total:             ~93 tokens
  Context usage:     <0.1% of 200k
```

Threshold warning: If total exceeds 4000 tokens (~2% of 200k context),
emit a warning suggesting skill consolidation.

### Interactive Build Mode (#60)

Add `--interactive` / `-i` flag to `build`. Interactive mode flow:

1. Assess clarity — if unclear, print questions and exit
2. Derive name — print "Name: {name}" and confirm
3. Generate description — print and confirm
4. Generate body preview — print first 20 lines
5. Confirm write
6. Validate and report

This surfaces the existing `ClarityAssessment` struct, which currently has
no user-facing CLI exposure. Read confirmation from stdin.

### Hooks for Continuous Validation (#62)

Add `hooks/hooks.json` to the plugin with a `PostToolUse` hook:

1. Triggers on `Write|Edit` tool uses
2. Extracts the file path from tool input JSON
3. Checks if the file is a SKILL.md
4. If `aigent` is available, runs validation on the parent directory
5. Always succeeds (trailing `|| true`) — validation failures are informational

### Diff-Aware Prompt Updates (#57)

Add `--output <file>` flag to `to-prompt`. When specified:

1. Write prompt output to file instead of stdout
2. If file exists, compare and report whether content changed
3. Exit code 0 if unchanged, 1 if changed (for CI pipelines)

### context:fork for Builder (#65)

Update `skills/aigent-builder/SKILL.md` frontmatter to add `context: fork`.
Enables the builder to analyze the user's existing codebase before generating,
avoiding conflicts with existing skills.

### Checksum Verification (#49)

Update `install.sh` and `.github/workflows/release.yml`:

1. Release workflow computes SHA256 for each archive, uploads `checksums.txt`
2. Install script downloads `checksums.txt`, verifies downloaded archive
3. Fails with clear error on mismatch

---

## Wave 1 — Templates + Prompt Enhancements

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m11-templates` | #58 | Template system for init/build |
| B | `task/m11-prompt` | #55, #56 | Token budget + multi-format prompt |
| C | `task/m11-checksum` | #49 | Checksum verification for install script |

**Merge**: A, B, C can all run in parallel. Merge order: any.

### Agent A — Template System (#58)

1. Create `src/builder/templates.rs` (replace existing `template.rs`):
   - `pub fn template_files(template: SkillTemplate, name: &str) -> HashMap<String, String>`
   - Each template returns relative path → content pairs
   - `SkillTemplate` enum with 6 variants

2. Update `src/builder/mod.rs`:
   - `init_skill` accepts optional `SkillTemplate` parameter
   - `build_skill` accepts template via `SkillSpec`

3. Update `src/main.rs`:
   - Add `--template` flag to `Init` and `Build` subcommands

4. Add `SkillTemplate` and `template` field to `SkillSpec` struct

5. Tests:
   - Each template generates expected file set
   - Minimal template matches current behavior
   - Generated files pass validation
   - Template names match `SkillTemplate` variants

### Agent B — Token Budget + Multi-Format Prompt (#55, #56)

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

### Agent C — Checksum Verification (#49)

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

## Wave 2 — Interactive Build + Plugin Depth

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| D | `task/m11-interactive` | #60 | Add `--interactive` flag to build |
| E | `task/m11-hooks` | #62 | Add PostToolUse hooks for SKILL.md validation |
| F | `task/m11-fork` | #65 | Add context:fork to builder skill |
| G | `task/m11-diff` | #57 | Add `--output <file>` to to-prompt |

**Merge**: D, E, F, G can all run in parallel. Merge order: any.

### Agent D — Interactive Build (#60)

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

### Agent E — Hooks (#62)

1. Create `hooks/hooks.json` with PostToolUse hook for Write|Edit
2. Hook detects SKILL.md writes and runs `aigent validate`
3. Test: `hooks/hooks.json` is valid JSON, matches expected structure

### Agent F — context:fork (#65)

1. Update `skills/aigent-builder/SKILL.md` frontmatter:
   - Add `context: fork`
2. Verify skill still passes validation with `--target claude-code`
3. Existing `--target standard` tests will flag `context` as W001
   warning (expected; plugin tests should use `--target claude-code`)

### Agent G — Diff-Aware Prompt Output (#57)

1. Add `--output <file>` flag to `ToPrompt`
2. Write prompt output to file instead of stdout
3. If file exists, compare content and report changes
4. Exit code: 0 unchanged, 1 changed

5. Tests:
   - `--output` writes file correctly
   - Second run with same input produces no change
   - Changed input produces exit code 1
   - Works with all `--format` options

---

## Wave 3 — Verify

Single agent runs the full check suite on `dev/m11`.

| Agent | Branch | Task |
|-------|--------|------|
| H | `dev/m11` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Cargo.toml Changes

No new dependencies required. All features use existing deps
(`serde`, `serde_json`, `serde_yaml_ng`, `regex`, `clap`).

---

## New Module Map

After M11, changes from M10 baseline:

```
src/
├── prompt.rs           # UPDATED: SkillEntry, multi-format, token budget
├── builder/
│   ├── mod.rs          # UPDATED: template + interactive support
│   └── templates.rs    # UPDATED: 6 template variants (was single template)
└── main.rs             # UPDATED: new flags (--template, --format, --budget,
                        #   --interactive, --output)
```

Plugin changes:

```
hooks/
└── hooks.json          # NEW: PostToolUse SKILL.md validation
skills/
└── aigent-builder/
    └── SKILL.md        # UPDATED: context: fork
install.sh              # UPDATED: checksum verification
.github/workflows/
└── release.yml         # UPDATED: SHA256 checksum generation
```

---

## New CLI Surface

After M11 (additions to M10 surface):

```
aigent to-prompt <dirs...> [--format xml|json|yaml|markdown] [--budget] [--output <file>]
aigent build <purpose> [--name] [--dir] [--no-llm] [--interactive] [--template <template>]
aigent init [dir] [--template <template>]
```

---

## Deliverables

- `src/builder/templates.rs` — 6 template variants
- `src/prompt.rs` — `SkillEntry`, multi-format output, token budget
- `src/main.rs` — new flags (`--template`, `--format` for to-prompt,
  `--budget`, `--output`, `--interactive`)
- `hooks/hooks.json` — PostToolUse SKILL.md validation hook
- `skills/aigent-builder/SKILL.md` — updated with `context: fork`
- `install.sh` — updated with checksum verification
- `.github/workflows/release.yml` — SHA256 checksum generation
- Updated tests across all modules
- PR: `M11: Builder & Prompt Enhancements`

---

## Reconciliation (2026-02-20)

### Dependencies Verified

All M10 deliverables consumed by M11 are in place:

- `Diagnostic`, `Severity`, `ValidationTarget` → `src/diagnostics.rs`
- `linter::lint()` → `src/linter.rs`
- `apply_fixes()` → `src/fixer.rs`
- `--format text|json`, `--recursive` → `src/main.rs`
- `CLAUDE_CODE_KEYS` → `src/parser.rs` (moved from validator during Copilot review)
- `E000`, `E008` registered as constants (added during Copilot review)

### Stale Assumptions

1. Plan §Current State says `CLAUDE_CODE_KEYS` is in `validator.rs` — it was
   moved to `parser.rs` during M10 Copilot review. No impact on M11.
2. Plan §Current State omits E000/E008 registration — done in M10. No impact.

### M10 Review Findings Deferred to M11

| Finding | M11 Coverage | Resolution |
|---------|-------------|------------|
| F3: `--apply-fixes` dry-run | No M11 issue | **Defer to M12** — tracked by #61 (upgrade) |
| F5: Hook `jq` dependency, duplicate `.file_path` | Agent E (#62) | **Address during implementation** |
| F7: Token budget `chars/4` inaccuracy | Agent B (#55) | **Document limitations in budget output** |
| F8: `template.rs` rename | Agent A (#58) | **Keep filename `template.rs`**, extend in place |
| F11: Interactive stdin testability | Agent D (#60) | **Use `BufRead` trait injection** |

### Ordering Fix

Agent G (#57, diff-aware output) depends on Agent B (#55/#56, multi-format):
G's test spec requires "Works with all `--format` options." Move G to run
after Wave 1 merges, not parallel with Wave 2.

Revised Wave 2:

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| D | `task/m11-interactive` | #60 | Interactive build mode |
| E | `task/m11-hooks` | #62 | PostToolUse hooks |
| F | `task/m11-fork` | #65 | context:fork for builder skill |

Wave 2.5 (after Wave 1 + Wave 2 merge):

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| G | `task/m11-diff` | #57 | Diff-aware `--output` for to-prompt |

### Additional Implementation Notes

1. **`lib.rs` re-exports**: Add `SkillEntry`, `SkillTemplate`,
   `estimate_tokens`, `to_prompt_format` as they are implemented.
2. **Plugin manifest**: Verify `.claude-plugin/plugin.json` auto-discovers
   `hooks/` or update manifest to reference `hooks/hooks.json`.
3. **`to_kebab_case()` overlap**: Current `template.rs` has `to_kebab_case()`,
   `builder/mod.rs` has `derive_name()`. Consolidate during Agent A if they
   overlap.
4. **CLI `--format` namespacing**: `to-prompt --format` uses
   `xml|json|yaml|markdown`; `validate --format` uses `text|json`. Different
   enum types, no conflict.
