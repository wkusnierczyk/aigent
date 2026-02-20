# M7: Skill Builder — Work Plan

## Overview

Implement the skill builder: deterministic and LLM-enhanced generation of
SKILL.md files from natural language descriptions. Includes `derive_name`,
`assess_clarity`, `build_skill`, the `init` command, the `LlmProvider` trait
with Anthropic/OpenAI/Google/Ollama/OpenAI-compatible backends, and CLI wiring
for `build` and `init` subcommands.

Issues: #21, #22, #23, #24, #25.

## Branch Strategy

- **Dev branch**: `dev/m07` (created from `main`)
- **Task branches**: `task/m07-<name>` (created from `dev/m07`)
- After each wave, task branches merge into `dev/m07`
- After all waves, PR from `dev/m07` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- `SkillProperties` — from M2 (`src/models.rs`)
- `AigentError::Build` — from M2 (`src/errors.rs`)
- `validate()` — from M4 (`src/validator.rs`)
- `find_skill_md()`, `read_properties()` — from M3 (`src/parser.rs`)
- `reqwest` — new dependency (HTTP client for LLM APIs)
- `tokio` — new dependency (async runtime for `reqwest`)
- `serde_json` — already in `Cargo.toml`
- `mockall` — new dev-dependency (mock LLM provider in tests)

## Current State

`src/builder.rs` has data model structs (`SkillSpec`, `BuildResult`,
`ClarityAssessment`) already defined from M1 scaffolding. Three functions are
stubs (`todo!()`): `build_skill`, `derive_name`, `assess_clarity`.

`src/main.rs` has `Build` and `Init` subcommands defined in the `Commands` enum
with all CLI arguments already wired up (`purpose`, `--name`, `--dir`,
`--no-llm` for Build; `dir` for Init). Both handlers print "not yet
implemented" and exit 1.

`src/lib.rs` already re-exports `build_skill`, `derive_name`, `assess_clarity`,
`BuildResult`, `ClarityAssessment`, `SkillSpec`.

---

## Review Finding Resolutions

### Finding 1 (High): `SkillSpec` gains `no_llm` field — breaks existing struct

**Resolution**: Add `no_llm: bool` to `SkillSpec` in Wave 1 (not Wave 2).
Add `#[derive(Default)]` to `SkillSpec` so `no_llm` defaults to `false`.
This avoids breaking downstream consumers and eliminates the Wave 1→2
sequencing problem. In Wave 1, `build_skill` ignores `no_llm` (always
deterministic). Wave 2 connects it to `detect_provider()`.

### Finding 2 (High): `build_skill` signature vs double name derivation

**Resolution**: Move output directory logic inside `build_skill`. Change
the signature: remove `output_dir: &Path` parameter, add
`output_dir: Option<PathBuf>` to `SkillSpec`. `build_skill` determines the
output directory internally:
- If `spec.output_dir` is `Some`, use it
- If `None`, derive from the skill name (which is derived once, internally)

This eliminates double name derivation and the CLI no longer needs to call
`derive_name` separately. The CLI passes `--dir` via `spec.output_dir`.

Updated signature: `build_skill(spec: &SkillSpec) -> Result<BuildResult>`

### Finding 3 (Medium): Ollama probe latency

**Resolution**: Require `OLLAMA_HOST` env var for Ollama detection (opt-in).
Don't auto-probe localhost. If `OLLAMA_HOST` is set, use it as the base URL
and skip the connectivity probe (fail on first LLM call instead). This
eliminates the 1-second latency for users without Ollama. Users who want
Ollama just set `OLLAMA_HOST=http://localhost:11434`.

### Finding 4 (Medium): `init_skill` error message

**Resolution**: Include the found filename in the error message. If
`find_skill_md(dir)` returns `Some(path)`, the error message is
`"already exists: {path}"` (showing the actual file found, whether
`SKILL.md` or `skill.md`).

### Finding 5 (Medium): Test count

**Resolution**: No change — arithmetic confirmed correct (38 + 6 + 7 = 51).

### Finding 6 (Low): Direct `SkillProperties` construction

**Resolution**: Construct `SkillProperties` directly instead of going
through a HashMap intermediary. Serialize it to YAML for the frontmatter.
This makes `SkillProperties` the single source of truth.

### Finding 7 (Low): `reqwest` dependency weight

