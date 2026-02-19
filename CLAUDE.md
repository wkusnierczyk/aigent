# aigent — Development Guide

## Overview

Rust library + CLI + Claude Code plugin for managing AI agent skill definitions (SKILL.md files).
Implements the [Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices).

## Project Structure

```
src/
├── lib.rs          # Library root — re-exports public API
├── errors.rs       # Error types (thiserror)
├── models.rs       # SkillProperties (serde)
├── parser.rs       # SKILL.md frontmatter parser (serde_yaml)
├── validator.rs    # Metadata & directory validator
├── prompt.rs       # XML prompt generation
├── builder.rs      # Skill builder (deterministic + LLM)
└── main.rs         # CLI entry point (clap)
```

## Commands

```bash
cargo build                         # Build debug
cargo build --release               # Build release
cargo test                          # Run all tests
cargo clippy -- -D warnings         # Lint (warnings = errors)
cargo fmt                           # Format code
cargo fmt --check                   # Check formatting
```

## Architecture

- **Error handling**: `thiserror` enum `AigentError` with `Result<T>` alias. No `unwrap()` in library code.
- **Serialization**: `serde` derive macros for YAML/JSON. `#[serde(rename)]` for kebab-case fields.
- **CLI**: `clap` derive macros with `#[command(subcommand)]`.
- **Unicode**: `unicode-normalization` crate for NFKC.
- **Version**: compile-time `env!("CARGO_PKG_VERSION")`, single source of truth in Cargo.toml.

## Coding Conventions

- Idiomatic Rust: use `Result<T, E>`, pattern matching, iterators
- No `unwrap()` or `expect()` in `src/lib.rs` and modules — propagate errors with `?`
- `unwrap()` allowed in tests and `main.rs` for known-safe operations
- `#[must_use]` on functions returning values that shouldn't be ignored
- Public items must have doc comments (`///`)

## Milestones

Project tracked at: https://github.com/users/wkusnierczyk/projects/39

M1: Project Scaffolding → M2: Errors & Models → M3: Parser → M4: Validator →
M5: Prompt → M6: CLI → M7: Builder → M8: Docs → M9: Claude Code Plugin
