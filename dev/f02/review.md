# F2: Product Improvement Review

## Context

Analysis of how aigent's validator, prompt generator, builder, and plugin
could be improved as a product — beyond spec compliance. Based on a scan of
the current specification landscape, Anthropic's best-practices documentation,
community patterns, and the existing codebase.

## Key Finding: The Spec Has Evolved Significantly

The Agent Skills format is now a genuine open standard (agentskills.io),
adopted by 25+ tools beyond Anthropic — including Gemini CLI, Cursor, VS Code
Copilot, OpenAI Codex, Roo Code, and Databricks. The standard defines a
portable core; Claude Code extends it with product-specific fields (`context`,
`agent`, `hooks`, `model`, etc.).

Claude Code treats `name` as optional (falls back to directory name) and
`description` as "recommended" rather than required. The open standard requires
both. This divergence is a product opportunity — aigent can validate against
both tiers.

---

## 1. Validator — From Structural to Semantic

aigent already exceeds the spec structurally (XML tag rejection, Unicode NFKC,
reserved words, directory-name match). But the biggest authoring pain point is
description quality, not format compliance. Anthropic's best-practices guide
devotes more space to "how to write a good description" than to field format
rules.

### V1. Semantic Linting / Quality Scoring (highest impact)

The spec defines mechanically-detectable quality rules:

