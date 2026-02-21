# Changes

## [0.3.0] — 2026-02-21

### Breaking

- Renamed CLI subcommands: `validate` → `check`, `generate` → `new`,
  `format` → `fmt`, `to-prompt` → `prompt` (#76)
- Old names (`validate`, `generate`, `format`, `lint`) retained as hidden
  aliases for backward compatibility, but may be removed in a future release
- `tester` scoring formula changed to weighted combination:
  name 15%, description 25%, trigger 20%, doc-comment 40% (#79)
- `CheckResult` now uses distinct `fail_label` for pass/fail display (#78)

### Added

- `build` subcommand: assemble one or more skills into a Claude Code plugin
  artifact with `plugin.json`, reference files, and structured warnings (#83)
- `test` subcommand: fixture-based skill testing with YAML test suites,
  `--generate` for auto-generating fixtures, and `--min-score` threshold (#84)
- `fmt` subcommand: canonical SKILL.md formatting — key reordering, trailing
  whitespace removal, blank line collapsing, idempotent output (#76)
- `upgrade --full`: combined `--apply` + `--suggest` in a single pass (#80)
- `check` superset command: runs `validate` + `lint` together, replacing
  separate invocations (#76)
- `AssembleWarning` public type for structured build warnings
- `scripts/bump-version.sh`: portable version bump across Cargo.toml,
  plugin.json, CHANGES.md, and Cargo.lock (#82)

### Changed

- `upgrade --apply` uses YAML AST-preserving insertion for metadata blocks,
  handling partial metadata, comments, and custom indentation correctly (#81)
- `BuildResult` includes structured `warnings` vec instead of `eprintln` (#45)
- `read_body()` deduplicated into `parser.rs` as single source of truth (#82)

### Fixed

- Path traversal validation in assembler (`is_unsafe_name`) rejects `..`
  components and absolute paths
- JSON injection prevented: plugin.json generation uses `serde_json` instead
  of string formatting
- `formatter` handles empty frontmatter without panicking
- `main.rs` exit codes: `test --generate` and `fmt` properly track errors
- `main.rs` parse error diagnostics shown in `check --no-validate`
- Delimiter matching uses `trim_end()` for robustness
- Doc-comment activation thresholds corrected to 0.4/0.15 in tester
- `bump-version.sh` uses portable `sedi()` wrapper for GNU/BSD sed

## [0.2.3] — 2026-02-20

### Added

- `score` command: rate skills 0–100 against a best-practices checklist
  (structural 60pts + quality 40pts)
- `test` command: simulate skill activation with query matching
- `upgrade` command: detect and apply missing best-practice fields
  (`compatibility`, `metadata.version`, `metadata.author`, trigger phrases)
- `doc` command: generate skill catalogs with diff-aware output
- `init` command: scaffold new skill directories from templates
- `lint` subcommand and `--lint` flag for semantic quality checks (I001–I005)
- `--structure` flag for directory structure validation (S001–S004)
- Cross-skill conflict detection (name, description, tool overlap)
- Watch mode (`--features watch`) for live re-validation on file changes
- JSON output format (`--format json`) for CI integration
- Claude Code plugin skill: `aigent-scorer`
- Comprehensive README with command examples and scoring documentation

### Changed

- Multi-skill prompt generation with `to-prompt` accepting multiple directories
- Builder supports building from existing SKILL.md files

### Fixed

- Jaccard similarity now case-insensitive (matching doc comment contract)
- `doc` catalog resolves `entry.location` to parent directory correctly
- `upgrade --apply` handles partial metadata blocks (inserts missing keys
  under existing `metadata:` rather than skipping)
- Windows CI: platform-specific script permission checks (`#[cfg(unix)]`)

## [0.1.0] — 2026-02-20

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
- Claude Code plugin with two skills (`aigent-builder`, `aigent-validator`)
  operating in hybrid CLI/prompt-only mode
- Install script (`install.sh`) for non-Rust users
- `--about` flag displaying project info from compile-time metadata
- Cross-platform support: Linux (x86_64, aarch64), macOS (x86_64, aarch64),
  Windows (x86_64)
- CI pipeline: formatting, linting, testing, release builds on all platforms
- Release workflow: automated GitHub Releases and crates.io publishing on tag push
