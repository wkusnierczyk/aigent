<table>
  <tr>
    <td>
      <img src="https://raw.githubusercontent.com/wkusnierczyk/aigent-skills/main/graphics/aigent.png" alt="logo" width="300" />
    </td>
    <td>
      <p><strong>aigent</strong>:
      A library and CLI tool for managing AI agent skill definitions.</p>
      <p>Validates, parses, and generates prompts from skill metadata stored in <code>SKILL.md</code> files with YAML frontmatter. Also provides a skill builder for creating new skills from natural language specifications.</p>
    </td>
  </tr>
</table>

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Library Usage](#library-usage)
- [SKILL.md Format](#skillmd-format)
- [Builder Modes](#builder-modes)
- [Specification Compliance](#specification-compliance)
- [CLI Reference](#cli-reference)
- [API Reference](#api-reference)
- [Claude Code Plugin](#claude-code-plugin)
- [Development](#development)
- [CI/CD and Release Workflows](#cicd-and-release-workflows)
- [References](#references)
- [About and Licence](#about-and-licence)

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- `cargo` (included with Rust)

### From crates.io

```bash
cargo install aigent
```

### From source

```bash
git clone https://github.com/wkusnierczyk/aigent.git
cd aigent
cargo install --path .
```

### Pre-built binaries

Download a pre-built binary from the
[latest release](https://github.com/wkusnierczyk/aigent/releases/latest)
(Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64).

### Install script (Linux and macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/wkusnierczyk/aigent/main/install.sh | bash
```

## Quick Start

```bash
# Initialize a new skill
aigent init my-skill/

# Build a skill from a description
aigent build "Process PDF files and extract text" --no-llm

# Validate a skill directory
aigent validate my-skill/

# Read skill properties as JSON
aigent read-properties my-skill/

# Generate XML prompt for LLM injection
aigent to-prompt my-skill/ other-skill/
```

To enable LLM-enhanced generation, set an API key in your environment
(for example, `export ANTHROPIC_API_KEY=sk-...`). Without an API key,
the builder uses deterministic mode, which requires no configuration.
See [Builder Modes](#builder-modes) for details.

## Library Usage

```rust
use std::path::Path;

// Validate a skill directory
let errors = aigent::validate(Path::new("my-skill"));

// Read skill properties
let props = aigent::read_properties(Path::new("my-skill")).unwrap();

// Generate prompt XML
let xml = aigent::to_prompt(&[Path::new("skill-a"), Path::new("skill-b")]);

// Build a skill
let spec = aigent::SkillSpec {
    purpose: "Process PDF files".to_string(),
    no_llm: true,
    ..Default::default()
};
let result = aigent::build_skill(&spec).unwrap();
```

## SKILL.md Format

Skills are defined in `SKILL.md` files with YAML frontmatter and a Markdown body:

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

## When to Use

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
| `description` | yes | What the skill does and when to use it |
| `license` | no | Licence identifier (e.g., `MIT`) |
| `compatibility` | no | Compatible agent platforms |
| `allowed-tools` | no | Tools the skill may use |

### Validation rules

- `name` and `description` are required and non-empty
- `name`: lowercase letters, digits, and hyphens only; maximum 64 characters
- `name`: must not contain reserved words (`anthropic`, `claude`)
- `name`: no XML tags; must match directory name
- `name`: Unicode NFKC normalization applied before validation
- `description`: maximum 1024 characters; no XML/HTML tags
- `compatibility`: maximum 500 characters (if present)
- Body: warning if longer than 500 lines

## Builder Modes

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

### Available models (as of 2026-02-20)

| Provider | Environment Variable | Default Model | Override Variable |
|----------|---------------------|---------------|-------------------|
| Anthropic | `ANTHROPIC_API_KEY` | `claude-sonnet-4-20250514` | `ANTHROPIC_MODEL` |
| OpenAI | `OPENAI_API_KEY` | `gpt-4o` | `OPENAI_MODEL` |
| Google | `GOOGLE_API_KEY` | `gemini-2.0-flash` | `GOOGLE_MODEL` |
| Ollama | `OLLAMA_HOST` | `llama3.2` | `OLLAMA_MODEL` |

OpenAI-compatible endpoints (vLLM, LM Studio, etc.) are supported via
`OPENAI_API_BASE` or `OPENAI_BASE_URL`.

Use `--no-llm` to force deterministic mode regardless of available providers.

## Specification Compliance

Three-way comparison of the
[Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices),
aigent, and the
[Python reference implementation](https://github.com/agentskills/agentskills).

> Table shows key validation rules from the Anthropic specification. Additional
> checks (frontmatter structure, metadata keys, YAML syntax) are implemented
> but not listed as they are standard parser behaviour.

| Rule | aigent | Specification | Python Reference |
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

aigent implements **all** rules from the specification, plus additional checks
(Unicode NFKC normalization, path canonicalization, post-build validation) that
go beyond both the specification and the reference implementation.

## CLI Reference

Run `aigent --help` for built-in documentation. Full API documentation is
available at [docs.rs/aigent](https://docs.rs/aigent).

### Commands

<table>
<tr><th width="280">Command</th><th>Description</th></tr>
<tr><td><code>validate &lt;directory&gt;</code></td><td>Validate a skill directory; exit 0 if valid</td></tr>
<tr><td><code>read-properties &lt;directory&gt;</code></td><td>Output skill properties as JSON</td></tr>
<tr><td><code>to-prompt &lt;directories...&gt;</code></td><td>Generate <code>&lt;available_skills&gt;</code> XML block</td></tr>
<tr><td><code>build &lt;purpose&gt;</code></td><td>Build a skill from natural language</td></tr>
<tr><td><code>init [directory]</code></td><td>Create a template SKILL.md</td></tr>
</table>

### Build Flags

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--name &lt;NAME&gt;</code></td><td>Override the derived skill name</td></tr>
<tr><td><code>--dir &lt;DIRECTORY&gt;</code></td><td>Output directory</td></tr>
<tr><td><code>--no-llm</code></td><td>Force deterministic mode (no LLM)</td></tr>
</table>

### Global Flags

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--about</code></td><td>Show project information</td></tr>
<tr><td><code>--version</code></td><td>Print version</td></tr>
<tr><td><code>--help</code></td><td>Print help</td></tr>
</table>

## API Reference

Full Rust API documentation with examples is published at
[docs.rs/aigent](https://docs.rs/aigent).

### Types

| Type | Module | Description |
|------|--------|-------------|
| `SkillProperties` | `models` | Parsed skill metadata (name, description, licence, compatibility, allowed-tools) |
| `SkillSpec` | `builder` | Input specification for skill generation (purpose, optional overrides) |
| `BuildResult` | `builder` | Build output (properties, files written, output directory) |
| `ClarityAssessment` | `builder` | Purpose clarity evaluation result (clear flag, follow-up questions) |
| `AigentError` | `errors` | Error enum: `Parse`, `Validation`, `Build`, `Io`, `Yaml` |
| `Result<T>` | `errors` | Convenience alias for `std::result::Result<T, AigentError>` |

### Functions

| Function | Module | Description |
|----------|--------|-------------|
| `validate(&Path) -> Vec<String>` | `validator` | Validate skill directory, return errors and warnings |
| `validate_metadata(&HashMap, Option<&Path>) -> Vec<String>` | `validator` | Validate metadata hash, return errors and warnings |
| `read_properties(&Path) -> Result<SkillProperties>` | `parser` | Parse directory into `SkillProperties` |
| `find_skill_md(&Path) -> Option<PathBuf>` | `parser` | Find `SKILL.md` in directory (prefers uppercase) |
| `parse_frontmatter(&str) -> Result<(HashMap, String)>` | `parser` | Split YAML frontmatter and body |
| `to_prompt(&[&Path]) -> String` | `prompt` | Generate `<available_skills>` XML system prompt |
| `build_skill(&SkillSpec) -> Result<BuildResult>` | `builder` | Full build pipeline with post-build validation |
| `derive_name(&str) -> String` | `builder` | Derive kebab-case name from purpose (deterministic) |
| `assess_clarity(&str) -> ClarityAssessment` | `builder` | Evaluate if purpose is clear enough for generation |
| `init_skill(&Path) -> Result<PathBuf>` | `builder` | Initialize skill directory with template SKILL.md |

### Traits

| Trait | Module | Description |
|-------|--------|-------------|
| `LlmProvider` | `builder::llm` | Text generation provider interface (`generate(system, user) -> Result<String>`) |

## Claude Code Plugin

This repository is a
[Claude Code plugin](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code/extensions#custom-slash-commands).
It provides two skills that Claude can use to build and validate SKILL.md files
interactively.

### Skills

| Skill | Description |
|-------|-------------|
| `aigent-builder` | Generates skill definitions from natural language. Triggered by "create a skill", "build a skill", etc. |
| `aigent-validator` | Validates skills against the Anthropic specification. Triggered by "validate a skill", "check a skill", etc. |

Both skills operate in **hybrid mode**: they use the `aigent` CLI when it is
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
├── prompt.rs                       # XML prompt generation
├── main.rs                         # CLI entry point (clap)
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
| M10 | Improvements and Extensions | 🔧 |
| M11 | Builder and Prompt Enhancements | 🔧 |
| M12 | Ecosystem and Workflow | 🔧 |

## CI/CD and Release Workflows

### Continuous integration

Every push to `main` and every pull request runs the CI pipeline on a
**three-OS matrix** (Ubuntu, macOS, Windows):

1. **Formatting** — `cargo fmt --check`
2. **Linting** — `cargo clippy -- -D warnings`
3. **Testing** — `cargo test`
4. **Release build** — `cargo build --release`

### Release workflow

Pushing a version tag (e.g., `v0.1.0`) triggers the release workflow:

1. **Test** — full test suite on Ubuntu
2. **Build** — cross-compile for five targets:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu` (via `cross`)
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-pc-windows-msvc`
3. **Release** — create GitHub Release with changelog and binary assets
4. **Publish** — publish to [crates.io](https://crates.io/crates/aigent)

## References

| Reference | Description |
|-----------|-------------|
| [Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices) | Official specification for SKILL.md format and validation rules |
| [Agent Skills organisation](https://github.com/agentskills) | Umbrella for agent skills tooling |
| [agentskills/agentskills](https://github.com/agentskills/agentskills) | Python reference implementation |
| [anthropics/skills](https://github.com/anthropics/skills) | Anthropic's skills repository |
| [docs.rs/aigent](https://docs.rs/aigent) | Rust API documentation |
| [crates.io/crates/aigent](https://crates.io/crates/aigent) | Package registry |

## About and Licence

```
aigent: AI Agent Skill Builder and Validator
├─ version:    0.1.0
├─ developer:  Wacław Kuśnierczyk
├─ source:     https://github.com/wkusnierczyk/aigent
└─ licence:    MIT https://opensource.org/licenses/MIT
```

[MIT](LICENSE) — see [opensource.org/licenses/MIT](https://opensource.org/licenses/MIT).
