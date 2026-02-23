# Developer community posts

## r/ClaudeAI

**Title:** `aigent` — a Rust toolchain for Agent Skills (validate, format, score, test, build)

Check out `aigent`, an open-source tool that implements the Agent Skills specification as a full development toolchain.

If you're writing skills for Claude Code, you know the format: `SKILL.md` with YAML frontmatter. Simple to write one. Harder to keep dozens consistent across teams and CI pipelines.

`aigent` gives you the same kind of enforcement you'd expect from a mature language ecosystem:

- **Validate** against the specification with typed diagnostics and JSON output
- **Format** with canonical key ordering (`--check` for pre-commit hooks)
- **Score** 0–100 against a best-practices checklist (CI-gatable)
- **Test** activation patterns with fixture-based test suites
- **Build** skills from natural language and assemble into Claude Code plugins
- **Validate plugins** — manifest, hooks, agents, commands, skills, cross-component consistency

Fully compliant with the Anthropic specification. Designed to complement `plugin-dev` — it teaches the patterns, `aigent` enforces them.

Ships as a native CLI, Rust library, and Claude Code plugin. Built entirely with Claude.

- GitHub: https://github.com/wkusnierczyk/aigent
- Blog post: [TODO: dev.to link]

`brew install wkusnierczyk/aigent/aigent` or `cargo install aigent`

It's feature-rich but still early stage — feedback welcome, especially on what's missing or could be improved.

---

## r/rust

**Title:** `aigent` — Rust CLI for validating, formatting, scoring, and testing AI agent skill files

Check out `aigent`, a Rust CLI + library for working with AI agent skill definitions (`SKILL.md` files with YAML frontmatter). It implements the Agent Skills open standard (originally from Anthropic for Claude Code).

The tool covers the full lifecycle: validation with typed diagnostics and error codes, idempotent formatting, a weighted 0-100 quality scorer, fixture-based testing, skill generation (deterministic or LLM-enhanced), and plugin assembly. It also validates entire Claude Code plugin directories — manifest, hooks, agents, commands, skills, and cross-component consistency.

Some Rust-specific notes:

- `clap` derive macros for the CLI, `serde` for YAML/JSON, `thiserror` for errors
- No `unwrap()` in library code — errors propagate with `?` throughout
- 527 unit tests + 133 CLI integration tests + 27 plugin tests
- Single binary, cross-compiled for Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
- Homebrew tap + crates.io + install script
- Apache 2.0

The project was milestone-driven (15 milestones from scaffolding to plugin ecosystem validation), with structured planning, reviews, and test coverage at every stage.

- GitHub: https://github.com/wkusnierczyk/aigent
- crates.io: https://crates.io/crates/aigent
- docs.rs: https://docs.rs/aigent
- Blog post: [TODO: dev.to link]

It's feature-rich but still early stage. Would especially value code reviews from experienced Rust engineers — the entire codebase is open and I'd welcome PRs, issues, or just candid feedback on the code, the CLI design, or the project approach.

---

## Hacker News

**Title:** Show HN: Rust CLI for building AI agent skills to the Anthropic spec

**URL:** https://github.com/wkusnierczyk/aigent

**Text:**

`aigent` is a Rust CLI that implements the Agent Skills open standard (https://agentskills.io) — the format used by Claude Code for packaging reusable instructions that AI agents discover and invoke automatically.

It validates, formats, scores, tests, and assembles SKILL.md files, and validates full Claude Code plugin directories. Think of it as the linter/formatter/test-runner layer for AI agent skills.

Fully compliant with the specification. 500+ tests. Single binary for Linux, macOS, Windows. Apache 2.0.

Blog post: [TODO: dev.to link]

---

## Lobsters

**Title:** `aigent`: Rust toolchain for AI agent skills — validate, format, score, test, build

**URL:** https://github.com/wkusnierczyk/aigent

**Tags:** rust, ai, cli
