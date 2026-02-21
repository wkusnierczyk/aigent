# LinkedIn Post — aigent v0.3.0

---

Agent Skills are how you teach AI coding assistants what they know. In Claude Code, a SKILL.md file — a Markdown document with YAML frontmatter — tells the agent what a skill does, when to activate it, and how to use it. As teams build more skills, quality becomes a bottleneck: inconsistent naming, vague descriptions, untested activation patterns, and no way to enforce standards in CI. The official reference implementation validates structure, but if you're authoring skills at scale, you need more.

**aigent** is a Rust library, CLI tool, and Claude Code plugin for working with Agent Skills.

It fully implements the Anthropic agent skill specification and aligns with the Python reference implementation — reconciling the cases where the two diverge.

**What you get:**

*Enforce quality in CI* — Deterministic validation with typed error codes, JSON output, and non-zero exit codes. A weighted 0–100 quality score you can use as a CI gate. Idempotent formatting with `--check` mode for pre-commit hooks.

*Build and test skills* — Generate skills from natural language (deterministic or LLM-enhanced with 4 provider backends). Scaffold with templates. Probe activation against sample queries. Run fixture-based regression tests from `tests.yml`.

*Manage skill collections* — Cross-skill conflict detection (name collisions, description overlap). Token budget estimation. Batch validation across directories. Assemble skills into Claude Code plugins with `aigent build`.

*Use it from Claude Code* — aigent ships as a Claude Code plugin with 3 skills (builder, validator, scorer) and a PostToolUse hook that auto-validates every SKILL.md you write. Each skill works with or without the CLI installed.

**New in 0.3.0:** `fmt` (idempotent formatting), `check` (validate + semantic lint), `build` (skill-to-plugin assembly), and `test` (fixture-based test suites).

**Complementary to plugin-dev** — Anthropic's plugin-dev plugin teaches the full Claude Code plugin ecosystem (skills, commands, agents, hooks, MCP, settings). aigent enforces one part of that ecosystem — SKILL.md files — with deterministic tooling. Think rustfmt+clippy vs. The Rust Book: you need both.

**On the roadmap:** extending aigent's validation to hooks, agents, commands, and plugin manifests — the same deterministic enforcement, applied across the full plugin ecosystem.

**Links:**

— GitHub: https://github.com/wkusnierczyk/aigent
— Releases: https://github.com/wkusnierczyk/aigent/releases
— Crate: https://crates.io/crates/aigent
— Docs: https://docs.rs/aigent

Open source under the MIT license.

#AgentSkills #Rust #OpenSource #AI #ClaudeCode #Anthropic #CLI #DeveloperTools #AIAgents
