# Changes

## [0.7.0] — 2026-02-23

- Improve probe/score semantics (#171)
- [SEC-4] Reject path traversal (`..`) in plugin.json path overrides (P011) (#151)
- [SEC-5] Enforce 1 MiB file size limit in all plugin validators (#152)
- [SEC-6] Re-check file type before write-back in fixer and formatter (#153)
- Fix non-portable `\n` in sed replacement in `version.sh`

## [0.6.10] — 2026-02-23

- Improve probe/score semantics: Snowball stemmer, synonym expansion, graduated scoring, proportional structural scoring, shared trigger phrases (#168)
- **Breaking:** Probe scores change due to improved stemmer, synonym expansion, and graduated trigger/name scoring
- **Breaking:** Score totals change due to proportional structural scoring (10 per check instead of all-or-nothing 60)
- Add integration tests against anthropics/skills repo (#167)

## [0.6.9] — 2026-02-23

- Clarify upgrade scope: [fix]/[info] tags, rule IDs, --dry-run (#165)

## [Unreleased]

- Clarify upgrade scope: `[fix]`/`[info]` tags, rule IDs (U001–U003), `--dry-run` flag, docs (#163)
- **Breaking:** `upgrade --format json` now emits structured suggestion objects (`code`, `kind`, `message`) instead of bare strings
- **Breaking:** `upgrade` now exits 0 when only informational suggestions remain (previously exited 1 for any suggestion)

## [0.6.8] — 2026-02-23

- Automate Homebrew formula updates on release (#162)

## [0.6.7] — 2026-02-23

- Clarify expect() policy for static regex initializers (#161)

## [0.6.6] — 2026-02-23

- Modularize main.rs into src/cli/ modules (#159)

## [0.6.5] — 2026-02-23

- Remove dev/ from repo, move plugin-dev.md to docs/ (#157)
- Address review items #1, #6, #8: tagline, See also, README split (#156)

## [0.6.4] — 2026-02-23

- Add demo video and README embed (#155)

## [0.6.3] — 2026-02-23

- Improve --help output: sort commands, show 'format' as primary name (#150)

## [0.6.2] — 2026-02-23

- Fix upgrade --apply adding non-spec fields that regress score (#146)
- Fix build output to pass validate-plugin (#145)
- Fix probe output: wrap long values aligned to value column (#144)
- Add Homebrew install instructions to README (#140)

## [0.6.1] — 2026-02-22

- Migrate license from MIT to Apache 2.0 (#133)

## [0.6.0] — 2026-02-22

- M15: Plugin component validation (#130)

## [0.5.0] — 2026-02-22

- Move build matrix table from CI to release section in README (#127)
- Add `release` subcommand to version.sh (#126)
- Default to current directory when no skill path is given (#125)
- Fix version.sh: case-sensitive heading match and missing verification (#123)
- Show diff in `format --check` output (#122)
- Add `properties` as alias for `read-properties` (#121)

## [0.4.1] — 2026-02-22

### Changed

- Comprehensive README rewrite: added agent skills introduction, expanded
  CLI reference with exit codes table, command flags, severity levels,
  validation targets, and release workflow instructions; fixed incorrect
  exit code documentation for `validate` and `check`; added backward
  compatibility aliases table with `fmt` → `format`; added safer
  install-script alternative; moved hard-coded date out of section
  headings (#107)


## [0.4.0] — 2026-02-21

### Breaking

- `read_body()` now returns `Result<String>` instead of `String`, propagating
  IO and parse errors instead of silently returning an empty string (#88)

### Added

- Symlink safety: new `fs_util` module with `is_regular_file()`,
  `is_regular_dir()`, `is_symlink()` helpers; all file-system walks use
  symlink-safe checks; `S005` diagnostic for symlinks in skill dirs (#87)
- Path traversal guard: `S006` diagnostic rejects `..` components in
  reference links (#89)
- File size cap: `read_file_checked()` helper rejects files exceeding
  1 MiB, preventing memory exhaustion (#90)
- Discovery error collection: `discover_skills_verbose()` and
  `collect_skills_verbose()` collect warnings instead of silently
  skipping unreadable paths; CLI commands print warnings to stderr (#91, #92)
- CRLF normalization in formatter: `format_content()` normalizes `\r\n`
  to `\n` before byte-offset arithmetic (#94)
- Recursion depth limits (10 levels) in `copy_dir_recursive` and
  `discover_skills_recursive` to prevent stack overflow (#96)
- Pre-tokenized Jaccard similarity in conflict detection eliminates
  per-pair allocation in the O(n²) loop (#95)
- Formatter comment anchoring: standalone YAML comments stay in position
  during key reordering instead of traveling with the preceding key (#103)
- Unified `scripts/version.sh` with `show`, `set`, and `bump` subcommands,
  replacing `scripts/bump-version.sh` (#102)

### Fixed

- TOCTOU race in `build_skill()` and `init_skill()`: replaced
  check-then-write with atomic `create_new(true)` (#93)

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
  (`compatibility`, trigger phrases)
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