**Resolution**: Use `ureq` (v3) instead of `reqwest`. `ureq` is a
sync-native HTTP client with a much smaller dependency footprint — no tokio
runtime, no hyper, no tower. This aligns with the sync API decision and
reduces compile times. The `ureq` API is straightforward:
`ureq::post(url).header(...).send_json(body)`.

Updated Cargo.toml:
```toml
ureq = { version = "3", features = ["json"] }
```

No `mockall` needed — we'll use a simple manual mock struct implementing
`LlmProvider` for tests, which avoids another macro-heavy dependency.

### Finding 8 (Low): Model env var overrides

**Resolution**: Each provider reads a model override from an env var:
- `ANTHROPIC_MODEL` (default: `"claude-sonnet-4-20250514"`)
- `OPENAI_MODEL` (default: `"gpt-4o"`)
- `GOOGLE_MODEL` (default: `"gemini-2.0-flash"`)
- `OLLAMA_MODEL` (default: `"llama3.2"`)

### Issue Deviations (from review)

- **Issue #24 specifies `async_trait`**: Plan deliberately uses sync
  (`ureq`). Rationale: sequential CLI, no server, smaller deps.
- **Issue #24 mentions `--provider` flag and `AIGENT_LLM_PROVIDER`**:
  Deferred to a future enhancement. Auto-detection covers the common cases.

---

## Design Decisions

### Module Organization

The builder grows beyond a single file. New module structure:

```
src/
├── builder/
│   ├── mod.rs          # Re-exports, build_skill, init_skill
│   ├── deterministic.rs # derive_name, generate_frontmatter, generate_body, assess_clarity
│   ├── llm.rs          # LlmProvider trait + provider selection
│   ├── providers/
│   │   ├── mod.rs      # Re-exports
│   │   ├── anthropic.rs
│   │   ├── openai.rs
│   │   ├── google.rs
│   │   └── ollama.rs
│   └── template.rs     # SKILL.md template for `init`
```

The existing `src/builder.rs` becomes `src/builder/mod.rs`. This is a
backward-compatible refactor — `pub mod builder;` in `lib.rs` still works, and
all re-exports remain unchanged.

### Sync vs Async API

The public library API remains **synchronous**. `build_skill()` is a blocking
function. Internally, LLM calls use `ureq` — a sync-native HTTP client with
no async runtime overhead. This avoids pulling in `tokio` or `hyper`:
- LLM calls are sequential (name → description → body), not concurrent
- The CLI is a single-threaded binary, not a server
- Library consumers can wrap in their own async runtime if needed