- Descriptions must be in **third person** (not "I can help you" or "You can
  use this")
- Descriptions should include both **what it does** and **when to use it**
  (look for trigger phrases like "Use when")
- Names should prefer **gerund form** ("processing-pdfs" not "pdf-processor")
- Vague names ("helper", "utils", "tools") should be flagged
- Vague descriptions ("Helps with documents") should be flagged

A `--lint` flag or `lint` subcommand could return advisory messages with a
quality score (0-100) and categorized suggestions. This is the kind of
guidance no other tool provides today.

### V2. Claude Code Extension Field Awareness

aigent currently warns on all unknown metadata keys — but Claude Code adds 8+
legitimate fields (`disable-model-invocation`, `user-invocable`, `context`,
`agent`, `model`, `hooks`, `argument-hint`, `allowed-tools` expansion). A
`--target standard|claude-code|permissive` flag would suppress false warnings
for the primary audience.

### V3. Batch Validation with Summary Report

`aigent validate` takes a single directory. Users managing collections
(`.claude/skills/`, plugin `skills/` trees) need `--recursive` to discover and
validate all skills at once, with a summary table and `--format json` for CI.

```
skill-one/        PASS
skill-two/        FAIL  (2 errors, 1 warning)
skill-three/      WARN  (1 warning)
---
3 skills checked: 1 passed, 1 failed, 1 warnings-only
```

### V4. Fix-It Suggestions

Current errors are descriptive but not actionable. A `--fix` flag could output
a corrected SKILL.md — truncating long names at hyphen boundaries (reusing
`truncate_at_boundary` from `deterministic.rs`), lowercasing, collapsing
consecutive hyphens, stripping XML tags, appending missing trigger phrases.

### V5. Directory Structure Validation

The spec recommends `scripts/`, `references/`, `assets/` directories and warns
against deeply nested references. The validator could optionally check that:

- Referenced files actually exist (broken link detection)
- Scripts have execute permissions
- References stay one level deep
- Total content doesn't exceed token budget estimates

---

## 2. Prompt Generator — Beyond XML Output

### P1. Token Budget Estimation

The spec emphasizes that metadata costs ~100 tokens per skill and the context
window is a "public good." A `--budget` flag on `to-prompt` could estimate
token count (chars/4 heuristic or `tiktoken-rs` integration), warn when totals
exceed reasonable thresholds (~40 skills = ~4000 tokens = ~2% of a 200k
window).

### P2. Multi-Format Output

The XML format is Claude-specific. Since the open standard is now multi-agent,
a `--format xml|json|yaml|markdown` flag would position aigent as a
cross-platform tool. JSON for non-Claude agents, markdown for human-readable
catalogs.

### P3. Diff-Aware Prompt Updates

For CI pipelines: `--output <file>` writes the XML, and comparing with
previous output reveals added/removed/changed skills. Enables workflows like:

```bash
aigent to-prompt skills/ --output .generated/prompt.xml
git diff .generated/prompt.xml
```

---

## 3. Builder — From Scaffolding to Quality

### B1. Template System / Starter Kits

The spec defines clear patterns with exact blueprints. A `--template` flag on
`init`/`build`:

| Template | Contents |
|----------|----------|
| `minimal` | Current behavior |
| `reference-guide` | SKILL.md + separate reference files (Pattern 1) |
| `domain-specific` | SKILL.md + `reference/` directory (Pattern 2) |
| `workflow` | SKILL.md with checklist pattern |
| `code-skill` | SKILL.md + `scripts/` directory with starter script |
| `claude-code` | Pre-populated Claude Code extension fields |

### B2. Skill Quality Assessment

The best-practices doc has a 20+ item checklist under "Checklist for effective
Skills." An `assess` or `score` subcommand running this checklist against
existing skills would implement the spec's "Evaluation and iteration" section
— something no tool does today.

Checklist items include:

- Is the description specific and includes key terms?
- Does the description include both "what" and "when"?
- Is SKILL.md body under 500 lines?
- Are additional details in separate files?
- Is terminology consistent throughout?
- Are examples concrete, not abstract?
- Are file references one level deep?
- Are workflows present with clear steps?

### B3. Interactive Build Mode

The existing `ClarityAssessment` struct has no user-facing CLI exposure. An
`--interactive` flag could: run clarity assessment, prompt for more detail if
unclear, show preview, ask confirmation, validate, offer iteration.

### B4. Skill Upgrade / Migration

As the spec evolves, an `upgrade` subcommand could read existing SKILL.md,
identify areas not following current best practices (missing
`metadata.version`, no trigger phrase, no `compatibility` field), and apply
fixes with `--apply`.

---

## 4. Plugin — Deeper Claude Code Integration

### PL1. Hooks for Continuous Validation

Claude Code plugins support `hooks/hooks.json` for event-driven automation. A
`PostToolUse` hook on `Write|Edit` could auto-validate SKILL.md files after
modification — "continuous validation" during development.

### PL2. Skill Scorer Plugin Skill

Beyond builder + validator, add an `aigent-scorer` skill that surfaces quality
assessment directly in Claude Code without dropping to the CLI.

### PL3. Skill Tester / Previewer

The spec describes "evaluation-driven development" but notes "There is not
currently a built-in way to run these evaluations." A skill that simulates
discovery + activation (given a test query) would be unique in the ecosystem.

### PL4. `context: fork` for Builder

The builder skill could use `context: fork` to analyze the user's codebase
before generating — reading existing skills to avoid conflicts, generating
contextually relevant content.

---

## 5. Cross-Cutting Improvements

### X1. Structured Diagnostics

The `Vec<String>` return type with ad-hoc "warning:" prefixes makes
programmatic consumption difficult. A `Diagnostic` struct with severity,
stable error codes (`E001`, `W001`), field references, source spans, and fix
suggestions would enable:

- `--format json` for CI
- Stable codes for scripting
- IDE/LSP integration path

Example:

```rust
pub struct Diagnostic {
    pub severity: Severity,     // Error, Warning, Info
    pub code: &'static str,     // "E001", "W001"
    pub message: String,
    pub field: Option<String>,  // "name", "description", etc.
    pub span: Option<Span>,     // line/column in SKILL.md
    pub suggestion: Option<String>,
}
```

### X2. Documentation Generation

A `doc` subcommand generating a markdown catalog from a skill collection —
with name, description, compatibility, and instruction summaries — for
maintaining skill libraries and plugin READMEs.

### X3. Watch Mode

A `--watch` flag on `validate` using the `notify` crate for filesystem-change
monitoring, useful during iterative skill development.

### X4. Cross-Skill Conflict Detection

For skill collections: detect description similarity, flag potential activation
conflicts (two skills triggering on the same query), estimate total token
budget, check for name collisions across scopes.

---

## Priority Ranking (impact / effort)

| # | Idea | Impact | Effort |
|---|------|--------|--------|
| 1 | V1: Semantic Linting | High | Medium |
| 2 | V2: Claude Code Field Awareness | High | Low |
| 3 | X1: Structured Diagnostics | High | Medium |
| 4 | V3: Batch Validation | High | Low |
| 5 | B1: Template System | High | Medium |
| 6 | P1: Token Budget Estimation | Medium | Low |
| 7 | V4: Fix-It Suggestions | Medium | Medium |
| 8 | PL1: Hooks Integration | Medium | Low |
| 9 | B2: Quality Assessment | Medium | Medium |
| 10 | B3: Interactive Build | Medium | Low |
| 11 | PL3: Skill Tester | Medium | High |
| 12 | P2: Multi-Format Output | Medium | Medium |
| 13 | B5: Cross-Skill Conflict Detection | Medium | High |
| 14 | X3: Watch Mode | Low | Low |
| 15 | X2: Documentation Generation | Low | Medium |

---

## Key Observations

1. **The biggest product gap is semantic over structural.** aigent already
   validates format rules better than the Python reference implementation. But
   no tool in the ecosystem addresses description quality — the single most
   important factor in skill discovery. The spec says: "Your description must
   provide enough detail for Claude to know when to select this Skill from
   potentially 100+ available Skills."

2. **The open standard creates a distribution opportunity.** With 25+ tools
   adopting agent skills, aigent could become the reference Rust implementation
   — the `skills-ref` equivalent in a compiled, fast, cross-platform binary.
   Multi-format output and strict spec compliance tracking would reinforce this
   positioning.

3. **Evaluation-driven development is the spec's recommended workflow** but no
   tooling supports it today. The spec shows an evaluation JSON format but
   notes "There is not currently a built-in way to run these evaluations."
   This is a greenfield opportunity.

4. **Claude Code's extensions create a two-tier ecosystem.** The open standard
   defines a portable core. Claude Code adds product-specific fields. aigent
   needs a strategy for both tiers — strict open-standard validation for
   portability, and Claude Code-aware validation for the primary audience.

5. **Progressive disclosure is the architectural principle** that separates
   good skills from bad ones. Tooling that helps authors structure content
   across files would add significant value.

---

## Sources

- [Agent Skills Specification](https://agentskills.io/specification)
- [Skill Authoring Best Practices — Anthropic](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
- [Extend Claude with Skills — Claude Code Docs](https://code.claude.com/docs/en/skills)
- [Equipping Agents for the Real World — Anthropic Engineering](https://claude.com/blog/equipping-agents-for-the-real-world-with-agent-skills)
- [Anthropic Skills Repository](https://github.com/anthropics/skills)
- [Skill Creator SKILL.md](https://github.com/anthropics/skills/blob/main/skills/skill-creator/SKILL.md)
- [Agent Skills Standard: Quality Contract — Benjamin Abt](https://benjamin-abt.com/blog/2026/02/12/agent-skills-standard-github-copilot/)
- [Claude Agent Skills: First Principles Deep Dive](https://leehanchung.github.io/blogs/2025/10/26/claude-skills-deep-dive/)
- [Top 10 Claude Code Plugins — Firecrawl](https://www.firecrawl.dev/blog/best-claude-code-plugins)
- [Claude Code Plugins & Agent Skills Community Registry](https://claude-plugins.dev/)
