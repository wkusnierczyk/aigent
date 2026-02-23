# `aigent`: the missing toolchain for AI agent skills

AI agents are getting good at following instructions. The bottleneck has shifted: it's no longer about what the model can do, but about how well you package what it should do. That packaging layer is **agent skills** — structured documents that tell an agent what a capability does, when to activate it, and how to use it.

The [Agent Skills open standard](https://agentskills.io), originally defined by Anthropic for Claude Code, codifies this into a simple format: a `SKILL.md` file with YAML frontmatter (name, description, compatibility, allowed tools) and a Markdown body with detailed instructions. The metadata is indexed at session start for fast discovery; the body is loaded on demand, following a progressive-disclosure pattern that keeps the context window lean.

The format is simple. Getting it right at scale is not.

## The problem

When you have a handful of skills, you can eyeball them. When you have dozens — across teams, repositories, and CI pipelines — you need tooling. Names drift from conventions. Descriptions become vague. Activation patterns go untested. Formatting diverges. Nobody catches the skill that fails to trigger because its description doesn't match the query patterns users actually type.

The specification defines *what* a valid skill looks like. The [Python reference implementation](https://github.com/agentskills/agentskills) provides basic validation, but it's a library — not a toolchain you can drop into CI, pre-commit hooks, or a plugin build pipeline.

## What `aigent` does

[**`aigent`**](https://github.com/wkusnierczyk/aigent) is a Rust library, native CLI, and Claude Code plugin that implements the Agent Skills specification as a proper toolchain — the same way a formatter, linter, and test runner enforce conventions in any mature language ecosystem.

It covers the full skill lifecycle:

**Validate** — Check skills against the specification with typed diagnostics, error codes, and JSON output. Run `aigent validate` in CI to catch problems before they reach production. Three severity levels (error, warning, info) and a `--format json` flag for machine consumption.

**Format** — Canonical YAML key ordering, consistent whitespace, idempotent output. `aigent format --check` returns non-zero when files need formatting — drop it into a pre-commit hook.

**Score** — A weighted 0-100 quality score against a best-practices checklist. Structural checks (60 points) verify specification conformance; quality checks (40 points) evaluate description clarity, trigger phrases, naming conventions, and detail. Use it as a CI gate: `aigent score my-skill/ || exit 1`.

**Test** — Fixture-based testing from `tests.yml`. Define input queries, expected match/no-match results, and minimum score thresholds. `aigent test` runs the suite and reports pass/fail. `aigent probe` does single-query dry-runs: "if a user said *this*, would the agent pick up *that* skill?"

**Build** — Generate skills from natural language (deterministic or LLM-enhanced with Anthropic, OpenAI, Google, and Ollama backends). Assemble skills into Claude Code plugins with `aigent build`. Validate entire plugin directories — manifest, hooks, agents, commands, skills, and cross-component consistency — with `aigent validate-plugin`.

![aigent demo](https://github.com/wkusnierczyk/aigent/raw/main/graphics/hello.gif)

## Specification compliance

`aigent` implements every validation rule from the [Anthropic specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices), plus additional checks that go beyond both the specification and the Python reference implementation:

- All name constraints (length, casing, reserved words, XML tags, Unicode NFKC normalization)
- All description constraints (length, non-empty, no XML/HTML)
- Frontmatter structure, delimiter matching, compatibility field limits
- Body length warnings
- Path canonicalization and symlink safety
- Post-build validation

Where the specification and the reference implementation diverge, `aigent` reconciles them and documents the decision. 

See the [compliance section](https://github.com/wkusnierczyk/aigent#compliance) in the README for a detailed three-way comparison.

## Beyond individual skills

Skills don't exist in isolation. As your collection grows, new problems emerge: name collisions, overlapping descriptions that confuse activation, token budgets that exceed context limits. `aigent` handles this with cross-skill conflict detection, token budget estimation, and batch validation across directories.

And skills are just one part of a Claude Code plugin. `aigent`'s `validate-plugin` command checks the full plugin ecosystem: the `plugin.json` manifest, `hooks.json` configuration, agent files, command files, skill subdirectories, and cross-component consistency. Typed diagnostics with error codes (P001-P010 for manifests, H001-H007 for hooks, X001-X008 for cross-component) give you the same deterministic enforcement across the entire plugin structure.

## Where it fits

The agentic AI ecosystem is moving fast. Tools like Anthropic's `plugin-dev` teach you the patterns; `aigent` enforces them. Think of it as the difference between a language tutorial and a linter — you need both, but they serve different purposes.

Other tools in the space, like [AI Agent Skills](https://github.com/skillcreatorai/Ai-Agent-Skills), focus on distribution — installing pre-built skills across multiple agents. `aigent` focuses on authoring quality: making sure what you publish is correct, consistent, and well-tested before it reaches any distribution channel.

## Get started

```bash
# Install
cargo install aigent                        # official distribution at crates.io
brew install wkusnierczyk/aigent/aigent     # homebrew tap

# Create, validate, score
aigent new "process PDF files and extract text" --no-llm
aigent check extracting-text-pdf-files/
aigent score extracting-text-pdf-files/
```

## About

```
aigent: Rust AI Agent Skills Tool
├─ version:    0.6.3
├─ author:     Wacław Kuśnierczyk
├─ developer:  mailto:waclaw.kusnierczyk@gmail.com
├─ source:     https://github.com/wkusnierczyk/aigent
└─ licence:    Apache-2.0 https://www.apache.org/licenses/LICENSE-2.0
```

[Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) — see [apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0).

| Reference | Link |
|-----------|------|
| Sources   | https://github.com/wkusnierczyk/aigent |
| Releases  | https://github.com/wkusnierczyk/aigent/releases |
| Crates    | https://crates.io/crates/aigent |
| Docs      | https://docs.rs/aigent |
