# Changes

## [0.1.0] — Unreleased

### Added

- Core library: error types (`AigentError`), data model (`SkillProperties`),
  YAML frontmatter parser, skill directory validator, XML prompt generator,
  and skill builder
- CLI tool with subcommands: `validate`, `read-properties`, `to-prompt`,
  `build`, `init`
- Full compliance with the
  [Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices):
  name/description validation, XML tag rejection, reserved word checks,
  Unicode NFKC normalization, body-length warnings
- Dual-mode skill builder: deterministic (zero-config) and LLM-enhanced
  (Anthropic, OpenAI, Google, Ollama) with per-function graceful fallback
- `--about` flag displaying project info from compile-time metadata
- Cross-platform support: Linux (x86_64, aarch64), macOS (x86_64, aarch64),
  Windows (x86_64)
- CI pipeline: formatting, linting, testing, release builds on all platforms
