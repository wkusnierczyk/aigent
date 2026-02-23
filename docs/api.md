# API reference

> Back to [README](../README.md)

- [Types](#types)
- [Functions](#functions)
- [Traits](#traits)

Full Rust API documentation with examples is published at
[docs.rs/aigent](https://docs.rs/aigent).

## Types

| Type | Module | Description |
|------|--------|-------------|
| `SkillProperties` | `models` | Parsed skill metadata (name, description, licence, compatibility, allowed-tools) |
| `SkillSpec` | `builder` | Input specification for skill generation (purpose, optional overrides) |
| `BuildResult` | `builder` | Build output (properties, files written, output directory) |
| `ClarityAssessment` | `builder` | Purpose clarity evaluation result (clear flag, follow-up questions) |
| `Diagnostic` | `diagnostics` | Structured diagnostic with severity, code, message, field, suggestion |
| `ScoreResult` | `scorer` | Quality score result with structural and semantic categories |
| `TestResult` | `tester` | Skill activation probe result (query match, score, diagnostics, token cost) |
| `TestSuiteResult` | `test_runner` | Fixture-based test suite result (passed, failed, per-case results) |
| `FormatResult` | `formatter` | `SKILL.md` formatting result (changed flag, formatted content) |
| `AssembleOptions` | `assembler` | Options for skill-to-plugin assembly (output dir, name, validate) |
| `AssembleResult` | `assembler` | Assembly output (plugin directory, skill count) |
| `SkillEntry` | `prompt` | Collected skill entry for prompt generation (name, description, location) |
| `PluginManifest` | `plugin` | Parsed `plugin.json` manifest with path override accessors |
| `AigentError` | `errors` | Error enum: `Parse`, `Validation`, `Build`, `Io`, `Yaml` |
| `Result<T>` | `errors` | Convenience alias for `std::result::Result<T, AigentError>` |

## Functions

| Function | Module | Description |
|----------|--------|-------------|
| `validate(&Path) -> Vec<Diagnostic>` | `validator` | Validate skill directory |
| `validate_with_target(&Path, ValidationTarget)` | `validator` | Validate with target profile |
| `read_properties(&Path) -> Result<SkillProperties>` | `parser` | Parse directory into `SkillProperties` |
| `find_skill_md(&Path) -> Option<PathBuf>` | `parser` | Find `SKILL.md` in directory (prefers uppercase) |
| `parse_frontmatter(&str) -> Result<(HashMap, String)>` | `parser` | Split YAML frontmatter and body |
| `to_prompt(&[&Path]) -> String` | `prompt` | Generate `<available_skills>` XML system prompt |
| `to_prompt_format(&[&Path], PromptFormat) -> String` | `prompt` | Generate prompt in specified format |
| `lint(&SkillProperties, &str) -> Vec<Diagnostic>` | `linter` | Run semantic quality checks |
| `score(&Path) -> ScoreResult` | `scorer` | Score skill against best-practices checklist |
| `test_skill(&Path, &str) -> Result<TestResult>` | `tester` | Probe skill activation against a query |
| `format_skill(&Path) -> Result<FormatResult>` | `formatter` | Format `SKILL.md` with canonical key order |
| `format_content(&str) -> Result<String>` | `formatter` | Format `SKILL.md` content string |
| `assemble_plugin(&[&Path], &AssembleOptions) -> Result<AssembleResult>` | `assembler` | Assemble skills into a plugin |
| `run_test_suite(&Path) -> Result<TestSuiteResult>` | `test_runner` | Run fixture-based test suite |
| `generate_fixture(&Path) -> Result<String>` | `test_runner` | Generate template `tests.yml` from skill metadata |
| `validate_structure(&Path) -> Vec<Diagnostic>` | `structure` | Validate directory structure |
| `detect_conflicts(&[SkillEntry]) -> Vec<Diagnostic>` | `conflict` | Detect cross-skill conflicts |
| `apply_fixes(&Path, &[Diagnostic]) -> Result<usize>` | `fixer` | Apply automatic fixes |
| `build_skill(&SkillSpec) -> Result<BuildResult>` | `builder` | Full build pipeline with post-build validation |
| `derive_name(&str) -> String` | `builder` | Derive kebab-case name from purpose (deterministic) |
| `assess_clarity(&str) -> ClarityAssessment` | `builder` | Evaluate if purpose is clear enough for generation |
| `init_skill(&Path, SkillTemplate) -> Result<PathBuf>` | `builder` | Initialize skill directory with template `SKILL.md` |
| `validate_manifest(&Path) -> Vec<Diagnostic>` | `plugin` | Validate `plugin.json` manifest |
| `validate_hooks(&Path) -> Vec<Diagnostic>` | `plugin` | Validate `hooks.json` configuration |
| `validate_agent(&Path) -> Vec<Diagnostic>` | `plugin` | Validate agent `.md` file |
| `validate_command(&Path) -> Vec<Diagnostic>` | `plugin` | Validate command `.md` file |
| `validate_cross_component(&Path) -> Vec<Diagnostic>` | `plugin` | Run cross-component consistency checks |

## Traits

| Trait | Module | Description |
|-------|--------|-------------|
| `LlmProvider` | `builder::llm` | Text generation provider interface (`generate(system, user) -> Result<String>`) |
