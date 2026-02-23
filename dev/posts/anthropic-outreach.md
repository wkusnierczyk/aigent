# Anthropic outreach

## Email to devrel@anthropic.com

Subject: `aigent` — open-source toolchain for the Agent Skills specification

Hi,

I'd like to introduce `aigent`, an open-source toolchain built to be fully compliant with the Anthropic agent skill specification. It's designed to complement Anthropic's own tooling: `plugin-dev` teaches the patterns, `aigent` enforces them deterministically.

`aigent` implements every validation rule from the specification, reconciles cases where the specification and the Python reference implementation diverge, and adds checks that go beyond both (Unicode NFKC normalization, symlink safety, path traversal guards, post-build validation). It provides deterministic validation, formatting, scoring, testing, and plugin assembly for `SKILL.md` files — the kind of infrastructure that lets teams enforce skill quality in CI, pre-commit hooks, and build pipelines.

Beyond individual skills, `aigent` validates entire Claude Code plugin directories — manifest, hooks, agents, commands, skills, and cross-component consistency — with typed diagnostics and error codes.

It ships as a native CLI (Homebrew, crates.io, pre-built binaries for Linux/macOS/Windows), a Rust library, and a Claude Code plugin with three hybrid skills that work with or without the CLI installed.

The project was developed with rigorous engineering discipline — milestone-driven planning, structured code reviews, and comprehensive test coverage (500+ tests across unit, integration, and plugin levels). Claude was used extensively at every stage of the development cycle: architecture, implementation, review, and documentation. `aigent` is itself a product of the Claude ecosystem.

Demo: https://github.com/wkusnierczyk/aigent#readme (GIF in the README)  
Blog post: https://dev.to/wkusnierczyk/aigent-toolchain-for-ai-agent-skills-3hib  
Repository: https://github.com/wkusnierczyk/aigent  
Crate: https://crates.io/crates/aigent

While feature-rich, `aigent` is still an early-stage project and I'd welcome any feedback on the functionality, usefullness, implementation of the tool. Happy to discuss integration opportunities — for example, listing in a future plugin directory.

Best,
Wacław Kuśnierczyk

---

## Discord / Forum post

**`aigent` — a Rust toolchain for Agent Skills**

Check out `aigent`, an open-source tool that implements the Anthropic agent skill specification as a full development toolchain.

What it does:
- **Validate** skills against the specification with typed diagnostics and JSON output for CI
- **Format** `SKILL.md` files with canonical key ordering (`--check` for pre-commit hooks)
- **Score** skills 0–100 against a best-practices checklist (CI-gatable)
- **Test** activation patterns with fixture-based test suites
- **Build** skills from natural language and assemble them into Claude Code plugins
- **Validate plugins** — manifest, hooks, agents, commands, skills, cross-component consistency

It's fully compliant with the specification and goes beyond the Python reference implementation in areas like Unicode NFKC normalization, symlink safety, and post-build validation.

Install: `brew install wkusnierczyk/aigent/aigent` or `cargo install aigent`

- GitHub: https://github.com/wkusnierczyk/aigent
- Blog post: https://dev.to/wkusnierczyk/aigent-toolchain-for-ai-agent-skills-3hib

Feedback welcome!

---

## X post

`aigent` — the missing toolchain for AI agent skills.

Validates, formats, scores, tests, and assembles SKILL.md files to the @AnthropicAI agent skill spec.

Full plugin ecosystem validation. Rust-native CLI. Claude Code plugin.

Open source (Apache 2.0).

https://github.com/wkusnierczyk/aigent

@alexalbert__
