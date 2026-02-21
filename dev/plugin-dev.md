# aigent vs. plugin-dev: Comparison

## Overview

**aigent** and **plugin-dev** both support skill development for Claude Code,
but they occupy different layers of the ecosystem.

| | **aigent** | **plugin-dev** |
|---|---|---|
| Nature | Rust library + CLI + Claude Code plugin | Claude Code plugin (skills + agents + scripts) |
| Execution | Deterministic code — parses, validates, scores, formats | LLM-guided — Claude reads skills and follows methodology |
| Scope | Deep: focused on SKILL.md files | Broad: entire plugin ecosystem (7 component types) |
| Author | [wkusnierczyk/aigent](https://github.com/wkusnierczyk/aigent) | Anthropic (official, bundled with Claude Code) |

### Analogy

aigent is to plugin-dev as **rustfmt + clippy** is to **The Rust Book**.

One enforces rules mechanically; the other teaches you how to think about the
domain. You need both: the book to learn the patterns, the tools to enforce
them at scale.

## What plugin-dev Provides

plugin-dev is a comprehensive teaching toolkit for the Claude Code plugin
ecosystem. It ships as 7 skills, 3 agents, 1 guided workflow command, and
utility scripts.

### Skills (7)

| Skill | Content | Purpose |
|-------|---------|---------|
| `skill-development` | 1,232 words + references | Skill creation methodology (6-step process, progressive disclosure) |
| `plugin-structure` | 1,619 words + 2 references | Plugin directory layout, manifest format, auto-discovery |
| `hook-development` | 1,619 words + 3 references + 3 examples + 3 scripts | Hook types (prompt/command), 9 events, security patterns |
| `agent-development` | 1,438 words + 2 examples + 3 references + 1 script | Agent file structure, system prompts, triggering |
| `command-development` | 1,535 words + 5 references + 10 examples | Slash commands, frontmatter, dynamic arguments |
| `mcp-integration` | 1,666 words + 3 references + 3 examples | MCP server types (stdio/SSE/HTTP/WS), auth patterns |
| `plugin-settings` | 1,623 words + 3 examples + 2 references + 2 scripts | Per-project config via `.local.md`, YAML frontmatter parsing |

Total: ~11,000 words of core skill content, ~10,000+ words of reference
documentation, 12+ working examples, 6 validation/testing scripts.

### Agents (3)

| Agent | Purpose |
|-------|---------|
| `plugin-validator` | Validates plugin structure, manifest, component files, naming conventions |
| `skill-reviewer` | Reviews skill quality, trigger phrases, progressive disclosure |
| `agent-creator` | AI-assisted agent generation using proven prompts from Claude Code's architecture |

### Command (1)

`/create-plugin` — an 8-phase guided workflow:
Discovery → Component Planning → Detailed Design → Structure Creation →
Component Implementation → Validation → Testing → Documentation.

### Utility Scripts

- `validate-hook-schema.sh`, `test-hook.sh`, `hook-linter.sh` (hook validation)
- `validate-agent.sh` (agent validation)
- `validate-settings.sh`, `parse-frontmatter.sh` (settings validation)

## Capability Comparison

### Skill Authoring

| Capability | aigent | plugin-dev |
|-----------|--------|-----------|
| Create from natural language | `aigent new` — deterministic or LLM | Guidance-based — skill-development teaches Claude |
| Template scaffolding | `aigent init` — 3 template levels | Examples in skill-development references |
| Interactive creation | `aigent new --interactive` | 8-phase `/create-plugin` workflow |
| LLM provider support | 4 providers (Anthropic, OpenAI, Google, Ollama) | Uses whatever model Claude Code is running |

### Validation

| Capability | aigent | plugin-dev |
|-----------|--------|-----------|
| Spec conformance | `aigent validate` — typed diagnostics with error codes | plugin-validator agent — heuristic checks |
| Semantic quality | `aigent check` (validate + lint) | skill-reviewer agent |
| Quality scoring | `aigent score` — weighted 0–100 | Not available |
| Auto-fix | `aigent validate --apply-fixes` | Not available |
| Structure validation | `aigent validate --structure` | plugin-validator covers structure |
| Cross-skill conflicts | Multi-dir validation detects collisions | Not available |
| CI integration | JSON output, non-zero exit codes | Not applicable (LLM-based) |
| Watch mode | `aigent validate --watch` | Not available |

### Formatting

| Capability | aigent | plugin-dev |
|-----------|--------|-----------|
| SKILL.md formatting | `aigent fmt` — canonical key order, idempotent | Not available |
| CI check mode | `aigent fmt --check` (exit 1 if unformatted) | Not available |

### Testing

| Capability | aigent | plugin-dev |
|-----------|--------|-----------|
| Single-query probe | `aigent probe <dir> <query>` — weighted match score | Not available |
| Fixture-based testing | `aigent test` — reads `tests.yml` | General guidance only |
| Fixture generation | `aigent test --generate` | Not available |
| Hook testing | Not available | `test-hook.sh` script |
| Agent validation | Not available | `validate-agent.sh` script |

### Assembly

| Capability | aigent | plugin-dev |
|-----------|--------|-----------|
| Skill-to-plugin assembly | `aigent build` — deterministic, scriptable | `/create-plugin` — guided, interactive |
| Plugin manifest generation | Generates `plugin.json` automatically | Teaches manifest format for manual creation |
| Validation on assembly | `aigent build --validate` | plugin-validator agent post-creation |

### Plugin Ecosystem Coverage

| Component Type | aigent | plugin-dev |
|---------------|--------|-----------|
| Skills (SKILL.md) | ✅ Full toolchain | ✅ Guidance + review |
| Commands (slash commands) | ❌ | ✅ Full coverage |
| Agents | ❌ | ✅ Full coverage + AI-assisted generation |
| Hooks | Consumer only (own `hooks.json`) | ✅ Full coverage + 3 validation scripts |
| MCP servers | ❌ | ✅ Full coverage (4 server types) |
| Plugin settings | ❌ | ✅ Full coverage |
| Plugin structure/manifest | Generated during assembly | ✅ Full coverage |

### Output Modes

| Feature | aigent | plugin-dev |
|---------|--------|-----------|
| Deterministic output | ✅ Always reproducible | ❌ LLM-dependent |
| Machine-parseable (JSON) | ✅ Most commands support `--format json` | ❌ |
| Library API | ✅ 30+ pub functions in Rust crate | ❌ |
| Multi-format prompts | ✅ XML, JSON, YAML, Markdown | ❌ |

## Where They Complement Each Other

A typical skill development workflow uses both:

```
# Learn — plugin-dev teaches the patterns
"What's the best way to structure a plugin with 3 skills and a hook?"
→ Claude reads plugin-structure, hook-development, skill-development skills

# Create — aigent generates the SKILL.md files
aigent new "Process PDF files" --no-llm
aigent new "Convert images" --no-llm

# Validate — aigent checks spec conformance + quality
aigent check skills/ --recursive

# Format — aigent normalizes formatting
aigent fmt skills/ --recursive

# Test — aigent runs activation tests
aigent test --generate skills/
aigent test skills/ --recursive

# Score — aigent rates quality
aigent score skills/processing-pdf-files

# Assemble — aigent packages into a plugin
aigent build skills/ --output ./dist --validate

# Review — plugin-dev validates the assembled plugin
→ plugin-validator agent checks plugin structure, manifest, components

# Iterate — plugin-dev's skill-reviewer catches quality issues
→ skill-reviewer agent suggests trigger phrase improvements
```

plugin-dev covers the **breadth** of the Claude Code plugin ecosystem
(7 component types, ~21,000 words of guidance). aigent covers the **depth**
of one component type (SKILL.md) with deterministic tooling that integrates
into CI/CD pipelines.

## What aigent Does That plugin-dev Cannot

1. **Deterministic validation** — typed error codes, severity levels, machine-
   parseable JSON output. Every run produces identical results.

2. **Numeric scoring** — weighted 0–100 quality score usable as a CI gate
   (`exit 1` if imperfect).

3. **Idempotent formatting** — `aigent fmt` normalizes SKILL.md files with
   canonical key ordering. `--check` mode for CI enforcement. Running twice
   produces no further changes.

4. **Fixture-based testing** — `tests.yml` files define expected activation
   patterns. Regression testing for skill descriptions.

5. **Programmatic assembly** — `aigent build` is reproducible and scriptable.
   No LLM variance between runs.

6. **Library API** — all functionality exposed as `pub fn` in a Rust crate.
   Consumers can embed validation, formatting, and assembly in their own tools.

7. **Auto-fix** — `--apply-fixes` corrects fixable issues (name casing, etc.)
   without human intervention.

8. **Cross-skill analysis** — conflict detection (name collisions, description
   similarity), token budget estimation across skill collections.

## What plugin-dev Does That aigent Cannot

1. **Covers 6 other component types** — commands, agents, hooks, MCP servers,
   settings, plugin structure. aigent only handles skills.

2. **Teaches design patterns** — not just "is this valid?" but "here's how to
   think about progressive disclosure, trigger phrases, hook security."

3. **AI-assisted generation** — agent-creator uses proven prompts from Claude
   Code's own architecture to generate agent configurations.

4. **Guided workflows** — `/create-plugin` walks through 8 phases with
   clarifying questions at each step. aigent commands are single-purpose.

5. **Real-world examples** — 12+ working examples and reference documents per
   component type. aigent has no tutorial content.

6. **Hook/agent validation scripts** — bash utilities for validating hook
   schemas, testing hooks, and validating agent files. aigent has no tooling
   for non-skill components.

## Summary

| Dimension | aigent | plugin-dev |
|-----------|--------|-----------|
| **Strength** | Depth: deterministic SKILL.md toolchain | Breadth: full plugin ecosystem guidance |
| **Execution model** | Code (Rust) | LLM (Claude reads skills) |
| **Reproducibility** | Deterministic | Varies per run |
| **CI/CD integration** | Native (exit codes, JSON, `--check`) | Not designed for CI |
| **Learning curve** | Tool documentation | Progressive guidance |
| **Ecosystem coverage** | Skills only | Skills + 6 other component types |
| **Best for** | Enforcing quality at scale | Learning and creating plugins |
