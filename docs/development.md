# Development

> Back to [README](../README.md)

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

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- `cargo` (included with Rust)

## Setup

```bash
git clone https://github.com/wkusnierczyk/aigent.git
cd aigent
cargo build
```

## Optional tooling

```bash
cargo install cargo-edit            # Adds `cargo set-version` for release versioning
```

## Common tasks

```bash
cargo build                         # Build (debug)
cargo build --release               # Build (release)
cargo test                          # Run all tests
cargo clippy -- -D warnings         # Lint (warnings as errors)
cargo fmt                           # Format code
cargo fmt --check                   # Check formatting
```

## Project structure

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

## Versioning

Version is stored in `Cargo.toml` (single source of truth) and read at compile
time via `env!("CARGO_PKG_VERSION")`.

## Milestones

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

## Roadmap

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