This means: **no `tokio` dependency, no `reqwest`**. Only `ureq` with `json`
feature. (Issue #24 specifies `async_trait` — this is a deliberate deviation.)

### LLM Provider Trait

```rust
pub trait LlmProvider: Send + Sync {
    fn generate(&self, system: &str, user: &str) -> Result<String>;
}
```

Synchronous, not async. Each provider implements this with `ureq`. Provider
structs hold the API key (and optionally a base URL and model override).

### Provider Selection

Auto-detection order (matching issue #24):
1. `ANTHROPIC_API_KEY` set → `AnthropicProvider`
2. `OPENAI_API_KEY` set → `OpenAiProvider`
3. `GOOGLE_API_KEY` set → `GoogleProvider`
4. `OLLAMA_HOST` set → `OllamaProvider` (opt-in, no auto-probe)
5. None → deterministic mode (no provider)

Override mechanisms:
- `--no-llm` flag → force deterministic mode regardless of env vars
- `--provider` / `AIGENT_LLM_PROVIDER` → deferred to future enhancement

The detection function:

```rust
fn detect_provider() -> Option<Box<dyn LlmProvider>>
```

Returns `None` for deterministic mode.

### Deterministic Mode

When no LLM is available (or `--no-llm`), the builder uses heuristic functions:

- `derive_name`: lowercase, strip filler words, kebab-case, prefer gerund form
  via simple suffix rules ("process" → "processing", "analyze" → "analyzing")
- `generate_description`: template-based third-person description derived from
  the purpose string
- `generate_body`: template with sections (Quick start, Usage, Notes) using the
  purpose and name
- `assess_clarity`: heuristic word-count and question-mark checks

These produce valid but formulaic output — the goal is a working baseline that
always succeeds with zero configuration.

### LLM-Enhanced Mode

When a provider is available, the builder uses it to enhance:

- `derive_name`: LLM produces a more natural gerund-form kebab-case name
- `generate_description`: LLM writes a richer third-person what+when description
- `generate_body`: LLM generates detailed, context-specific markdown instructions
- `assess_clarity`: LLM provides better ambiguity detection and clarifying questions

Each LLM call has a focused system prompt and the user's purpose string.
If an LLM call fails, the builder falls back to the deterministic version for
that specific function and prints a warning to stderr. This is per-function
fallback, not all-or-nothing.

### `build_skill` Pipeline

Updated signature: `build_skill(spec: &SkillSpec) -> Result<BuildResult>`.
No `output_dir` parameter — the output directory is determined internally
from `spec.output_dir` (if provided) or derived from the skill name.

```
1. Select provider (detect, or skip if spec.no_llm)
2. Derive name (from purpose, or use spec.name override)
3. Determine output directory (spec.output_dir or ./<name>/)
4. Generate description (from purpose + name)
5. Construct SkillProperties directly (name, description, optional fields)
6. Generate body (from purpose + name + description)
7. Serialize SkillProperties to YAML frontmatter
8. Assemble SKILL.md content (frontmatter + body)
9. Create output directory if it doesn't exist
10. Check for existing SKILL.md — fail if present
11. Write SKILL.md + any extra_files
12. Validate output with validate()
13. Return BuildResult
```

Step 12 is critical: the builder validates its own output. If validation
returns errors (not just warnings), the build fails with `AigentError::Build`.
Warnings are printed to stderr but do not fail the build.

### `init` Command

`aigent init [dir]` creates a template SKILL.md in the target directory:
- `dir` defaults to `.` (current directory)
- If SKILL.md already exists, exit with error (no overwrite)
- The template contains placeholder frontmatter and a body skeleton:

```yaml
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

The name placeholder is derived from the directory name (kebab-cased). The
`init` function lives in `builder/mod.rs` as `pub fn init_skill(dir: &Path)
-> Result<PathBuf>` and returns the path to the created SKILL.md.

### Output Directory Logic

For `build_skill`:
- If `--dir` is specified, use it as the output directory
- If `--dir` is not specified, create `./<derived-name>/` in the current directory
- If the output directory already exists and contains a SKILL.md, fail with
  `AigentError::Build` ("directory already contains SKILL.md")
- If the output directory doesn't exist, create it

### Cargo.toml Changes

```toml
[dependencies]
ureq = { version = "3", features = ["json"] }
```

No `tokio`, no `reqwest`, no `mockall`. `ureq` is sync-native with minimal
deps. Tests use a simple manual mock struct implementing `LlmProvider`.

### Error Handling

All builder errors use `AigentError::Build { message }`. Specific cases:
- LLM API errors → fall back to deterministic, warn to stderr
- Directory already has SKILL.md → `Build` error
- Validation of generated output fails → `Build` error with validation details
- IO errors during file writes → propagate as `AigentError::Io` via `?`

---

## Wave 1 — Data Model & Deterministic Core

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m07-deterministic` | #21, #22 | Refactor builder into module, implement deterministic functions |

**Merge**: A → `dev/m07`. Checkpoint with user.

### Agent A — Deterministic Builder (#21, #22)

#### Pre-requisite: Module refactor

1. Move `src/builder.rs` → `src/builder/mod.rs`
2. Create `src/builder/deterministic.rs`
3. Create `src/builder/template.rs`
4. Verify `src/lib.rs` re-exports still work unchanged

#### `src/builder/deterministic.rs`

##### `derive_name(purpose: &str) -> String`

Deterministic name derivation from natural language:

1. Lowercase the input
2. Remove filler words: "a", "an", "the", "to", "for", "from", "with", "and",
   "or", "that", "which", "this", "my"
3. Apply gerund form to the first word if it looks like a verb:
   - Ends in "e" (not "ee") → drop "e", add "ing" ("analyze" → "analyzing")
   - Ends in consonant + short vowel + consonant (CVC) pattern for common
     short verbs → double final consonant + "ing" ("run" → "running")
   - Otherwise → add "ing" ("process" → "processing")
   - Already ends in "ing" → keep as-is
4. Join remaining words with hyphens
5. Remove any characters not in `[a-z0-9-]`
6. Collapse consecutive hyphens
7. Trim leading/trailing hyphens
8. Truncate to 64 characters (at a hyphen boundary if possible)
9. If result is empty, return `"my-skill"`

##### `generate_description(purpose: &str, name: &str) -> String`

Template-based description:

```
"{Capitalized purpose}. Use when {trigger context derived from purpose}."
```

The trigger context is a simple heuristic: if the purpose contains an object
noun (last significant word), use "working with {object}" as the trigger.
Fallback: "this capability is needed".

Truncate to 1024 characters if needed.

##### `generate_body(purpose: &str, name: &str, description: &str) -> String`

Template-based markdown body:

```markdown
# {Title-cased name}

## Quick start

{Purpose-derived instruction}

## Usage

{Purpose-derived usage section}

## Notes

- Generated by aigent {version}
- Edit this file to customize the skill
```

##### `assess_clarity(purpose: &str) -> ClarityAssessment`

Deterministic clarity heuristics:

1. If purpose is fewer than 3 words → unclear, question: "Can you provide more
   detail about what the skill should do?"
2. If purpose contains a question mark → unclear, question: "Please provide a
   statement describing the skill, not a question."
3. If purpose is longer than 10 words → clear
4. If purpose contains at least one noun and one verb (heuristic: check for
   common verb endings) → clear
5. Otherwise → unclear, question: "Can you describe the specific task or
   workflow this skill should handle?"

Returns `ClarityAssessment { clear, questions }`.

#### `src/builder/template.rs`

##### `skill_template(dir_name: &str) -> String`

Returns the SKILL.md template string with the directory name kebab-cased as
the `name` field and title-cased as the heading. Used by `init_skill`.

#### `src/builder/mod.rs`

Re-export data types (`SkillSpec`, `BuildResult`, `ClarityAssessment`) — move
struct definitions here from the old `builder.rs`.

##### `init_skill(dir: &Path) -> Result<PathBuf>`

1. If `find_skill_md(dir)` returns `Some`, fail: "SKILL.md already exists"
2. Derive `dir_name` from `dir.file_name()` (fallback: "my-skill")
3. Generate template content via `skill_template(dir_name)`
4. Create `dir` if it doesn't exist (`fs::create_dir_all`)
5. Write `dir/SKILL.md`
6. Return the path to the created file

##### `build_skill(spec: &SkillSpec) -> Result<BuildResult>`

Deterministic-only implementation in Wave 1 (LLM added in Wave 2):

1. Derive name: `spec.name.clone().unwrap_or_else(|| derive_name(&spec.purpose))`
2. Determine output dir: `spec.output_dir.clone().unwrap_or_else(|| PathBuf::from(&name))`
3. Generate description: `generate_description(&spec.purpose, &name)`
4. Construct `SkillProperties` directly:
   ```rust
   SkillProperties { name, description, license: spec.license.clone(),
     compatibility: spec.compatibility.clone(),
     allowed_tools: spec.tools.clone(), metadata: None }
   ```
5. Generate body: `generate_body(&spec.purpose, &props.name, &props.description)`
6. Serialize `SkillProperties` to YAML, assemble SKILL.md: `---\n{yaml}\n---\n{body}`
7. Check output directory: if exists and contains SKILL.md → `AigentError::Build`
8. Create output directory if it doesn't exist: `fs::create_dir_all`
9. Write `SKILL.md` to output directory
10. Write `extra_files` if present (relative paths within output dir)
11. Validate: call `validate(&output_dir)` — if errors (non-warnings), fail
    with `AigentError::Build` containing the validation messages
12. Build `files` HashMap: `"SKILL.md"` → content, plus extra files
13. Return `BuildResult { properties, files, output_dir }`

---

## Wave 2 — LLM Providers (depends on Wave 1)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| B | `task/m07-llm` | #23, #24 | Implement LlmProvider trait and provider backends |

**Merge**: B → `dev/m07`. Checkpoint with user.

### Agent B — LLM Providers (#23, #24)

#### Cargo.toml

Add `ureq` with json feature:

```toml
ureq = { version = "3", features = ["json"] }
```

#### `src/builder/llm.rs`

##### `LlmProvider` trait

```rust
/// Trait for LLM text generation providers.
pub trait LlmProvider: Send + Sync {
    /// Generate a text response given a system prompt and user message.
    fn generate(&self, system: &str, user: &str) -> Result<String>;
}
```

##### `detect_provider() -> Option<Box<dyn LlmProvider>>`

Check environment variables in priority order (no network probes):

1. `ANTHROPIC_API_KEY` → `AnthropicProvider` (model: `ANTHROPIC_MODEL` or default)
2. `OPENAI_API_KEY` → `OpenAiProvider` (model: `OPENAI_MODEL` or default)
3. `GOOGLE_API_KEY` → `GoogleProvider` (model: `GOOGLE_MODEL` or default)
4. `OLLAMA_HOST` → `OllamaProvider` (model: `OLLAMA_MODEL` or default)
5. Return `None`

For OpenAI-compatible endpoints: if `OPENAI_API_BASE` (or `OPENAI_BASE_URL`)
is set alongside `OPENAI_API_KEY`, use that base URL instead of
`api.openai.com`. This covers vLLM, LM Studio, etc.

#### `src/builder/providers/anthropic.rs`

```rust
pub struct AnthropicProvider {
    api_key: String,
    model: String,  // ANTHROPIC_MODEL or "claude-sonnet-4-20250514"
}
```

`generate` implementation:
- POST to `https://api.anthropic.com/v1/messages` via `ureq`
- Headers: `x-api-key`, `anthropic-version: 2023-06-01`, `content-type: application/json`
- Body: `{ model, max_tokens: 1024, system, messages: [{ role: "user", content: user }] }`
- Extract `content[0].text` from response
- Map HTTP/parse errors to `AigentError::Build`

#### `src/builder/providers/openai.rs`

```rust
pub struct OpenAiProvider {
    api_key: String,
    base_url: String,  // OPENAI_API_BASE or "https://api.openai.com/v1"
    model: String,     // OPENAI_MODEL or "gpt-4o"
}
```

`generate` implementation:
- POST to `{base_url}/chat/completions`
- Headers: `Authorization: Bearer {api_key}`, `Content-Type: application/json`
- Body: `{ model, messages: [{ role: "system", content: system }, { role: "user", content: user }] }`
- Extract `choices[0].message.content` from response

#### `src/builder/providers/google.rs`

```rust
pub struct GoogleProvider {
    api_key: String,
    model: String,  // GOOGLE_MODEL or "gemini-2.0-flash"
}
```

`generate` implementation:
- POST to `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}`
- Body: `{ system_instruction: { parts: [{ text: system }] }, contents: [{ parts: [{ text: user }] }] }`
- Extract `candidates[0].content.parts[0].text` from response

#### `src/builder/providers/ollama.rs`

```rust
pub struct OllamaProvider {
    base_url: String,  // OLLAMA_HOST or "http://localhost:11434"
    model: String,     // OLLAMA_MODEL or "llama3.2"
}
```

`generate` implementation:
- POST to `{base_url}/api/generate`
- Body: `{ model, system, prompt: user, stream: false }`
- Extract `response` field from JSON response

#### LLM-Enhanced Builder Functions

In `src/builder/mod.rs`, add internal functions that use a provider:

##### `llm_derive_name(provider: &dyn LlmProvider, purpose: &str) -> Result<String>`

System prompt: "You are a naming assistant. Given a purpose description, derive
a kebab-case skill name using gerund form (e.g., 'processing-pdfs',
'analyzing-data'). Reply with ONLY the name, no explanation. The name must be
lowercase, use only letters, numbers, and hyphens, and be at most 64 characters."

Validate result: strip whitespace, check format. If invalid, fall back to
deterministic `derive_name`.

##### `llm_generate_description(provider: &dyn LlmProvider, purpose: &str, name: &str) -> Result<String>`

System prompt: "You are a technical writer. Write a concise skill description
in third person. Describe what the skill does and when to use it. Maximum 200
characters. No quotes or formatting."

##### `llm_generate_body(provider: &dyn LlmProvider, purpose: &str, name: &str, description: &str) -> Result<String>`

System prompt: "You are a skill author following the Anthropic agent skill
specification. Generate a markdown body for a SKILL.md file. Be concise — only
add context the model doesn't already have. Use sections with ## headings.
Keep under 100 lines. Do not include frontmatter delimiters (---)."

##### `llm_assess_clarity(provider: &dyn LlmProvider, purpose: &str) -> Result<ClarityAssessment>`

System prompt: "Evaluate if this purpose description is clear enough to
generate an AI agent skill. Reply in JSON: {\"clear\": true/false,
\"questions\": [\"question1\", ...]}. If clear, questions should be empty."

Parse JSON response. Fall back to deterministic if parse fails.

#### Integrate LLM into `build_skill`

Update `build_skill` to accept an optional provider:

The public signature is `build_skill(spec: &SkillSpec) -> Result<BuildResult>`.
Internally, it calls `detect_provider()` unless `spec.no_llm` is true.

**Decision**: Add `no_llm` and `output_dir` fields to `SkillSpec`, add
`Default` derive:

```rust
#[derive(Debug, Clone, Default)]
pub struct SkillSpec {
    pub purpose: String,
    pub name: Option<String>,
    pub tools: Option<String>,
    pub compatibility: Option<String>,
    pub license: Option<String>,
    pub extra_files: Option<HashMap<String, String>>,
    pub output_dir: Option<PathBuf>,  // NEW — output directory override
    pub no_llm: bool,                 // NEW — force deterministic mode
}
```

`build_skill` checks `spec.no_llm`: if false, calls `detect_provider()`;
if true, uses deterministic mode. The `Default` derive ensures `no_llm`
defaults to `false` and `output_dir` to `None` (backward-compatible).

For each generation step (name, description, body), if a provider is available:
1. Try LLM-enhanced version
2. On failure, warn to stderr, fall back to deterministic version

---

## Wave 3 — CLI Wiring (depends on Waves 1 + 2)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| C | `task/m07-cli` | #22 | Wire Build and Init handlers in `src/main.rs` |

**Merge**: C → `dev/m07`. Checkpoint with user.

### Agent C — CLI Wiring (#22)

#### `Build` handler

Replace the stub with:

```rust
Some(Commands::Build { purpose, name, dir, no_llm }) => {
    let spec = aigent::SkillSpec {
        purpose,
        name,
        output_dir: dir,
        no_llm,
        ..Default::default()
    };
    match aigent::build_skill(&spec) {
        Ok(result) => {
            println!("Created skill '{}' at {}", result.properties.name,
                     result.output_dir.display());
        }
        Err(e) => {
            eprintln!("aigent build: {e}");
            std::process::exit(1);
        }
    }
}
```

#### `Init` handler

Replace the stub with:

```rust
Some(Commands::Init { dir }) => {
    let target = dir.unwrap_or_else(|| PathBuf::from("."));
    match aigent::init_skill(&target) {
        Ok(path) => {
            println!("Created {}", path.display());
        }
        Err(e) => {
            eprintln!("aigent init: {e}");
            std::process::exit(1);
        }
    }
}
```

#### `lib.rs` update

Add `init_skill` to the re-exports:

```rust
pub use builder::{
    assess_clarity, build_skill, derive_name, init_skill,
    BuildResult, ClarityAssessment, SkillSpec,
};
```

---

## Wave 4 — Tests (depends on Waves 1 + 2 + 3)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| D | `task/m07-tests` | #25 | Write builder tests in `src/builder/` and `tests/cli.rs` |

**Merge**: D → `dev/m07`. Checkpoint with user.

### Agent D — Tests (#25)

Tests split between unit tests (in `src/builder/deterministic.rs` and
`src/builder/mod.rs`) and integration tests (appended to `tests/cli.rs`).

#### `derive_name` unit tests

`src/builder/deterministic.rs` — `#[cfg(test)] mod tests`

| # | Test | Type |
|---|------|------|
| 1 | "Process PDF files" → starts with "processing" | happy path |
| 2 | "Analyze spreadsheet data" → starts with "analyzing" | gerund |
| 3 | "Run database migrations" → starts with "running" | CVC doubling |
| 4 | Already gerund "processing files" → keeps "processing" | no-op |
| 5 | Single word "deploy" → "deploying" | minimal |
| 6 | Filler words removed: "a tool for the processing of data" | filtering |
| 7 | Special characters stripped: "Process PDFs!" → valid kebab-case | sanitize |
| 8 | Empty input → "my-skill" | edge case |
| 9 | Very long input → truncated to ≤ 64 chars | boundary |
| 10 | Result passes name validation (no uppercase, no consecutive hyphens) | invariant |

#### `generate_description` unit tests

| # | Test | Type |
|---|------|------|
| 11 | Returns non-empty string | basic |
| 12 | Contains purpose-related words | content |
| 13 | Does not exceed 1024 characters | boundary |
| 14 | Written in third person (no "I" or "you" at start) | style |

#### `generate_body` unit tests

| # | Test | Type |
|---|------|------|
| 15 | Returns non-empty markdown | basic |
| 16 | Contains heading with skill name | content |
| 17 | Contains "Quick start" section | structure |
| 18 | Contains aigent version reference | content |

#### `assess_clarity` unit tests

| # | Test | Type |
|---|------|------|
| 19 | Short input (< 3 words) → not clear | heuristic |
| 20 | Question input → not clear | heuristic |
| 21 | Detailed purpose (> 10 words) → clear | heuristic |
| 22 | Clear result has empty questions | invariant |
| 23 | Unclear result has non-empty questions | invariant |

#### `init_skill` unit tests

`src/builder/mod.rs` — `#[cfg(test)] mod tests`

| # | Test | Type |
|---|------|------|
| 24 | Creates SKILL.md in empty directory | happy path |
| 25 | Returns path to created file | happy path |
| 26 | Created file has valid frontmatter (parseable) | validation |
| 27 | Name derived from directory name | content |
| 28 | Fails if SKILL.md already exists | error |
| 29 | Creates directory if it doesn't exist | create-dir |

#### `build_skill` unit tests

| # | Test | Type |
|---|------|------|
| 30 | Deterministic build creates valid SKILL.md | happy path |
| 31 | Output passes `validate()` with no errors | validation |
| 32 | Uses `--name` override when provided | override |
| 33 | Derives name from purpose when no override | derivation |
| 34 | Fails if directory already has SKILL.md | error |
| 35 | Creates output directory if missing | create-dir |
| 36 | `BuildResult.files` contains "SKILL.md" key | result |
| 37 | Extra files written to output directory | extra-files |
| 38 | Spec with all optional fields populates frontmatter | full-spec |

#### LLM provider tests (mocked)

`src/builder/llm.rs` — `#[cfg(test)] mod tests`

| # | Test | Type |
|---|------|------|
| 39 | Mock provider returns expected text | unit |
| 40 | `detect_provider` returns `None` when no env vars | detection |
| 41 | LLM name derivation falls back on invalid response | fallback |
| 42 | LLM description generation falls back on error | fallback |
| 43 | LLM body generation falls back on error | fallback |
| 44 | LLM clarity assessment falls back on parse error | fallback |

#### CLI integration tests

Appended to `tests/cli.rs`:

| # | Test | Type |
|---|------|------|
| 45 | `aigent build "Process PDFs" --no-llm` → exit 0, creates dir | happy path |
| 46 | `aigent build "Process PDFs" --no-llm --name my-pdf-tool` → uses name | override |
| 47 | `aigent build "Process PDFs" --no-llm --dir <tmpdir>` → uses dir | dir-override |
| 48 | `aigent init` in empty dir → exit 0, creates SKILL.md | happy path |
| 49 | `aigent init` where SKILL.md exists → exit 1 | error |
| 50 | `aigent init <tmpdir>` → creates in specified dir | dir-arg |
| 51 | Built skill passes `aigent validate` | round-trip |

---

## Wave 5 — Verify (depends on Wave 4)

Single agent runs the full check suite on `dev/m07`.

| Agent | Branch | Task |
|-------|--------|------|
| E | `dev/m07` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release` |

---

## Deliverables

- `src/builder/mod.rs` — `build_skill`, `init_skill`, LLM integration, re-exports
- `src/builder/deterministic.rs` — `derive_name`, `generate_description`,
  `generate_body`, `assess_clarity`
- `src/builder/template.rs` — `skill_template` for `init`
- `src/builder/llm.rs` — `LlmProvider` trait, `detect_provider`
- `src/builder/providers/` — Anthropic, OpenAI, Google, Ollama implementations
- `src/main.rs` — `Build` and `Init` handlers wired up
- `src/lib.rs` — `init_skill` re-export added
- `Cargo.toml` — `ureq` (json feature)
- 51 tests (38 unit + 7 integration + 6 mocked LLM)
- PR: `M7: Skill Builder`
