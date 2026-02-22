<table>
  <tr>
    <td>
      <img src="https://github.com/wkusnierczyk/aigent/raw/main/graphics/aigent.png" alt="logo" width="300" />
    </td>
    <td>
      <p><strong><code>aigent</code></strong>: AI agent Swiss Army knife.</p>
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

## Table of contents

- [Installation](#installation)
  - [Pre-built binaries](#pre-built-binaries)
  - [Install script (Linux and macOS)](#install-script-linux-and-macos)
  - [From crates.io](#from-cratesio)
  - [From source](#from-source)
- [Quick start](#quick-start)
- [Library usage](#library-usage)
- [`SKILL.md` format](#skillmd-format)
  - [Frontmatter fields](#frontmatter-fields)
  - [Validation rules](#validation-rules)
- [Builder modes](#builder-modes)
  - [Provider detection order](#provider-detection-order)
  - [Available models](#available-models)
- [Compliance](#compliance)
  - [Specification coverage](#specification-coverage)
  - [`aigent` vs. `plugin-dev`](#aigent-vs-plugin-dev)
  - [Extras](#extras)
- [CLI reference](#cli-reference)
  - [Commands](#commands)
  - [Exit codes](#exit-codes)
  - [Command flags](#command-flags)
    - [`build` (assembly) flags](#build-assembly-flags)
    - [`check` flags](#check-flags)
    - [`format` flags](#format-flags)
    - [`new` flags](#new-flags)
    - [`probe` flags](#probe-flags)
    - [`test` flags](#test-flags)
    - [`upgrade` flags](#upgrade-flags)
    - [`validate` flags](#validate-flags)
    - [`validate-plugin` flags](#validate-plugin-flags)
  - [Command examples](#command-examples)
    - [`build` — Assemble skills into a plugin](#build--assemble-skills-into-a-plugin)
    - [`check` — Validate + semantic quality checks](#check--validate--semantic-quality-checks)
    - [`doc` — Generate a skill catalog](#doc--generate-a-skill-catalog)
    - [`format` — Format `SKILL.md` files](#format--format-skillmd-files)
    - [`init` — Create a template `SKILL.md`](#init--create-a-template-skillmd)
    - [`new` — Create a skill from natural language](#new--create-a-skill-from-natural-language)
    - [`probe` — Simulate skill activation](#probe--simulate-skill-activation)
    - [`prompt` — Generate XML prompt block](#prompt--generate-xml-prompt-block)
    - [`properties` — Output skill metadata as JSON](#properties--output-skill-metadata-as-json)
    - [`score` — Rate a skill 0–100](#score--rate-a-skill-0100)
    - [`test` — Run fixture-based test suites](#test--run-fixture-based-test-suites)
    - [`upgrade` — Detect and apply best-practice improvements](#upgrade--detect-and-apply-best-practice-improvements)
    - [`validate` — Check skill directories for specification conformance](#validate--check-skill-directories-for-specification-conformance)
    - [`validate-plugin` — Validate a Claude Code plugin directory](#validate-plugin--validate-a-claude-code-plugin-directory)
  - [Watch mode](#watch-mode)
  - [Global flags](#global-flags)
- [API reference](#api-reference)
  - [Types](#types)
  - [Functions](#functions)
  - [Traits](#traits)
- [Claude Code plugin](#claude-code-plugin)
  - [Skills](#skills)
  - [Plugin installation](#plugin-installation)
- [Development](#development)
  - [Prerequisites](#prerequisites)
  - [Setup](#setup)
  - [Optional tooling](#optional-tooling)
  - [Common tasks](#common-tasks)
  - [Project structure](#project-structure)
  - [Versioning](#versioning)
  - [Milestones](#milestones)
  - [Roadmap](#roadmap)
- [CI/CD and release workflows](#cicd-and-release-workflows)
  - [Continuous integration](#continuous-integration)
  - [Release workflow](#release-workflow)
- [References](#references)
- [About and licence](#about-and-licence)

## Installation

Pre-built binaries are available for all major platforms — no Rust toolchain required.
If you prefer to build from source, see [From crates.io](#from-cratesio) or
[From source](#from-source) below.

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

```rust
use std::path::Path;

// Validate a skill directory
let errors = aigent::validate(Path::new("my-skill"));

// Read skill properties
let props = aigent::read_properties(Path::new("my-skill")).unwrap();

// Generate prompt XML
let xml = aigent::to_prompt(&[Path::new("skill-a"), Path::new("skill-b")]);

// Format a SKILL.md
let result = aigent::format_skill(Path::new("my-skill")).unwrap();

// Assemble skills into a plugin
let opts = aigent::AssembleOptions {
    output_dir: std::path::PathBuf::from("./dist"),
    name: None,
    validate: true,
};
let result = aigent::assemble_plugin(
    &[Path::new("skill-a"), Path::new("skill-b")], &opts,
).unwrap();

// Build a skill
let spec = aigent::SkillSpec {
    purpose: "Process PDF files".to_string(),
    no_llm: true,
    ..Default::default()
};
let result = aigent::build_skill(&spec).unwrap();

// Validate a plugin directory (manifest, hooks, agents, commands, skills)
let manifest_diags = aigent::validate_manifest(Path::new("my-plugin/plugin.json"));
let hooks_diags = aigent::validate_hooks(Path::new("my-plugin/hooks.json"));
let cross_diags = aigent::validate_cross_component(Path::new("my-plugin"));
```

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

For a complete comparison, see [dev/plugin-dev.md](dev/plugin-dev.md).

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

Run `aigent --help` for a list of commands.  
Run `aigent <command> --help` for details on a specific command (flags, arguments, examples).

Full API documentation is available at [docs.rs/aigent](https://docs.rs/aigent).

### Commands

**Quality spectrum:** `validate` (specification conformance) → `check` (+ semantic quality) → `score` (quantitative 0–100).

<table>
<tr><th width="280">Command</th><th>Description</th></tr>
<tr><td><code>build [dirs...]</code></td><td>Assemble skills into a Claude Code plugin</td></tr>
<tr><td><code>check [dirs...]</code></td><td>Run validate + semantic lint checks (superset of <code>validate</code>)</td></tr>
<tr><td><code>doc [dirs...]</code></td><td>Generate a markdown skill catalog</td></tr>
<tr><td><code>format [dirs...]</code></td><td>Format <code>SKILL.md</code> files (canonical key order, clean whitespace)</td></tr>
<tr><td><code>init [directory]</code></td><td>Create a template <code>SKILL.md</code></td></tr>
<tr><td><code>new &lt;purpose&gt;</code></td><td>Create a skill from natural language</td></tr>
<tr><td><code>probe [dirs...] --query &lt;query&gt;</code></td><td>Probe skill activation against a sample user query</td></tr>
<tr><td><code>prompt [dirs...]</code></td><td>Generate <code>&lt;available_skills&gt;</code> XML block</td></tr>
<tr><td><code>properties [directory]</code></td><td>Output skill properties as JSON</td></tr>
<tr><td><code>score [directory]</code></td><td>Score a skill against best-practices checklist (0–100)</td></tr>
<tr><td><code>test [dirs...]</code></td><td>Run fixture-based test suites from <code>tests.yml</code></td></tr>
<tr><td><code>upgrade [directory]</code></td><td>Check a skill for upgrade opportunities</td></tr>
<tr><td><code>validate [dirs...]</code></td><td>Validate skill directories against the specification</td></tr>
<tr><td><code>validate-plugin [plugin-dir]</code></td><td>Validate a Claude Code plugin directory (manifest, hooks, agents, commands, skills, cross-component)</td></tr>
</table>

> **Note**
> When no path is given, the current directory is used. This lets you run
> `aigent validate`, `aigent format --check`, etc. without specifying a path
> when the current directory contains a `SKILL.md` file. The tool does not
> search parent directories.

> **Note**
> Backward compatibility: The following old command names are available as hidden
> aliases and continue to work.
>
> | Old name | Current name |
> |----------|--------------|
> | `create` | `new` |
> | `fmt` | `format` |
> | `lint` | `check` |
> | `read-properties` | `properties` |
> | `to-prompt` | `prompt` |

### Exit codes

All commands exit 0 on success and 1 on failure. The table below clarifies
what "success" means for each command.

| Command | Exit 0 | Exit 1 |
|---------|--------|--------|
| `build` | Plugin assembled successfully | Assembly error |
| `check` | No errors | Errors found (warnings do not affect exit code) |
| `doc` | Catalog generated | I/O error |
| `format` | All files already formatted | Files were reformatted (with `--check`) or error |
| `init` | Template created | Directory already exists or I/O error |
| `new` | Skill created | Build error |
| `probe` | At least one result printed | All directories failed to parse |
| `prompt` | Prompt generated | No valid skills found |
| `properties` | Properties printed | Parse error |
| `score` | Perfect score (100/100) | Score below 100 |
| `test` | All test cases pass | Any test case fails |
| `upgrade` | No suggestions, or applied successfully | Unapplied suggestions remain, or error |
| `validate` | No errors | Errors found (warnings do not affect exit code) |
| `validate-plugin` | No errors | Errors found in manifest, hooks, agents, commands, skills, or cross-component checks |

### Command flags

#### `build` (assembly) flags

Assemble skills into a Claude Code plugin.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--name &lt;name&gt;</code></td><td>Override the plugin name (default: first skill name)</td></tr>
<tr><td><code>--output &lt;dir&gt;</code></td><td>Output directory for the assembled plugin (default: <code>./dist</code>)</td></tr>
<tr><td><code>--validate</code></td><td>Run validation on assembled skills</td></tr>
</table>

#### `check` flags

Run validate + semantic lint checks (superset of `validate`).

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--apply-fixes</code></td><td>Apply automatic fixes for fixable issues</td></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
<tr><td><code>--no-validate</code></td><td>Skip specification conformance checks (semantic quality only)</td></tr>
<tr><td><code>--recursive</code></td><td>Discover skills recursively</td></tr>
<tr><td><code>--structure</code></td><td>Run directory structure checks</td></tr>
<tr><td><code>--target &lt;target&gt;</code></td><td>Validation target profile (see <a href="#validate-flags"><code>validate</code> flags</a>)</td></tr>
</table>

#### `format` flags

Format `SKILL.md` files (canonical key order, clean whitespace).

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--check</code></td><td>Check formatting without modifying files (exit 1 if unformatted)</td></tr>
<tr><td><code>--recursive</code></td><td>Discover skills recursively</td></tr>
</table>

#### `new` flags

Create a skill from natural language.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--dir &lt;dir&gt;</code></td><td>Output directory</td></tr>
<tr><td><code>--interactive, -i</code></td><td>Step-by-step confirmation mode</td></tr>
<tr><td><code>--name &lt;name&gt;</code></td><td>Override the derived skill name</td></tr>
<tr><td><code>--no-llm</code></td><td>Force deterministic mode (no LLM)</td></tr>
</table>

#### `probe` flags

Probe skill activation against a sample user query.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--query, -q &lt;query&gt;</code></td><td>Sample user query to test activation against (required)</td></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
</table>

#### `test` flags

Run fixture-based test suites from `tests.yml`.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
<tr><td><code>--generate</code></td><td>Generate a template <code>tests.yml</code> for skills that lack one</td></tr>
<tr><td><code>--recursive</code></td><td>Discover skills recursively</td></tr>
</table>

#### `upgrade` flags

Check a skill for upgrade opportunities.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--apply</code></td><td>Apply automatic upgrades</td></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
<tr><td><code>--full</code></td><td>Run validate + lint before upgrade (with <code>--apply</code>, also fix errors first)</td></tr>
</table>

#### `validate` flags

Validate skill directories against the specification.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--apply-fixes</code></td><td>Apply automatic fixes for fixable issues</td></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
<tr><td><code>--recursive</code></td><td>Discover skills recursively</td></tr>
<tr><td><code>--structure</code></td><td>Run directory structure checks</td></tr>
<tr><td><code>--target &lt;target&gt;</code></td><td>Validation target profile (see below)</td></tr>
<tr><td><code>--watch</code></td><td>Watch for changes and re-validate (see <a href="#watch-mode">Watch mode</a>)</td></tr>
</table>

**Validation targets** control which frontmatter fields are considered known:

| Target | Description |
|--------|-------------|
| `standard` (default) | Only Anthropic specification fields; warns on extras like `argument-hint` |
| `claude-code` | Standard fields plus Claude Code extension fields (e.g., `argument-hint`, `context`) |
| `permissive` | No unknown-field warnings; all fields accepted |

#### `validate-plugin` flags

Validate a Claude Code plugin directory.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
</table>

> **Note**
> Semantic lint checks are available with `check`.  
> Use `aigent check` for combined validation + linting, or `aigent check --no-validate` for lint-only.

### Command examples

#### `build` — Assemble skills into a plugin

Packages one or more skill directories into a Claude Code plugin directory
with a `plugin.json` manifest, `skills/` subdirectory, and scaffolded
`agents/` and `hooks/` directories.

```
$ aigent build skills/aigent-validator skills/aigent-scorer --output ./dist
Assembled 2 skill(s) into ./dist
```

The output structure:

```
dist/
├── plugin.json
├── skills/
│   ├── aigent-validator/
│   │   └── SKILL.md
│   └── aigent-scorer/
│       └── SKILL.md
├── agents/
└── hooks/
```

#### `check` — Validate + semantic quality checks

Runs specification conformance (like `validate`) plus semantic quality checks:
third-person descriptions, trigger phrases, gerund name forms, generic
names, and description detail. Use `--no-validate` to skip specification checks
and run semantic lint only.

Diagnostics use three severity levels:
- **error** — specification violation (causes exit 1)
- **warning** — specification conformance issue (does not affect exit code)
- **info** — quality suggestion from semantic lint (does not affect exit code)

```
$ aigent check skills/aigent-validator
warning: unexpected metadata field: 'argument-hint'
info: name does not use gerund form
```

Semantic lint only:

```
$ aigent check --no-validate skills/aigent-validator
info: name does not use gerund form
```

#### `doc` — Generate a skill catalog

Produces a markdown catalog of skills. Use `--recursive` to discover skills
in subdirectories, and `--output` to write to a file (diff-aware — only
writes if content changed).

```
$ aigent doc skills --recursive
# Skill Catalog

## aigent-builder
> Generates AI agent skill definitions (SKILL.md files) from natural
> language descriptions. ...
**Location**: `skills/aigent-builder/SKILL.md`

---

## aigent-scorer
> Scores AI agent skill definitions (SKILL.md files) against the Anthropic
> best-practices checklist. ...
**Location**: `skills/aigent-scorer/SKILL.md`

---

## aigent-validator
> Validates AI agent skill definitions (SKILL.md files) against the
> Anthropic agent skill specification. ...
**Location**: `skills/aigent-validator/SKILL.md`

---
```

```
$ aigent doc skills --recursive --output catalog.md
(writes catalog.md; re-running skips write if content unchanged)
```

#### `format` — Format `SKILL.md` files

Normalizes `SKILL.md` files with canonical YAML key ordering, consistent
whitespace, and clean formatting. The operation is idempotent — running
it twice produces no further changes.

```
$ aigent format my-skill/
Formatted my-skill/
```

Check mode reports which files would change without modifying them,
and shows a unified diff of the changes:

```
$ aigent format --check my-skill/
Would reformat: my-skill/
--- my-skill/
+++ my-skill/ (formatted)
@@ -1,6 +1,6 @@
 ---
-allowed-tools: Bash, Read, Write
 name: my-skill
 description: ...
+allowed-tools: Bash, Read, Write
 ---
```

#### `init` — Create a template `SKILL.md`

Scaffolds a skill directory with a template `SKILL.md` ready for editing.

```
$ aigent init my-skill
Created my-skill/SKILL.md
```

```
$ cat my-skill/SKILL.md
---
name: my-skill
description: Describe what this skill does and when to use it
---

# My Skill

## Quick start
[Add quick start instructions here]

## Usage
[Add detailed usage instructions here]
```

#### `new` — Create a skill from natural language

Creates a complete skill directory with `SKILL.md` from a purpose description.
Uses LLM when an API key is available, or `--no-llm` for deterministic mode.

```
$ aigent new "Extract text from PDF files" --no-llm
Created skill 'extracting-text-pdf-files' at extracting-text-pdf-files
```

The generated `SKILL.md` includes derived name, description, and a template body:

```markdown
---
name: extracting-text-pdf-files
description: Extract text from PDF files. Use when working with files.
---
# Extracting Text Pdf Files

## Quick start
Extract text from PDF files

## Usage
Use this skill to Extract text from PDF files.
```

#### `probe` — Simulate skill activation

Probes whether a skill's description would activate for a given user query.
This is a dry-run of skill discovery — "if a user said *this*, would Claude
pick up *that* skill?" Accepts multiple directories — results are ranked by
match score (best first).

Uses a **weighted formula** to compute a match score (0.0–1.0):
- **0.5 × description overlap** — fraction of query tokens in description
- **0.3 × trigger score** — match against trigger phrases ("Use when...")
- **0.2 × name score** — query-to-name token overlap

Categories based on weighted score:
- **Strong** (≥ 0.4) — skill would reliably activate
- **Weak** (≥ 0.15) — might activate, but description could be improved
- **None** (< 0.15) — skill would not activate for this query

Also reports estimated token cost and any validation issues.

Single directory:

```
$ aigent probe skills/aigent-validator --query "validate a skill"
Skill: aigent-validator
Query: "validate a skill"
Description: Validates AI agent skill definitions (SKILL.md files) against
the Anthropic agent skill specification. ...

Activation: STRONG ✓ — description aligns well with query (score: 0.65)
Token footprint: ~76 tokens

Validation warnings (1):
  warning: unexpected metadata field: 'argument-hint'
```

Multiple directories (results ranked by score, best first):

```
$ aigent probe skills/* --query "validate a skill"
Skill: aigent-validator
...
Activation: STRONG ✓ — description aligns well with query (score: 0.65)

Skill: aigent-scorer
...
Activation: WEAK ⚠ — some overlap, but description may not trigger reliably (score: 0.25)

Skill: aigent-builder
...
Activation: NONE ✗ — description does not match the test query (score: 0.00)
```

Default directory (from inside a skill directory):

```
$ cd skills/aigent-validator
$ aigent probe --query "validate a skill"
Skill: aigent-validator
...
Activation: STRONG ✓ — description aligns well with query (score: 0.65)
```

#### `prompt` — Generate XML prompt block

Generates the `<available_skills>` XML block that gets injected into Claude's
system prompt. Accepts multiple skill directories.

```
$ aigent prompt skills/aigent-validator
<available_skills>
  <skill>
    <name>aigent-validator</name>
    <description>Validates AI agent skill definitions ...</description>
    <location>skills/aigent-validator/SKILL.md</location>
  </skill>
</available_skills>
```

#### `properties` — Output skill metadata as JSON

Parses the `SKILL.md` frontmatter and outputs structured JSON. Useful for
scripting and integration with other tools.

```
$ aigent properties skills/aigent-validator
{
  "name": "aigent-validator",
  "description": "Validates AI agent skill definitions ...",
  "allowed-tools": "Bash(aigent validate *), Bash(command -v *), Read, Glob",
  "metadata": {
    "argument-hint": "[skill-directory-or-file]"
  }
}
```

#### `score` — Rate a skill 0–100

Rates a skill from 0 to 100 against the Anthropic best-practices checklist.
The score has two weighted categories:

- **Structural (60 points)** — Checks that the `SKILL.md` parses correctly, the
  name matches the directory, required fields are present, no unknown fields
  exist, and the body is within size limits. All six checks must pass to earn
  the 60 points; any failure zeros the structural score.

- **Quality (40 points)** — Five semantic lint checks worth 8 points each:
  third-person description, trigger phrase (`"Use when..."`), gerund name form
  (`converting-pdfs` not `pdf-converter`), specific (non-generic) name, and
  description length (≥ 20 words).

The exit code is 0 for a perfect score and 1 otherwise, making it suitable for
CI gating.

**Example** — a skill that passes all checks:

```
$ aigent score converting-pdfs/
Score: 100/100

Structural (60/60):
  [PASS] SKILL.md exists and is parseable
  [PASS] Name format valid
  [PASS] Description valid
  [PASS] Required fields present
  [PASS] No unknown fields
  [PASS] Body within size limits

Quality (40/40):
  [PASS] Third-person description
  [PASS] Trigger phrase present
  [PASS] Gerund name form
  [PASS] Specific name
  [PASS] Detailed description
```

**Example** — a skill with issues. Each check shows a distinct label for its
pass/fail state (e.g., "No unknown fields" when passing, "Unknown fields
found" when failing):

```
$ aigent score aigent-validator/
Score: 32/100

Structural (0/60):
  [PASS] SKILL.md exists and is parseable
  [PASS] Name format valid
  [PASS] Description valid
  [PASS] Required fields present
  [FAIL] Unknown fields found
         unexpected metadata field: 'argument-hint'
  [PASS] Body within size limits

Quality (32/40):
  [PASS] Third-person description
  [PASS] Trigger phrase present
  [FAIL] Non-gerund name form
         name does not use gerund form
  [PASS] Specific name
  [PASS] Detailed description
```

#### `test` — Run fixture-based test suites

Runs test suites defined in `tests.yml` files alongside skills. Each test
case specifies an input query, whether it should match, and an optional
minimum score threshold.

Generate a template `tests.yml`:

```
$ aigent test --generate my-skill/
Generated my-skill/tests.yml
```

```yaml
# Test fixture for my-skill
# Run with: aigent test my-skill/
queries:
- input: process pdf files and extract text
  should_match: true
  min_score: 0.3
- input: something completely unrelated to this skill
  should_match: false
```

Run the test suite:

```
$ aigent test my-skill/
[PASS] "process pdf files" (score: 0.65)
[PASS] "something completely unrelated to this skill" (score: 0.00)

2 passed, 0 failed, 2 total
```

#### `upgrade` — Detect and apply best-practice improvements

Checks for recommended-but-optional fields and patterns. Use `--apply` to
write missing fields into the `SKILL.md`. The `--full` flag first runs
validate + lint and optionally fixes errors before analysing upgrades.

```
$ aigent upgrade skills/aigent-validator
Missing 'compatibility' field — recommended for multi-platform skills.
Missing 'metadata.version' — recommended for tracking skill versions.
Missing 'metadata.author' — recommended for attribution.

Run with --apply to apply 3 suggestion(s).
```

```
$ aigent upgrade --apply skills/aigent-validator
(applies missing fields in-place, prints confirmation to stderr)
```

Full mode runs validate + lint first, and with `--apply` fixes errors
before performing upgrades:

```
$ aigent upgrade --full --apply skills/aigent-validator
[full] Applied 1 validation/lint fix(es)
Missing 'compatibility' field — recommended for multi-platform skills.
```

#### `validate` — Check skill directories for specification conformance

Validates one or more skill directories against the Anthropic specification.
Exit code 0 means valid; non-zero means errors were found (warnings do not affect exit code). For
combined validation + semantic quality checks, use `check` instead.

```
$ aigent validate my-skill/
(no output — skill is valid)
```

With `--structure` for directory layout checks:

```
$ aigent validate skills/aigent-validator --structure
warning: unexpected metadata field: 'argument-hint'
```

Multiple directories trigger cross-skill conflict detection automatically:

```
$ aigent validate skills/aigent-validator skills/aigent-builder skills/aigent-scorer
skills/aigent-validator:
  warning: unexpected metadata field: 'argument-hint'
skills/aigent-builder:
  warning: unexpected metadata field: 'argument-hint'
  warning: unexpected metadata field: 'context'
skills/aigent-scorer:
  warning: unexpected metadata field: 'argument-hint'

3 skills: 0 ok, 0 errors, 3 warnings only
```

JSON output for CI integration:

```
$ aigent validate skills/aigent-validator --format json
[
  {
    "diagnostics": [
      {
        "code": "W001",
        "field": "metadata",
        "message": "unexpected metadata field: 'argument-hint'",
        "severity": "warning"
      }
    ],
    "path": "skills/aigent-validator"
  }
]
```

#### `validate-plugin` — Validate a Claude Code plugin directory

Validates the full plugin ecosystem: `plugin.json` manifest, `hooks.json`,
agent files, command files, skill directories, and cross-component consistency
(naming, duplicates, token budget, orphaned files, hook script references).

```
$ aigent validate-plugin my-plugin/
plugin.json: ok
hooks.json: ok
agents/code-reviewer: ok
commands/deploy: ok
skills/pdf-processor: ok
Cross-component: ok
```

With errors:

```
$ aigent validate-plugin my-plugin/
plugin.json:
  error [P003]: `name` is not kebab-case: "My Plugin"
hooks.json:
  error [H003]: unknown event name: "OnSave"
Cross-component:
  warning [X006]: duplicate component name "helper" across agent and command
```

### Watch mode

The `--watch` flag on `validate` monitors skill directories for filesystem
changes and re-validates automatically on each edit — a live feedback loop
while developing skills.

Watch mode is behind a **Cargo feature gate** because it pulls in
platform-specific filesystem notification libraries (`notify`, `fsevent-sys`
on macOS, `inotify` on Linux). Without the feature, the binary is smaller
and has fewer dependencies.

**Building with watch mode:**

```bash
# Build with watch support
cargo build --release --features watch

# Run with watch support (--features must be passed every time)
cargo run --release --features watch -- validate --watch skills/

# Or install with watch support
cargo install aigent --features watch
```

> **Note**  
> Cargo feature flags are per-invocation — they are not remembered
> between builds. You must pass `--features watch` on every `cargo build` or
> `cargo run` invocation. Building debug with `--features watch` does not
> enable it for release builds, and vice versa.

Without the `watch` feature, using `--watch` prints a helpful error:

```
$ aigent validate --watch my-skill/
Watch mode requires the 'watch' feature. Rebuild with: cargo build --features watch
```

### Global flags

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--about</code></td><td>Show project information</td></tr>
<tr><td><code>--version</code></td><td>Print version</td></tr>
<tr><td><code>--help</code></td><td>Print help</td></tr>
</table>

## API reference

Full Rust API documentation with examples is published at
[docs.rs/aigent](https://docs.rs/aigent).

### Types

| Type | Module | Description |
|------|--------|-------------|
| `SkillProperties` | `models` | Parsed skill metadata (name, description, licence, compatibility, allowed-tools) |
| `SkillSpec` | `builder` | Input specification for skill generation (purpose, optional overrides) |
| `BuildResult` | `builder` | Build output (properties, files written, output directory) |
| `ClarityAssessment` | `builder` | Purpose clarity evaluation result (clear flag, follow-up questions) |
| `Diagnostic` | `diagnostics` | Structured diagnostic with severity, code, message, field, suggestion |
| `ScoreResult` | `scorer` | Quality score result with structural and semantic categories |
| `TestResult` | `tester` | Skill activation probe result (query match, score, diagnostics, token cost) |
| `TestSuiteResult` | `test_runner` | Fixture-based test suite result (passed, failed, per-case results) |
| `FormatResult` | `formatter` | `SKILL.md` formatting result (changed flag, formatted content) |
| `AssembleOptions` | `assembler` | Options for skill-to-plugin assembly (output dir, name, validate) |
| `AssembleResult` | `assembler` | Assembly output (plugin directory, skill count) |
| `SkillEntry` | `prompt` | Collected skill entry for prompt generation (name, description, location) |
| `PluginManifest` | `plugin` | Parsed `plugin.json` manifest with path override accessors |
| `AigentError` | `errors` | Error enum: `Parse`, `Validation`, `Build`, `Io`, `Yaml` |
| `Result<T>` | `errors` | Convenience alias for `std::result::Result<T, AigentError>` |

### Functions

| Function | Module | Description |
|----------|--------|-------------|
| `validate(&Path) -> Vec<Diagnostic>` | `validator` | Validate skill directory |
| `validate_with_target(&Path, ValidationTarget)` | `validator` | Validate with target profile |
| `read_properties(&Path) -> Result<SkillProperties>` | `parser` | Parse directory into `SkillProperties` |
| `find_skill_md(&Path) -> Option<PathBuf>` | `parser` | Find `SKILL.md` in directory (prefers uppercase) |
| `parse_frontmatter(&str) -> Result<(HashMap, String)>` | `parser` | Split YAML frontmatter and body |
| `to_prompt(&[&Path]) -> String` | `prompt` | Generate `<available_skills>` XML system prompt |
| `to_prompt_format(&[&Path], PromptFormat) -> String` | `prompt` | Generate prompt in specified format |
| `lint(&SkillProperties, &str) -> Vec<Diagnostic>` | `linter` | Run semantic quality checks |
| `score(&Path) -> ScoreResult` | `scorer` | Score skill against best-practices checklist |
| `test_skill(&Path, &str) -> Result<TestResult>` | `tester` | Probe skill activation against a query |
| `format_skill(&Path) -> Result<FormatResult>` | `formatter` | Format `SKILL.md` with canonical key order |
| `format_content(&str) -> Result<String>` | `formatter` | Format `SKILL.md` content string |
| `assemble_plugin(&[&Path], &AssembleOptions) -> Result<AssembleResult>` | `assembler` | Assemble skills into a plugin |
| `run_test_suite(&Path) -> Result<TestSuiteResult>` | `test_runner` | Run fixture-based test suite |
| `generate_fixture(&Path) -> Result<String>` | `test_runner` | Generate template `tests.yml` from skill metadata |
| `validate_structure(&Path) -> Vec<Diagnostic>` | `structure` | Validate directory structure |
| `detect_conflicts(&[SkillEntry]) -> Vec<Diagnostic>` | `conflict` | Detect cross-skill conflicts |
| `apply_fixes(&Path, &[Diagnostic]) -> Result<usize>` | `fixer` | Apply automatic fixes |
| `build_skill(&SkillSpec) -> Result<BuildResult>` | `builder` | Full build pipeline with post-build validation |
| `derive_name(&str) -> String` | `builder` | Derive kebab-case name from purpose (deterministic) |
| `assess_clarity(&str) -> ClarityAssessment` | `builder` | Evaluate if purpose is clear enough for generation |
| `init_skill(&Path, SkillTemplate) -> Result<PathBuf>` | `builder` | Initialize skill directory with template `SKILL.md` |
| `validate_manifest(&Path) -> Vec<Diagnostic>` | `plugin` | Validate `plugin.json` manifest |
| `validate_hooks(&Path) -> Vec<Diagnostic>` | `plugin` | Validate `hooks.json` configuration |
| `validate_agent(&Path) -> Vec<Diagnostic>` | `plugin` | Validate agent `.md` file |
| `validate_command(&Path) -> Vec<Diagnostic>` | `plugin` | Validate command `.md` file |
| `validate_cross_component(&Path) -> Vec<Diagnostic>` | `plugin` | Run cross-component consistency checks |

### Traits

| Trait | Module | Description |
|-------|--------|-------------|
| `LlmProvider` | `builder::llm` | Text generation provider interface (`generate(system, user) -> Result<String>`) |

## Claude Code plugin

This repository is a
[Claude Code plugin](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code/extensions#custom-slash-commands).
It provides three skills that Claude can use to build, validate, and score
SKILL.md files interactively.

### Skills

| Skill | Description |
|-------|-------------|
| `aigent-builder` | Generates `SKILL.md` definitions from natural language. Triggered by "create a skill", "build a skill", etc. |
| `aigent-validator` | Validates `SKILL.md` files against the Anthropic specification. Triggered by "validate a skill", "check a skill", etc. |
| `aigent-scorer` | Scores `SKILL.md` files against best-practices checklist. Triggered by "score a skill", "rate a skill", etc. |

All skills operate in **hybrid mode**: they use the `aigent` CLI when it is
installed, and fall back to Claude-based generation/validation when it is not.
This means the plugin works out of the box — no installation required — but
produces higher-quality results with `aigent` available.

### Plugin installation

To use the plugin in Claude Code, add it to your project's
`.claude/settings.json`:

```json
{
  "permissions": {
    "allow": []
  },
  "plugins": [
    "wkusnierczyk/aigent"
  ]
}
```

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- `cargo` (included with Rust)

### Setup

```bash
git clone https://github.com/wkusnierczyk/aigent.git
cd aigent
cargo build
```

### Optional tooling

```bash
cargo install cargo-edit            # Adds `cargo set-version` for release versioning
```

### Common tasks

```bash
cargo build                         # Build (debug)
cargo build --release               # Build (release)
cargo test                          # Run all tests
cargo clippy -- -D warnings         # Lint (warnings as errors)
cargo fmt                           # Format code
cargo fmt --check                   # Check formatting
```

### Project structure

```
src/
├── lib.rs                          # Library root — re-exports public API
├── errors.rs                       # Error types (thiserror)
├── models.rs                       # SkillProperties (serde)
├── parser.rs                       # SKILL.md frontmatter parser (serde_yaml_ng)
├── validator.rs                    # Metadata and directory validator
├── linter.rs                       # Semantic lint checks
├── fixer.rs                        # Auto-fix for fixable diagnostics
├── diagnostics.rs                  # Structured diagnostics with error codes
├── prompt.rs                       # Multi-format prompt generation
├── scorer.rs                       # Quality scoring with pass/fail labels (0–100)
├── structure.rs                    # Directory structure validation
├── conflict.rs                     # Cross-skill conflict detection
├── tester.rs                       # Skill activation probe with weighted scoring
├── formatter.rs                    # SKILL.md formatting (canonical key order, whitespace)
├── assembler.rs                    # Skill-to-plugin assembly
├── test_runner.rs                  # Fixture-based testing (tests.yml)
├── fs_util.rs                      # Symlink-safe filesystem helpers
├── main.rs                         # CLI entry point (clap)
├── plugin/
│   ├── mod.rs                      # Plugin module declarations
│   ├── manifest.rs                 # plugin.json manifest validation
│   ├── hooks.rs                    # hooks.json validation
│   ├── agent.rs                    # Agent file (.md) validation
│   ├── command.rs                  # Command file (.md) validation
│   └── cross.rs                    # Cross-component consistency checks
└── builder/
    ├── mod.rs                      # Build pipeline orchestration
    ├── deterministic.rs            # Heuristic name/description/body generation
    ├── llm.rs                      # LLM provider trait and generation functions
    ├── template.rs                 # Template for init command
    ├── util.rs                     # Internal utilities
    └── providers/
        ├── mod.rs                  # Provider module declarations
        ├── anthropic.rs            # Anthropic Claude API
        ├── openai.rs               # OpenAI (and compatible) API
        ├── google.rs               # Google Gemini API
        └── ollama.rs               # Ollama local API
```

### Versioning

Version is stored in `Cargo.toml` (single source of truth) and read at compile
time via `env!("CARGO_PKG_VERSION")`.

### Milestones

**Status:** Implementation complete (M1–M15).

Project tracked at
[github.com/users/wkusnierczyk/projects/39](https://github.com/users/wkusnierczyk/projects/39).

| Milestone | Title | Status |
|:---------:|-------|:------:|
| M1 | Project Scaffolding | ✅ |
| M2 | Errors and Models | ✅ |
| M3 | Parser | ✅ |
| M4 | Validator | ✅ |
| M5 | Prompt | ✅ |
| M6 | CLI | ✅ |
| M7 | Builder | ✅ |
| M8 | Main Module and Documentation | ✅ |
| M9 | Claude Code Plugin | ✅ |
| M10 | Improvements and Extensions | ✅ |
| M11 | Builder and Prompt Enhancements | ✅ |
| M12 | Ecosystem and Workflow | ✅ |
| M13 | Enhancements | ✅ |
| M14 | SRE Review | ✅ |
| M15 | Plugin Ecosystem Validation | ✅ |

### Roadmap

See [open issues](https://github.com/wkusnierczyk/aigent/issues) for planned work.

Notable: [#131](https://github.com/wkusnierczyk/aigent/issues/131) — modular CLI
redesign with subcommand groups (`aigent skill ...`, `aigent plugin ...`) for
when additional AI agent domains are supported.

## CI/CD and release workflows

### Continuous integration

The `main` branch is protected: direct pushes are not allowed. Changes are
merged via squash-merge of pull requests only, requiring green CI/CD and positive reviews.

Every pull request runs the CI pipeline on three OSes
(Linux, macOS, Windows).

| Step | Command |
| --- | --- |
| Formatting | `cargo fmt --check` |
| Linting | `cargo clippy -- -D warnings` |
| Testing | `cargo test` |
| Release build | `cargo build --release` |

### Release workflow

Releases are automated via `scripts/version.sh release`:

```bash
./scripts/version.sh release 0.5.0  # explicit version
./scripts/version.sh release patch  # auto-increment patch
./scripts/version.sh release minor  # auto-increment minor
```

This single command:

1. Checks for a clean working tree and that the version tag doesn't exist
2. Generates a changelog from merged PRs since the previous tag (via `gh`)
3. Writes the changelog to `CHANGES.md`
4. Updates version across all files (`Cargo.toml`, `plugin.json`, `README.md`, `Cargo.lock`)
5. Commits, tags, and pushes — triggering the release workflow

Use `--dry-run` to preview without executing:

```bash
./scripts/version.sh release patch --dry-run
```

**Prerequisite:** The [`gh` CLI](https://cli.github.com) must be installed and
authenticated for changelog generation.

Once the `v*` tag is pushed, the release workflow runs:

| Architecture | OS | Full name |
| --- | --- | --- |
| x86_64 | linux | `x86_64-unknown-linux-gnu` |
| aarch64 | linux | `aarch64-unknown-linux-gnu` (via `cross`) |
| x86_64 | macos | `x86_64-apple-darwin` |
| aarch64 | macos | `aarch64-apple-darwin` |
| x86_64 | windows | `x86_64-pc-windows-msvc` |

| Step | Action |
| --- | --- |
| Test | Full test suite on Ubuntu |
| Build | Cross-compile the five targets above |
| Release | Create GitHub Release with changelog and binary assets |
| Publish | Publish to [crates.io](https://crates.io/crates/aigent) |

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
├─ version:    0.5.0
├─ author:     Wacław Kuśnierczyk
├─ developer:  mailto:waclaw.kusnierczyk@gmail.com
├─ source:     https://github.com/wkusnierczyk/aigent
└─ licence:    MIT https://opensource.org/licenses/MIT
```

[MIT](LICENSE) — see [opensource.org/licenses/MIT](https://opensource.org/licenses/MIT).
