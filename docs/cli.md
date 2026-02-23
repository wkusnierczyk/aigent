# CLI reference

> Back to [README](../README.md)

- [Commands](#commands)
- [Exit codes](#exit-codes)
- [Command flags](#command-flags)
  - [`build` (assembly) flags](#build-assembly-flags)
  - [`check` flags](#check-flags)
  - [`format` flags](#format-flags)
  - [`new` flags](#new-flags)
  - [`probe` flags](#probe-flags)
  - [`test` flags](#test-flags)
  - [`upgrade` flags](#upgrade-flags)
  - [`validate` flags](#validate-flags)
  - [`validate-plugin` flags](#validate-plugin-flags)
- [Command examples](#command-examples)
  - [`build` — Assemble skills into a plugin](#build--assemble-skills-into-a-plugin)
  - [`check` — Validate + semantic quality checks](#check--validate--semantic-quality-checks)
  - [`doc` — Generate a skill catalog](#doc--generate-a-skill-catalog)
  - [`format` — Format `SKILL.md` files](#format--format-skillmd-files)
  - [`init` — Create a template `SKILL.md`](#init--create-a-template-skillmd)
  - [`new` — Create a skill from natural language](#new--create-a-skill-from-natural-language)
  - [`probe` — Simulate skill activation](#probe--simulate-skill-activation)
  - [`prompt` — Generate XML prompt block](#prompt--generate-xml-prompt-block)
  - [`properties` — Output skill metadata as JSON](#properties--output-skill-metadata-as-json)
  - [`score` — Rate a skill 0–100](#score--rate-a-skill-0100)
  - [`test` — Run fixture-based test suites](#test--run-fixture-based-test-suites)
  - [`upgrade` — Detect and apply best-practice improvements](#upgrade--detect-and-apply-best-practice-improvements)
  - [`validate` — Check skill directories for specification conformance](#validate--check-skill-directories-for-specification-conformance)
  - [`validate-plugin` — Validate a Claude Code plugin directory](#validate-plugin--validate-a-claude-code-plugin-directory)
- [Watch mode](#watch-mode)
- [Global flags](#global-flags)

Run `aigent --help` for a list of commands.
Run `aigent <command> --help` for details on a specific command (flags, arguments, examples).

Full API documentation is available at [docs.rs/aigent](https://docs.rs/aigent).

## Commands

**Quality spectrum:** `validate` (specification conformance) → `check` (+ semantic quality) → `score` (quantitative 0–100).

<table>
<tr><th width="280">Command</th><th>Description</th></tr>
<tr><td><code>build [dirs...]</code></td><td>Assemble skills into a Claude Code plugin</td></tr>
<tr><td><code>check [dirs...]</code></td><td>Run validate + semantic lint checks (superset of <code>validate</code>)</td></tr>
<tr><td><code>doc [dirs...]</code></td><td>Generate a markdown skill catalog</td></tr>
<tr><td><code>format [dirs...]</code></td><td>Format <code>SKILL.md</code> files (canonical key order, clean whitespace)</td></tr>
<tr><td><code>init [directory]</code></td><td>Create a template <code>SKILL.md</code></td></tr>
<tr><td><code>new &lt;purpose&gt;</code></td><td>Create a skill from natural language</td></tr>
<tr><td><code>probe [dirs...] --query &lt;query&gt;</code></td><td>Probe skill activation against a sample user query</td></tr>
<tr><td><code>prompt [dirs...]</code></td><td>Generate <code>&lt;available_skills&gt;</code> XML block</td></tr>
<tr><td><code>properties [directory]</code></td><td>Output skill properties as JSON</td></tr>
<tr><td><code>score [directory]</code></td><td>Score a skill against best-practices checklist (0–100)</td></tr>
<tr><td><code>test [dirs...]</code></td><td>Run fixture-based test suites from <code>tests.yml</code></td></tr>
<tr><td><code>upgrade [directory]</code></td><td>Check a skill for upgrade opportunities</td></tr>
<tr><td><code>validate [dirs...]</code></td><td>Validate skill directories against the specification</td></tr>
<tr><td><code>validate-plugin [plugin-dir]</code></td><td>Validate a Claude Code plugin directory (manifest, hooks, agents, commands, skills, cross-component)</td></tr>
</table>

> **Note**
> When no path is given, the current directory is used. This lets you run
> `aigent validate`, `aigent format --check`, etc. without specifying a path
> when the current directory contains a `SKILL.md` file. The tool does not
> search parent directories.

> **Note**
> Backward compatibility: The following old command names are available as hidden
> aliases and continue to work.
>
> | Old name | Current name |
> |----------|--------------|
> | `create` | `new` |
> | `fmt` | `format` |
> | `lint` | `check` |
> | `read-properties` | `properties` |
> | `to-prompt` | `prompt` |

## Exit codes

All commands exit 0 on success and 1 on failure. The table below clarifies
what "success" means for each command.

| Command | Exit 0 | Exit 1 |
|---------|--------|--------|
| `build` | Plugin assembled successfully | Assembly error |
| `check` | No errors | Errors found (warnings do not affect exit code) |
| `doc` | Catalog generated | I/O error |
| `format` | All files already formatted | Files were reformatted (with `--check`) or error |
| `init` | Template created | Directory already exists or I/O error |
| `new` | Skill created | Build error |
| `probe` | At least one result printed | All directories failed to parse |
| `prompt` | Prompt generated | No valid skills found |
| `properties` | Properties printed | Parse error |
| `score` | Perfect score (100/100) | Score below 100 |
| `test` | All test cases pass | Any test case fails |
| `upgrade` | No suggestions, or all fixes applied | Unapplied fix suggestions remain, or error |
| `validate` | No errors | Errors found (warnings do not affect exit code) |
| `validate-plugin` | No errors | Errors found in manifest, hooks, agents, commands, skills, or cross-component checks |

## Command flags

### `build` (assembly) flags

Assemble skills into a Claude Code plugin.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--name &lt;name&gt;</code></td><td>Override the plugin name (default: first skill name)</td></tr>
<tr><td><code>--output &lt;dir&gt;</code></td><td>Output directory for the assembled plugin (default: <code>./dist</code>)</td></tr>
<tr><td><code>--validate</code></td><td>Run validation on assembled skills</td></tr>
</table>

### `check` flags

Run validate + semantic lint checks (superset of `validate`).

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--apply-fixes</code></td><td>Apply automatic fixes for fixable issues</td></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
<tr><td><code>--no-validate</code></td><td>Skip specification conformance checks (semantic quality only)</td></tr>
<tr><td><code>--recursive</code></td><td>Discover skills recursively</td></tr>
<tr><td><code>--structure</code></td><td>Run directory structure checks</td></tr>
<tr><td><code>--target &lt;target&gt;</code></td><td>Validation target profile (see <a href="#validate-flags"><code>validate</code> flags</a>)</td></tr>
</table>

### `format` flags

Format `SKILL.md` files (canonical key order, clean whitespace).

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--check</code></td><td>Check formatting without modifying files (exit 1 if unformatted)</td></tr>
<tr><td><code>--recursive</code></td><td>Discover skills recursively</td></tr>
</table>

### `new` flags

Create a skill from natural language.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--dir &lt;dir&gt;</code></td><td>Output directory</td></tr>
<tr><td><code>--interactive, -i</code></td><td>Step-by-step confirmation mode</td></tr>
<tr><td><code>--name &lt;name&gt;</code></td><td>Override the derived skill name</td></tr>
<tr><td><code>--no-llm</code></td><td>Force deterministic mode (no LLM)</td></tr>
</table>

### `probe` flags

Probe skill activation against a sample user query.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--query, -q &lt;query&gt;</code></td><td>Sample user query to test activation against (required)</td></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
</table>

### `test` flags

Run fixture-based test suites from `tests.yml`.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
<tr><td><code>--generate</code></td><td>Generate a template <code>tests.yml</code> for skills that lack one</td></tr>
<tr><td><code>--recursive</code></td><td>Discover skills recursively</td></tr>
</table>

### `upgrade` flags

Check a skill for upgrade opportunities. Suggestions are tagged `[fix]`
(auto-applied with `--apply`) or `[info]` (informational only).

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--apply</code></td><td>Apply automatic upgrades (<code>[fix]</code> suggestions only)</td></tr>
<tr><td><code>--dry-run</code></td><td>Preview suggestions without modifying files (default behavior; cannot be combined with <code>--apply</code>)</td></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
<tr><td><code>--full</code></td><td>Run validate + lint before upgrade (with <code>--apply</code>, also fix errors first)</td></tr>
</table>

**Scope and boundaries:**

- Appends missing optional fields to frontmatter; never removes or rewrites existing fields
- Never modifies the markdown body
- Never changes field values that already exist
- All suggestions are spec-compliant (no non-spec fields)

**Upgrade rules:**

| Code | Description | Kind | Modifies |
|------|-------------|------|----------|
| U001 | Missing `compatibility` field | fix | Appends `compatibility: claude-code` to frontmatter |
| U002 | Missing "Use when..." trigger phrase | info | Nothing (advisory) |
| U003 | Body exceeds 500 lines | info | Nothing (advisory) |

`--apply` only acts on `fix`-kind rules. Informational suggestions are always
advisory — they appear in output but are never auto-applied.

### `validate` flags

Validate skill directories against the specification.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--apply-fixes</code></td><td>Apply automatic fixes for fixable issues</td></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
<tr><td><code>--recursive</code></td><td>Discover skills recursively</td></tr>
<tr><td><code>--structure</code></td><td>Run directory structure checks</td></tr>
<tr><td><code>--target &lt;target&gt;</code></td><td>Validation target profile (see below)</td></tr>
<tr><td><code>--watch</code></td><td>Watch for changes and re-validate (see <a href="#watch-mode">Watch mode</a>)</td></tr>
</table>

**Validation targets** control which frontmatter fields are considered known:

| Target | Description |
|--------|-------------|
| `standard` (default) | Only Anthropic specification fields; warns on extras like `argument-hint` |
| `claude-code` | Standard fields plus Claude Code extension fields (e.g., `argument-hint`, `context`) |
| `permissive` | No unknown-field warnings; all fields accepted |

### `validate-plugin` flags

Validate a Claude Code plugin directory.

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--format &lt;format&gt;</code></td><td>Output format: <code>text</code> or <code>json</code></td></tr>
</table>

> **Note**
> Semantic lint checks are available with `check`.
> Use `aigent check` for combined validation + linting, or `aigent check --no-validate` for lint-only.

## Command examples

### `build` — Assemble skills into a plugin

Packages one or more skill directories into a Claude Code plugin directory
with a `plugin.json` manifest, `skills/` subdirectory, and scaffolded
`agents/` and `hooks/` directories.

```
$ aigent build skills/aigent-validator skills/aigent-scorer --output ./dist
Assembled 2 skill(s) into ./dist
```

The output structure:

```
dist/
├── plugin.json
├── skills/
│   ├── aigent-validator/
│   │   └── SKILL.md
│   └── aigent-scorer/
│       └── SKILL.md
├── agents/
└── hooks/
```

### `check` — Validate + semantic quality checks

Runs specification conformance (like `validate`) plus semantic quality checks:
third-person descriptions, trigger phrases, gerund name forms, generic
names, and description detail. Use `--no-validate` to skip specification checks
and run semantic lint only.

Diagnostics use three severity levels:
- **error** — specification violation (causes exit 1)
- **warning** — specification conformance issue (does not affect exit code)
- **info** — quality suggestion from semantic lint (does not affect exit code)

```
$ aigent check skills/aigent-validator
warning: unexpected metadata field: 'argument-hint'
info: name does not use gerund form
```

Semantic lint only:

```
$ aigent check --no-validate skills/aigent-validator
info: name does not use gerund form
```

### `doc` — Generate a skill catalog

Produces a markdown catalog of skills. Use `--recursive` to discover skills
in subdirectories, and `--output` to write to a file (diff-aware — only
writes if content changed).

```
$ aigent doc skills --recursive
# Skill Catalog

## aigent-builder
> Generates AI agent skill definitions (SKILL.md files) from natural
> language descriptions. ...
**Location**: `skills/aigent-builder/SKILL.md`

---

## aigent-scorer
> Scores AI agent skill definitions (SKILL.md files) against the Anthropic
> best-practices checklist. ...
**Location**: `skills/aigent-scorer/SKILL.md`

---

## aigent-validator
> Validates AI agent skill definitions (SKILL.md files) against the
> Anthropic agent skill specification. ...
**Location**: `skills/aigent-validator/SKILL.md`

---
```

```
$ aigent doc skills --recursive --output catalog.md
(writes catalog.md; re-running skips write if content unchanged)
```

### `format` — Format `SKILL.md` files

Normalizes `SKILL.md` files with canonical YAML key ordering, consistent
whitespace, and clean formatting. The operation is idempotent — running
it twice produces no further changes.

```
$ aigent format my-skill/
Formatted my-skill/
```

Check mode reports which files would change without modifying them,
and shows a unified diff of the changes:

```
$ aigent format --check my-skill/
Would reformat: my-skill/
--- my-skill/
+++ my-skill/ (formatted)
@@ -1,6 +1,6 @@
 ---
-allowed-tools: Bash, Read, Write
 name: my-skill
 description: ...
+allowed-tools: Bash, Read, Write
 ---
```

### `init` — Create a template `SKILL.md`

Scaffolds a skill directory with a template `SKILL.md` ready for editing.

```
$ aigent init my-skill
Created my-skill/SKILL.md
```

```
$ cat my-skill/SKILL.md
---
name: my-skill
description: Describe what this skill does and when to use it
---

# My Skill

## Quick start
[Add quick start instructions here]

## Usage
[Add detailed usage instructions here]
```

### `new` — Create a skill from natural language

Creates a complete skill directory with `SKILL.md` from a purpose description.
Uses LLM when an API key is available, or `--no-llm` for deterministic mode.

```
$ aigent new "Extract text from PDF files" --no-llm
Created skill 'extracting-text-pdf-files' at extracting-text-pdf-files
```

The generated `SKILL.md` includes derived name, description, and a template body:

```markdown
---
name: extracting-text-pdf-files
description: Extract text from PDF files. Use when working with files.
---
# Extracting Text Pdf Files

## Quick start
Extract text from PDF files

## Usage
Use this skill to Extract text from PDF files.
```

### `probe` — Simulate skill activation

Probes whether a skill's description would activate for a given user query.
This is a dry-run of skill discovery — "if a user said *this*, would Claude
pick up *that* skill?" Accepts multiple directories — results are ranked by
match score (best first).

Uses a **weighted formula** to compute a match score (0.0–1.0):
- **0.5 × description overlap** — fraction of query tokens in description
- **0.3 × trigger score** — match against trigger phrases ("Use when...")
- **0.2 × name score** — query-to-name token overlap

Categories based on weighted score:
- **Strong** (≥ 0.4) — skill would reliably activate
- **Weak** (≥ 0.15) — might activate, but description could be improved
- **None** (< 0.15) — skill would not activate for this query

Also reports estimated token cost and any validation issues.

Single directory:

```
$ aigent probe skills/aigent-validator --query "validate a skill"
Skill: aigent-validator
Query: "validate a skill"
Description: Validates AI agent skill definitions (SKILL.md files) against
the Anthropic agent skill specification. ...

Activation: STRONG ✓ — description aligns well with query (score: 0.65)
Token footprint: ~76 tokens

Validation warnings (1):
  warning: unexpected metadata field: 'argument-hint'
```

Multiple directories (results ranked by score, best first):

```
$ aigent probe skills/* --query "validate a skill"
Skill: aigent-validator
...
Activation: STRONG ✓ — description aligns well with query (score: 0.65)

Skill: aigent-scorer
...
Activation: WEAK ⚠ — some overlap, but description may not trigger reliably (score: 0.25)

Skill: aigent-builder
...
Activation: NONE ✗ — description does not match the test query (score: 0.00)
```

Default directory (from inside a skill directory):

```
$ cd skills/aigent-validator
$ aigent probe --query "validate a skill"
Skill: aigent-validator
...
Activation: STRONG ✓ — description aligns well with query (score: 0.65)
```

### `prompt` — Generate XML prompt block

Generates the `<available_skills>` XML block that gets injected into Claude's
system prompt. Accepts multiple skill directories.

```
$ aigent prompt skills/aigent-validator
<available_skills>
  <skill>
    <name>aigent-validator</name>
    <description>Validates AI agent skill definitions ...</description>
    <location>skills/aigent-validator/SKILL.md</location>
  </skill>
</available_skills>
```

### `properties` — Output skill metadata as JSON

Parses the `SKILL.md` frontmatter and outputs structured JSON. Useful for
scripting and integration with other tools.

```
$ aigent properties skills/aigent-validator
{
  "name": "aigent-validator",
  "description": "Validates AI agent skill definitions ...",
  "allowed-tools": "Bash(aigent validate *), Bash(command -v *), Read, Glob",
  "metadata": {
    "argument-hint": "[skill-directory-or-file]"
  }
}
```

### `score` — Rate a skill 0–100

Rates a skill from 0 to 100 against the Anthropic best-practices checklist.
The score has two weighted categories:

- **Structural (60 points)** — Checks that the `SKILL.md` parses correctly, the
  name matches the directory, required fields are present, no unknown fields
  exist, and the body is within size limits. All six checks must pass to earn
  the 60 points; any failure zeros the structural score.

- **Quality (40 points)** — Five semantic lint checks worth 8 points each:
  third-person description, trigger phrase (`"Use when..."`), gerund name form
  (`converting-pdfs` not `pdf-converter`), specific (non-generic) name, and
  description length (≥ 20 words).

The exit code is 0 for a perfect score and 1 otherwise, making it suitable for
CI gating.

**Example** — a skill that passes all checks:

```
$ aigent score converting-pdfs/
Score: 100/100

Structural (60/60):
  [PASS] SKILL.md exists and is parseable
  [PASS] Name format valid
  [PASS] Description valid
  [PASS] Required fields present
  [PASS] No unknown fields
  [PASS] Body within size limits

Quality (40/40):
  [PASS] Third-person description
  [PASS] Trigger phrase present
  [PASS] Gerund name form
  [PASS] Specific name
  [PASS] Detailed description
```

**Example** — a skill with issues. Each check shows a distinct label for its
pass/fail state (e.g., "No unknown fields" when passing, "Unknown fields
found" when failing):

```
$ aigent score aigent-validator/
Score: 32/100

Structural (0/60):
  [PASS] SKILL.md exists and is parseable
  [PASS] Name format valid
  [PASS] Description valid
  [PASS] Required fields present
  [FAIL] Unknown fields found
         unexpected metadata field: 'argument-hint'
  [PASS] Body within size limits

Quality (32/40):
  [PASS] Third-person description
  [PASS] Trigger phrase present
  [FAIL] Non-gerund name form
         name does not use gerund form
  [PASS] Specific name
  [PASS] Detailed description
```

### `test` — Run fixture-based test suites

Runs test suites defined in `tests.yml` files alongside skills. Each test
case specifies an input query, whether it should match, and an optional
minimum score threshold.

Generate a template `tests.yml`:

```
$ aigent test --generate my-skill/
Generated my-skill/tests.yml
```

```yaml
# Test fixture for my-skill
# Run with: aigent test my-skill/
queries:
- input: process pdf files and extract text
  should_match: true
  min_score: 0.3
- input: something completely unrelated to this skill
  should_match: false
```

Run the test suite:

```
$ aigent test my-skill/
[PASS] "process pdf files" (score: 0.65)
[PASS] "something completely unrelated to this skill" (score: 0.00)

2 passed, 0 failed, 2 total
```

### `upgrade` — Detect and apply best-practice improvements

Checks for recommended-but-optional fields and patterns. Suggestions are
tagged `[fix]` (auto-applied with `--apply`) or `[info]` (informational only).

```
$ aigent upgrade skills/aigent-validator
[fix] U001: Missing 'compatibility' field — recommended for multi-platform skills.
[info] U002: Description lacks 'Use when...' trigger phrase — helps Claude activate the skill.

Run with --apply to apply 1 fix(es). 1 informational suggestion(s) shown above.
```

```
$ aigent upgrade --apply skills/aigent-validator
(applies fix-kind suggestions in-place, prints confirmation to stderr)
```

Use `--dry-run` for explicit no-modify intent in scripts (equivalent to
omitting `--apply`):

```
$ aigent upgrade --dry-run skills/aigent-validator
```

Full mode runs validate + lint first, and with `--apply` fixes errors
before performing upgrades:

```
$ aigent upgrade --full --apply skills/aigent-validator
[full] Applied 1 validation/lint fix(es)
[fix] U001: Missing 'compatibility' field — recommended for multi-platform skills.
```

### `validate` — Check skill directories for specification conformance

Validates one or more skill directories against the Anthropic specification.
Exit code 0 means valid; non-zero means errors were found (warnings do not affect exit code). For
combined validation + semantic quality checks, use `check` instead.

```
$ aigent validate my-skill/
(no output — skill is valid)
```

With `--structure` for directory layout checks:

```
$ aigent validate skills/aigent-validator --structure
warning: unexpected metadata field: 'argument-hint'
```

Multiple directories trigger cross-skill conflict detection automatically:

```
$ aigent validate skills/aigent-validator skills/aigent-builder skills/aigent-scorer
skills/aigent-validator:
  warning: unexpected metadata field: 'argument-hint'
skills/aigent-builder:
  warning: unexpected metadata field: 'argument-hint'
  warning: unexpected metadata field: 'context'
skills/aigent-scorer:
  warning: unexpected metadata field: 'argument-hint'

3 skills: 0 ok, 0 errors, 3 warnings only
```

JSON output for CI integration:

```
$ aigent validate skills/aigent-validator --format json
[
  {
    "diagnostics": [
      {
        "code": "W001",
        "field": "metadata",
        "message": "unexpected metadata field: 'argument-hint'",
        "severity": "warning"
      }
    ],
    "path": "skills/aigent-validator"
  }
]
```

### `validate-plugin` — Validate a Claude Code plugin directory

Validates the full plugin ecosystem: `plugin.json` manifest, `hooks.json`,
agent files, command files, skill directories, and cross-component consistency
(naming, duplicates, token budget, orphaned files, hook script references).

```
$ aigent validate-plugin my-plugin/
plugin.json: ok
hooks.json: ok
agents/code-reviewer: ok
commands/deploy: ok
skills/pdf-processor: ok
Cross-component: ok
```

With errors:

```
$ aigent validate-plugin my-plugin/
plugin.json:
  error [P003]: `name` is not kebab-case: "My Plugin"
hooks.json:
  error [H003]: unknown event name: "OnSave"
Cross-component:
  warning [X006]: duplicate component name "helper" across agent and command
```

## Watch mode

The `--watch` flag on `validate` monitors skill directories for filesystem
changes and re-validates automatically on each edit — a live feedback loop
while developing skills.

Watch mode is behind a **Cargo feature gate** because it pulls in
platform-specific filesystem notification libraries (`notify`, `fsevent-sys`
on macOS, `inotify` on Linux). Without the feature, the binary is smaller
and has fewer dependencies.

**Building with watch mode:**

```bash
# Build with watch support
cargo build --release --features watch

# Run with watch support (--features must be passed every time)
cargo run --release --features watch -- validate --watch skills/

# Or install with watch support
cargo install aigent --features watch
```

> **Note**
> Cargo feature flags are per-invocation — they are not remembered
> between builds. You must pass `--features watch` on every `cargo build` or
> `cargo run` invocation. Building debug with `--features watch` does not
> enable it for release builds, and vice versa.

Without the `watch` feature, using `--watch` prints a helpful error:

```
$ aigent validate --watch my-skill/
Watch mode requires the 'watch' feature. Rebuild with: cargo build --features watch
```

## Global flags

<table>
<tr><th width="280">Flag</th><th>Description</th></tr>
<tr><td><code>--about</code></td><td>Show project information</td></tr>
<tr><td><code>--version</code></td><td>Print version</td></tr>
<tr><td><code>--help</code></td><td>Print help</td></tr>
</table>
