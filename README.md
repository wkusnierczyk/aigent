# aigent

[![CI](https://github.com/wkusnierczyk/aigent/actions/workflows/ci.yml/badge.svg)](https://github.com/wkusnierczyk/aigent/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aigent)](https://crates.io/crates/aigent)
[![docs.rs](https://docs.rs/aigent/badge.svg)](https://docs.rs/aigent)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A Rust library, CLI, and Claude Code plugin for managing AI agent skill
definitions (SKILL.md files). Implements the
[Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
with validation, prompt generation, and skill building.

## Installation

```bash
# From crates.io
cargo install aigent

# From source
cargo install --path .
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

## Builder Modes

The skill builder operates in two modes:

**Deterministic** — Always available, zero configuration. Uses heuristic
rules to derive skill names (gerund form, kebab-case), descriptions, and
markdown bodies. Output is formulaic but valid.

**LLM-enhanced** — Auto-detected via environment variables. Produces
richer, more natural output. Each generation step (name, description, body)
independently falls back to deterministic on LLM failure.

Provider detection order:

| Priority | Environment Variable | Provider |
|:--------:|---------------------|----------|
| 1 | `ANTHROPIC_API_KEY` | Anthropic Claude |
| 2 | `OPENAI_API_KEY` | OpenAI |
| 3 | `GOOGLE_API_KEY` | Google Gemini |
| 4 | `OLLAMA_HOST` | Ollama (local) |

Model overrides: `ANTHROPIC_MODEL`, `OPENAI_MODEL`, `GOOGLE_MODEL`,
`OLLAMA_MODEL`.

Use `--no-llm` to force deterministic mode regardless of available
providers.

## Spec Compliance

Three-way comparison of the Anthropic agent skill specification, aigent,
and the Python reference implementation.

> Table shows key validation rules from the Anthropic spec. Additional checks
> (frontmatter structure, metadata keys, YAML syntax) are implemented but not
> listed as they are standard parser behavior.

| Rule | Spec | aigent | Python Ref |
|------|:----:|:------:|:----------:|
| Name ≤ 64 chars | ✅ | ✅ | ✅ |
| Name: lowercase + hyphens | ✅ | ✅ | ✅ |
| Name: no XML tags | ✅ | ✅ | ❌ |
| Name: no reserved words | ✅ | ✅ | ❌ |
| Name: Unicode NFKC | — | ✅ | ❌ |
| Description: non-empty | ✅ | ✅ | ✅ |
| Description ≤ 1024 chars | ✅ | ✅ | ✅ |
| Description: no XML tags | ✅ | ✅ | ❌ |
| Frontmatter `---` delimiters | ✅ | ✅ | ✅ |
| Compatibility ≤ 500 chars | ✅ | ✅ | ❌ |
| Body ≤ 500 lines warning | ✅ | ✅ | ❌ |
| Prompt XML format | ✅ | ✅ | ✅ |
| Path canonicalization | — | ✅ | ✅ |
| Post-build validation | — | ✅ | ❌ |

## CLI Reference

| Command | Description |
|---------|-------------|
| `validate <dir>` | Validate a skill directory; exit 0 if valid |
| `read-properties <dir>` | Output skill properties as JSON |
| `to-prompt <dirs...>` | Generate `<available_skills>` XML block |
| `build <purpose>` | Build a skill from natural language |
| `init [dir]` | Create a template SKILL.md |

### Build Flags

| Flag | Description |
|------|-------------|
| `--name <NAME>` | Override the derived skill name |
| `--dir <DIR>` | Output directory |
| `--no-llm` | Force deterministic mode (no LLM) |

### Global Flags

| Flag | Description |
|------|-------------|
| `--about` | Show project information |
| `--version` | Print version |
| `--help` | Print help |

## License

MIT — see [LICENSE](LICENSE) for details.
