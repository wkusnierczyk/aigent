## M11 Code Review

### Findings

1. High: `build --template` is exposed in CLI/API but not applied during build output generation.
   - References: `src/main.rs:143`, `src/main.rs:375`, `src/builder/mod.rs:45`, `src/builder/mod.rs:149`, `src/builder/mod.rs:169`
   - `SkillSpec.template` is threaded from CLI into `SkillSpec`, but `build_skill()` always writes a single generated `SKILL.md` and never calls `template::template_files(...)`.
   - Repro: `aigent build "...\" --template code-skill ...` creates only `SKILL.md` (no `scripts/run.sh`).
   - Impact: advertised feature (#58) is partially non-functional; users selecting non-minimal templates in `build` get minimal output.

2. High: hook command uses single-quoted `$TOOL_INPUT`, so JSON payload is never expanded.
   - Reference: `hooks/hooks.json:10`
   - The command uses `echo '$TOOL_INPUT'`, which passes a literal string to `jq`, so file extraction returns empty and validation hook never triggers.
   - Impact: continuous validation hook (#62) is effectively disabled in normal usage.

3. Medium: `code-skill` template script is not made executable on `init`.
   - References: `src/builder/template.rs:72`, `src/builder/template.rs:270`, `src/builder/mod.rs:286`
   - Template generates `scripts/run.sh` with shebang and strict mode, but `init_skill()` writes it with default file mode (e.g., `0644`) and never sets execute bit.
   - Impact: generated quick-start command `./scripts/run.sh` fails until users manually `chmod +x`.

### Testing Gaps

1. Missing integration test for `build --template code-skill` asserting expected extra files are created (for example `scripts/run.sh`).
2. Missing hook behavior test that validates `$TOOL_INPUT` parsing actually extracts `file_path`.
3. Missing executable-permission test for generated `scripts/run.sh` in `code-skill` template.

---

## M11 Code Review — Full Review

**Branch**: `dev/m11`
**Commit**: `baba66c` — "M11: Builder & prompt enhancements"
**Delta**: +1907 lines across 12 files (1 commit on top of M10 merge)
**Reviewer**: Claude (automated)
**Date**: 2025-06-14

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --check` | ✅ Clean |
| `cargo clippy -- -D warnings` | ✅ Clean |
| `cargo test` | ✅ 331 passed (255 unit + 54 cli + 21 plugin + 1 doc-test) |
| `cargo doc --no-deps` | ✅ Clean |

### Scope

M11 addresses 8 issues across prompt enhancements, builder templates, install hardening, and plugin hooks:

| Issue | Title | Status |
|-------|-------|--------|
| #53 | Multi-format prompt output | ✅ Implemented |
| #54 | Token budget estimation | ✅ Implemented |
| #55 | SkillEntry intermediate representation | ✅ Implemented |
| #56 | Skill templates for `init` | ✅ Implemented |
| #57 | Interactive build mode | ✅ Implemented |
| #58 | Template-aware `build` | ⚠️ Partial (see Prior F1) |
| #61 | Install script checksum verification | ✅ Implemented |
| #62 | PostToolUse validation hook | ✅ Implemented (see Prior F2) |

### Changed Files

| File | Lines | Summary |
|------|-------|---------|
| `src/prompt.rs` | 597 | SkillEntry, PromptFormat, collect_skills, estimate_tokens, format_budget, 4 renderers |
| `src/builder/mod.rs` | 708 | SkillSpec.template, init_skill(dir,tmpl), interactive_build(), confirm() |
| `src/builder/template.rs` | 475 | 6 SkillTemplate variants, template_files(), skill_template() compat |
| `src/lib.rs` | 67 | Re-exports for new public API |
| `src/main.rs` | 483 | PromptOutputFormat, --format/--budget/--output, --interactive, --template |
| `install.sh` | 99 | SHA256 checksum download and verification |
| `.github/workflows/release.yml` | 146 | sha256sum generation step |
| `hooks/hooks.json` | 14 | PostToolUse hook for Write\|Edit → aigent validate |
| `skills/aigent-builder/SKILL.md` | 94 | Added `context: fork` |
| `tests/cli.rs` | +211 | 12 new CLI tests |
| `tests/plugin.rs` | +91 | 8 new plugin tests |
| `Cargo.toml` | 34 | No new dependencies |

### Plan Conformance

All 8 issues are addressed in code. Issue #58 (template-aware build) is partial: `SkillSpec.template` is threaded through the API but `build_skill()` never calls `template_files()`, so only `init` benefits from templates.

Plan review findings resolution:
- **F3** (template variant doc examples): Deferred — no examples in SKILL.md yet.
- **F5** (YAML special-char escaping): Fixed — `yaml_quote()` added in `src/prompt.rs`.
- **F7** (token estimation documented): Documented — `estimate_tokens()` doc comment notes `chars/4` heuristic.
- **F8** (format_budget threshold): Resolved — 4000-token warning threshold hardcoded.
- **F11** (install.sh POSIX compliance): Resolved — uses `#!/bin/sh`, degrades gracefully.

### Prior Finding Validation

**Prior F1 (High): `build --template` not applied** — **CONFIRMED**

`build_skill()` at `src/builder/mod.rs:149` receives `SkillSpec` with `.template` field but never branches on it. The function always writes a single `SKILL.md` regardless of template. `template_files()` is only called by `init_skill()`. Users running `aigent build "desc" --template code-skill` get minimal output without `scripts/run.sh`.

**Prior F2 (High): Hook `$TOOL_INPUT` quoting** — **CONTEXT-DEPENDENT**

The hook command at `hooks/hooks.json:10` uses:
```
echo '$TOOL_INPUT' | jq -r '.file_path // empty'
```
Whether `$TOOL_INPUT` is expanded depends on Claude Code's variable substitution model. If Claude Code substitutes `$TOOL_INPUT` *before* shell interpretation, the single quotes prevent word splitting of the JSON payload (correct). If Claude Code passes it as a shell environment variable, single quotes prevent expansion (bug). The [Claude Code hooks documentation](https://docs.anthropic.com/en/docs/claude-code/hooks) indicates substitution happens before shell execution, making single quotes correct for preventing shell interpretation of JSON special characters. **Reclassified: Low risk** — but a comment in `hooks.json` explaining the quoting rationale would help maintainability.

**Prior F3 (Medium): No execute bit on `scripts/run.sh`** — **CONFIRMED**

`init_skill()` at `src/builder/mod.rs:286` uses `fs::write()` which creates files with mode `0644` on Unix. The `code-skill` template generates `scripts/run.sh` with `#!/usr/bin/env bash` shebang but the file is not executable. Users must manually `chmod +x scripts/run.sh`.

### Additional Findings

**F4 (Medium): `init_skill()` breaking API change**

`init_skill()` signature changed from `init_skill(dir: &Path) -> Result<PathBuf>` to `init_skill(dir: &Path, tmpl: SkillTemplate) -> Result<PathBuf>`. This is a breaking change for any downstream consumer of the library API. The `skill_template()` wrapper function was preserved for backward compatibility of content generation, but the `init_skill()` entry point itself has a new required parameter.

- References: `src/builder/mod.rs:269`
- Impact: Library consumers must update call sites. Since pre-1.0, acceptable with documentation.
- Recommendation: Consider `impl Into<Option<SkillTemplate>>` or a default parameter pattern for smoother migration, or document the break in CHANGELOG.

**F5 (Medium): Interactive build always forces deterministic mode**

`interactive_build()` at `src/builder/mod.rs:200` hardcodes `spec.no_llm = true` regardless of the caller's intent. This means interactive mode can *never* use LLM generation, even if the user explicitly enables it. The interactive flow collects a description from the user but then always produces deterministic (template-based) output.

- References: `src/builder/mod.rs:200`
- Impact: Users expecting interactive + LLM-enhanced build get deterministic output silently.
- Recommendation: Either document this limitation or allow `--no-llm` to be independently controlled.

**F6 (Medium): `--output` always overwrites then checks unchanged**

The `to-prompt --output` implementation at `src/main.rs:310-340` always writes the file, then checks if content matches the previous version. This means:
1. File mtime always updates even when content is unchanged.
2. The "unchanged" detection reads the file twice (before write to compare, or after).
3. CI pipelines relying on mtime for cache invalidation will always see the file as changed.

- References: `src/main.rs:310-340`
- Impact: Minor — CI false-positives on prompt drift detection.
- Recommendation: Read existing file first, compare, skip write if identical.

**F7 (Low): `yaml_quote()` misses some YAML edge cases**

`yaml_quote()` at `src/prompt.rs:560-575` handles colons, hashes, and leading/trailing spaces but doesn't handle:
- Strings starting with `&`, `*`, `!`, `%`, `@`, `` ` `` (YAML indicators)
- Multi-line strings (no folding/literal block support)
- Boolean-like strings (`yes`, `no`, `true`, `false`, `on`, `off`)

- References: `src/prompt.rs:560-575`
- Impact: Low — skill names/descriptions unlikely to hit these cases.
- Recommendation: Add a comment noting known limitations, or use a YAML library for serialization.

**F8 (Low): `format_budget()` estimation accuracy**

`estimate_tokens()` uses `chars / 4` which is a rough heuristic. For skill descriptions with heavy punctuation, code blocks, or non-ASCII characters, this can under- or over-estimate by 2-3x. The 4000-token warning threshold is hardcoded without configuration.

- References: `src/prompt.rs:524-535`, `src/prompt.rs:540-555`
- Impact: Low — advisory output only, doesn't affect functionality.
- Recommendation: Document the heuristic's limitations in `--help` or `--budget` output header.

**F9 (Low): Hook assumes `jq` is installed**

The PostToolUse hook at `hooks/hooks.json:10` pipes through `jq` without checking if it's available. On systems without `jq`, the hook will fail silently or produce an error that may confuse users.

- References: `hooks/hooks.json:10`
- Impact: Low — `jq` is widely available; hook failure is non-blocking.
- Recommendation: Add a `command -v jq` guard or document `jq` as a dependency in README.

### Observations

1. **Well-structured IR pattern**: `SkillEntry` as an intermediate representation between discovery and rendering is a clean separation of concerns. All 4 format renderers consume the same `Vec<SkillEntry>`, making new formats trivial to add.

2. **Comprehensive test coverage**: 331 tests with good distribution across unit (255), CLI integration (54), and plugin validation (21). Template tests are particularly thorough, covering all 6 variants.

3. **BufRead trait injection**: `interactive_build()` accepting `&mut dyn BufRead` instead of reading stdin directly is excellent for testability — tests use `Cursor<Vec<u8>>` to simulate user input.

4. **Backward-compatible wrapper**: `skill_template()` preserving the old API while `template_files()` provides the new multi-file interface is a good migration pattern.

5. **Graceful degradation in install.sh**: The checksum verification tries `sha256sum`, falls back to `shasum -a 256`, and ultimately skips with a warning. This handles macOS (no `sha256sum` by default) and minimal containers.

6. **No new dependencies**: M11 adds significant functionality without any new crate dependencies — all features built on existing `serde`, `clap`, and standard library.

7. **Clean clippy/fmt**: Zero warnings across the entire 1907-line delta.

8. **`context: fork` on builder skill**: Adding `context: fork` to `skills/aigent-builder/SKILL.md` ensures the builder runs in an isolated context, preventing skill-building operations from polluting the main conversation state.

9. **PromptOutputFormat duplication**: `PromptOutputFormat` in `main.rs` mirrors `PromptFormat` in `prompt.rs` with a `From` impl between them. This is a reasonable pattern to keep CLI concerns separate from library concerns, but adds maintenance surface.

10. **`collect_skills()` silently skips errors**: Skills that fail to parse are silently dropped. This is documented behavior but could surprise users when a typo in frontmatter causes a skill to vanish from prompt output.

### Verdict

**Conditional merge** — The implementation is solid with excellent test coverage and clean tooling output. Two issues should be addressed before merge:

1. **Prior F1 (High)**: `build --template` must either wire through to `build_skill()` or the `--template` flag should be removed from the `build` subcommand to avoid advertising a non-functional feature.
2. **Prior F3 (Medium)**: `init_skill()` should set the execute bit on generated shell scripts (`scripts/run.sh`), or the template should include a post-init note telling users to `chmod +x`.

### Pre-merge Checklist

- [ ] Resolve Prior F1: Wire `SkillSpec.template` into `build_skill()` output, or remove `--template` from `build` subcommand
- [ ] Resolve Prior F3: Set execute bit on generated scripts in `init_skill()`, or document the manual step
- [ ] Consider F4: Document `init_skill()` API break in changelog or release notes
- [ ] Consider F5: Document that interactive mode is always deterministic, or allow independent `--no-llm` control
- [ ] Consider F9: Add `jq` dependency note to README or hook documentation
