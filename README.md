<table>
  <tr>
    <td>
      <img src="https://github.com/wkusnierczyk/aigent/raw/main/graphics/aigent.png" alt="logo" width="300" />
    </td>
    <td>
      <p><strong><code>aigent</code></strong>: validate, format, score, test, and build AI agent skills.</p>
      <p>A library, CLI tool, and Claude Code plugin for managing AI agent skill definitions.</p>
      <p>Validates, parses, and generates prompts from skill metadata stored in <code>SKILL.md</code> files with YAML frontmatter. 
         Provides a skill builder for creating new skills from natural language specifications.</p>
    </td>
  </tr>
</table>

**Agent skills** are an [open standard](https://agentskills.io) for packaging
reusable instructions that AI coding agents can discover and invoke automatically.
Each skill is defined in a `SKILL.md` file — a Markdown document fronted by YAML metadata
(name, description, compatibility, allowed tools) that tells the agent *what* the skill does
and *when* to invoke it. The metadata is indexed at session start for fast discovery; the full
Markdown body is loaded on demand, following a
[progressive-disclosure](https://code.claude.com/docs/en/skills)
pattern that keeps the context window lean. 

The `aigent` tool validates, formats, and assembles these
skill files so you can focus on writing the instructions rather than fighting the specification.

Beyond individual skills, `aigent` assembles and validates entire
[Claude Code plugin](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code/extensions)
directories — building plugins from skills with `aigent build`, and checking
the `plugin.json` manifest, `hooks.json` configuration, agent and command files,
skill subdirectories, and cross-component consistency with `aigent validate-plugin`.

<p align="center">
  <img src="https://github.com/wkusnierczyk/aigent/raw/main/graphics/hello.gif" alt="aigent demo" width="800" />
</p>

## Table of contents

- [Installation](#installation)
- [Quick start](#quick-start)
- [Library usage](#library-usage)
- [`SKILL.md` format](#skillmd-format)
- [Builder modes](#builder-modes)
- [Compliance](#compliance)
- [CLI reference](#cli-reference)
- [API reference](#api-reference)
- [Claude Code plugin](#claude-code-plugin)
- [Development](#development)
- [See also](#see-also)
- [References](#references)
- [About and licence](#about-and-licence)

## Installation

Pre-built binaries are available for all major platforms — no Rust toolchain required.
If you prefer to build from source, see [From crates.io](#from-cratesio) or
[From source](#from-source) below.

### Homebrew (macOS and Linux)

```bash
brew install wkusnierczyk/aigent/aigent
```

### Pre-built binaries

Download a pre-built binary from the
[latest release](https://github.com/wkusnierczyk/aigent/releases/latest)
(Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64).

### Install script (Linux and macOS)

The script detects the OS and architecture, downloads the latest release archive from GitHub,
verifies its SHA-256 checksum, and extracts the binary to `~/.local/bin`.

```bash
curl -fsSL https://raw.githubusercontent.com/wkusnierczyk/aigent/main/install.sh | bash
```

Or download and review the script before running:

```bash
curl -fsSL https://raw.githubusercontent.com/wkusnierczyk/aigent/main/install.sh -o install.sh
less install.sh        # review the script
bash install.sh
```

### From crates.io

Requires [Rust](https://www.rust-lang.org/tools/install) (stable toolchain).

```bash
cargo install aigent
```

### From source

```bash
git clone https://github.com/wkusnierczyk/aigent.git
cd aigent
cargo install --path .
```

## Quick start

> **Note**  
> The examples below use the `aigent` CLI.  
> * For library usage, see [Library Usage](#library-usage).  
> * For the Claude Code plugin, see [Claude Code Plugin](#claude-code-plugin).

```bash
# Initialize a new skill
aigent init my-skill/

# Create a skill from a description
aigent new "Process PDF files and extract text" --no-llm

# Validate (from inside a skill directory — path defaults to .)
cd my-skill/
aigent validate --structure

# Or specify a path explicitly
aigent validate my-skill/ --structure

# Run validate + semantic quality checks
aigent check my-skill/

# Score a skill against best practices (0–100)
aigent score my-skill/

# Format a SKILL.md (canonical key order, clean whitespace)
aigent format my-skill/

# Probe skill activation against a query
aigent probe my-skill/ --query "process PDF files"

# Run fixture-based test suite
aigent test my-skill/

# Check for upgrade opportunities
aigent upgrade my-skill/

# Assemble skills into a Claude Code plugin
aigent build my-skill/ other-skill/ --output ./dist

# Generate a skill catalog
aigent doc skills/ --recursive

# Read skill properties as JSON
aigent properties my-skill/

# Generate XML prompt for LLM injection
aigent prompt my-skill/ other-skill/

# Validate a full Claude Code plugin directory
aigent validate-plugin my-plugin/
```

To enable LLM-enhanced generation, set an API key for any
[supported provider](#builder-modes) (Anthropic, OpenAI, Google, or Ollama).
Without an API key, the builder uses deterministic mode, which requires
no configuration.

## Library usage

See [docs/library.md](docs/library.md).

## `SKILL.md` format

The format follows the [Agent Skills open standard](https://agentskills.io),
originally defined by [Anthropic](https://code.claude.com/docs/en/skills).
Skills are defined in `SKILL.md` files with YAML frontmatter and a Markdown body.

> **Note**  
> `skill.md` is also recognized, but `SKILL.md` is preferred.

For example:

```markdown
---
name: extract-csv-data
description: >-
  Extract and transform data from CSV files. Use when the user needs
  to parse, filter, or aggregate CSV data.
license: MIT
compatibility: claude
allowed-tools: Read, Write, Bash
---

# Extract CSV Data

Parse and transform CSV files into structured data.

## When to use

Use this skill when:
- The user asks to extract data from CSV files
- The task involves filtering or aggregating tabular data

## Instructions

1. Read the CSV file using the Read tool
2. Parse the header row to identify columns
3. Apply any requested filters or transformations
```

### Frontmatter fields

| Field | Required | Description |
|-------|:--------:|-------------|
| `name` | yes | Kebab-case identifier (e.g., `extract-csv-data`) |
| `description` | yes | Free-text description of what the skill does and when to use it |
| `license` | no | Free-text licence string (e.g., `MIT`) |
| `compatibility` | no | Free-text string indicating compatible agent platforms (e.g., `claude-code`) |
| `allowed-tools` | no | Comma-separated list of tools the skill may use (e.g., `Bash, Read, Write`) |

### Validation rules

| Field | Rule |
|-------|------|
| `name`, `description` | Required and non-empty |
| `name` | Lowercase letters, digits, and hyphens only; maximum 64 characters |
| `name` | Must not contain reserved words (`anthropic`, `claude`) |
| `name` | No XML tags; must match directory name |
| `name` | Unicode [NFKC](https://unicode.org/reports/tr15/) normalization applied before validation (e.g., `ﬁ` → `fi`) |
| `description` | Maximum 1024 characters; no XML/HTML tags |
| `compatibility` | Maximum 500 characters (if present) |
| Body | Warning if longer than 500 lines |

## Builder modes

The skill builder operates in two modes:

**Deterministic** — Always available, zero configuration. Uses heuristic
rules to derive skill names (gerund form, kebab-case), descriptions, and
Markdown bodies. Output is formulaic but valid.

**LLM-enhanced** — Auto-detected via environment variables. Produces
richer, more natural output. Each generation step (name, description, body)
independently falls back to deterministic on LLM failure.

### Provider detection order

| Priority | Environment Variable | Provider |
|:--------:|---------------------|----------|
| 1 | `ANTHROPIC_API_KEY` | Anthropic Claude |
| 2 | `OPENAI_API_KEY` | OpenAI |
| 3 | `GOOGLE_API_KEY` | Google Gemini |
| 4 | `OLLAMA_HOST` | Ollama (local) |

### Available models

Default models as of 2026-02-20:

| Provider | Environment Variable | Default Model | Override Variable |
|----------|---------------------|---------------|-------------------|
| Anthropic | `ANTHROPIC_API_KEY` | `claude-sonnet-4-20250514` | `ANTHROPIC_MODEL` |
| OpenAI | `OPENAI_API_KEY` | `gpt-4o` | `OPENAI_MODEL` |
| Google | `GOOGLE_API_KEY` | `gemini-2.0-flash` | `GOOGLE_MODEL` |
| Ollama | `OLLAMA_HOST` | `llama3.2` | `OLLAMA_MODEL` |

OpenAI-compatible endpoints (vLLM, LM Studio, etc.) are supported via
`OPENAI_API_BASE` or `OPENAI_BASE_URL`.

Use `--no-llm` to force deterministic mode regardless of available providers.

## Compliance

`aigent` is built to be fully compliant with the
[Agent Skills open standard](https://agentskills.io) and the
[Python reference implementation](https://github.com/agentskills/agentskills).

### Specification coverage

Three-way comparison of the
[Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices),
`aigent`, and the
[Python reference implementation](https://github.com/agentskills/agentskills).

> The following table shows key validation rules from the Anthropic specification. Additional
> checks (frontmatter structure, metadata keys, YAML syntax) are implemented
> but not listed as they are standard parser behaviour.

| Rule | `aigent` | Specification | Python Reference |
|------|:------:|:-------------:|:----------------:|
| Name ≤ 64 characters | ✅ | ✅ | ✅ |
| Name: lowercase + hyphens | ✅ | ✅ | ✅ |
| Name: no XML tags | ✅ | ✅ | ❌ |
| Name: no reserved words | ✅ | ✅ | ❌ |
| Name: Unicode NFKC | ✅ | — | ❌ |
| Description: non-empty | ✅ | ✅ | ✅ |
| Description ≤ 1024 characters | ✅ | ✅ | ✅ |
| Description: no XML tags | ✅ | ✅ | ❌ |
| Frontmatter `---` delimiters | ✅ | ✅ | ✅ |
| Compatibility ≤ 500 characters | ✅ | ✅ | ❌ |
| Body ≤ 500 lines warning | ✅ | ✅ | ❌ |
| Prompt XML format | ✅ | ✅ | ✅ |
| Path canonicalization | ✅ | — | ✅ |
| Post-build validation | ✅ | — | ❌ |

`aigent` implements **all** rules from the specification, plus additional checks
(Unicode NFKC normalization, path canonicalization, post-build validation) that
go beyond both the specification and the reference implementation.

`aigent` is integration-tested against the
[Anthropic skill collection](https://github.com/anthropics/skills)
(12 Apache 2.0-licensed skills).

### `aigent` vs. `plugin-dev`

Anthropic's **`plugin-dev`** plugin (bundled with Claude Code) and **`aigent`**
are complementary tools for plugin development.

| | **`aigent`** | **`plugin-dev`** |
|---|---|---|
| **What** | Rust CLI + library | Claude Code plugin (LLM-guided) |
| **Scope** | Deep: skills + plugin ecosystem validation | Broad: entire plugin ecosystem guidance |
| **Validation** | Deterministic — typed diagnostics, error codes, JSON output | Heuristic — agent-based review |
| **Plugin validation** | `aigent validate-plugin` — manifest, hooks, agents, commands, skills, cross-component | `plugin-validator` agent — LLM-driven review |
| **Scoring** | Weighted 0–100 with CI gating | Not available |
| **Formatting** | `aigent format` — idempotent, `--check` for CI | Not available |
| **Testing** | Fixture-based (`tests.yml`) + single-query probe | General guidance only |
| **Assembly** | `aigent build` — reproducible, scriptable | `/create-plugin` — guided, interactive |

Overall:
* `aigent` provides **deterministic enforcement** — skill quality (validation,
scoring, formatting, testing, assembly) and plugin-level validation (manifest,
hooks, agents, commands, cross-component consistency).
* `plugin-dev` provides **LLM-guided breadth** across the Claude Code plugin
ecosystem (7 component types, ~21,000 words of guidance).

Use `plugin-dev` to learn patterns; use `aigent` to enforce them.

For a complete comparison, see [docs/plugin-dev.md](docs/plugin-dev.md).

### Extras

Features in `aigent` that go beyond the specification and reference implementation.

| Feature | Description |
|---------|-------------|
| Semantic linting | Quality checks: third-person descriptions, trigger phrases, gerund names, generic names |
| Quality scoring | Weighted 0–100 score with distinct pass/fail labels per check |
| Auto-fix | Automatic correction of fixable issues (e.g., name casing) |
| Skill builder | Generate skills from natural language (deterministic + multi-provider LLM) |
| Interactive build | Step-by-step confirmation mode for skill generation |
| Skill tester (probe) | Simulate skill activation with weighted scoring formula (0.5×description + 0.3×trigger + 0.2×name) |
| Fixture-based testing | Run test suites from `tests.yml` with expected match/no-match and minimum score thresholds |
| `SKILL.md` formatter | Canonical YAML key ordering, consistent whitespace, idempotent formatting |
| Skill-to-plugin assembly | Package skill directories into a Claude Code plugin with `plugin.json` manifest |
| Skill upgrade | Detect and apply best-practice upgrades with `--full` mode (validate + lint + fix + upgrade) |
| Unified check command | `check` = validate + semantic lint; `--no-validate` for lint-only |
| Directory structure validation | Check for missing references, script permissions, nesting depth |
| Cross-skill conflict detection | Name collisions, description similarity, token budget analysis |
| Documentation generation | Markdown skill catalog with diff-aware output |
| Watch mode | Continuous validation on filesystem changes (optional `notify` feature) |
| Multi-format prompt output | XML, JSON, YAML, Markdown prompt generation |
| Multi-format validation output | Text and JSON diagnostic output |
| Token budget estimation | Per-skill and total token usage reporting |
| Plugin ecosystem validation | Validate full plugin directories: manifest, hooks, agents, commands, skills, cross-component |
| Claude Code plugin | Hybrid skills that work with or without the CLI installed |

## CLI reference

See [docs/cli.md](docs/cli.md).

## API reference

See [docs/api.md](docs/api.md). Full Rust API documentation is at [docs.rs/aigent](https://docs.rs/aigent).

## Claude Code plugin

See [docs/plugin.md](docs/plugin.md).

## Development

See [docs/development.md](docs/development.md).

## See also

| Tool | Focus |
|------|-------|
| [agent-skills](https://crates.io/crates/agent-skills) | Parse and validate agent skills (Rust library) |
| [skills](https://github.com/cortesi/skills) | Sync skills across Claude Code and Codex (Rust CLI) |
| [oh-my-agent-skills](https://crates.io/crates/oh-my-agent-skills) | Terminal-based skill manager (TUI) |

## References

| Reference | Description |
|-----------|-------------|
| [Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices) | Official specification for `SKILL.md` format and validation rules |
| [Agent Skills organisation](https://github.com/agentskills) | Umbrella for agent skills tooling |
| [agentskills/agentskills](https://github.com/agentskills/agentskills) | Python reference implementation |
| [anthropics/skills](https://github.com/anthropics/skills) | Anthropic's skills repository |
| [docs.rs/aigent](https://docs.rs/aigent) | Rust API documentation |
| [crates.io/crates/aigent](https://crates.io/crates/aigent) | Package registry |

## About and licence

```
aigent: Rust AI Agent Skills Tool
├─ version:    0.7.0
├─ author:     Wacław Kuśnierczyk
├─ developer:  mailto:waclaw.kusnierczyk@gmail.com
├─ source:     https://github.com/wkusnierczyk/aigent
└─ licence:    Apache-2.0 https://www.apache.org/licenses/LICENSE-2.0
```

[Apache 2.0](LICENSE) — see [apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0).
