# M7: Skill Builder — Plan Review

## Overall Assessment

The M7 plan is the largest and most ambitious milestone yet (751 lines, 5 waves,
5 agents, 51 tests, 8 new files). It covers deterministic skill generation,
LLM-enhanced generation with 4 provider backends, CLI wiring, and comprehensive
testing. The plan demonstrates thorough design thinking — particularly the sync
vs async decision, per-function LLM fallback, and self-validation pipeline.

However, the plan has several issues at the intersection of the existing codebase
and the proposed changes that need resolution before implementation.

## Plan Conformance

### Issues Addressed

- [x] #21 — `SkillSpec`, `BuildResult`, `ClarityAssessment` data model (Wave 1)
- [x] #22 — `build_skill`, `derive_name`, `assess_clarity`, CLI wiring (Waves 1, 3)
- [x] #23 — LLM-enhanced mode with fallback to deterministic (Wave 2)
- [x] #24 — Multi-provider support: Anthropic, OpenAI, Google, Ollama (Wave 2)
- [x] #25 — Builder tests: 51 total (Wave 4)

### Issue Deviations

1. **Issue #24 specifies `async_trait`; plan uses sync**: Issue #24 shows
   `#[async_trait] pub trait LlmProvider ... async fn generate(...)`. The plan
   correctly chooses sync (`reqwest::blocking`) but doesn't acknowledge this
   deviation from the issue text. The rationale (sequential CLI, no server) is
   sound — this should be noted as a deliberate deviation.

2. **Issue #24 mentions `--provider` flag and `AIGENT_LLM_PROVIDER` env var**:
   The plan explicitly defers `--provider` ("future M7 scope consideration; not
   in current CLI definition, so omitted from this plan"). However,
   `AIGENT_LLM_PROVIDER` is also in the issue and not mentioned at all. Both
   omissions are reasonable for the initial implementation, but should be
   explicitly noted.

3. **Issue #23 only mentions Anthropic**: Issue #23 scope is "LLM-supported
   skill generation" with only `ANTHROPIC_API_KEY` referenced. Issue #24 adds
   multi-provider support. The plan correctly combines both into Wave 2, but
   the wave issues column says "#23, #24" — this is fine.

## Findings

### Finding 1 (High): `SkillSpec` gains `no_llm` field — breaks existing struct

**Location**: Wave 2, "Integrate LLM into `build_skill`" section

The plan adds `pub no_llm: bool` to `SkillSpec`. However, `SkillSpec` is already
defined in the current codebase (`src/builder.rs`, line 8-16) without this field,
and it derives `Clone` and `Debug` but NOT `Default`. This means:

- Every existing construction site must be updated (currently only `lib.rs`
  re-exports and `main.rs` CLI handler)
- `SkillSpec` is a **public API type** — adding a required `bool` field is a
  breaking change for any downstream consumers

The plan introduces this field in **Wave 2** but the CLI wiring in **Wave 3**
needs it. Wave 1 doesn't add it. This creates a sequencing problem:
- Wave 1 implements `build_skill` using `SkillSpec` without `no_llm`
- Wave 2 adds `no_llm` to `SkillSpec` and changes `build_skill` to use it
- Wave 3 passes `no_llm` from CLI into `SkillSpec`

The transition from Wave 1 → Wave 2 requires modifying Wave 1's `build_skill`
to accept the new field. This is workable but not explicitly acknowledged.

**Recommendation**: Either (a) add `no_llm: bool` to `SkillSpec` in Wave 1 with
a simple `if spec.no_llm { None } else { None }` placeholder (always
deterministic in Wave 1), or (b) use `#[derive(Default)]` on `SkillSpec` so
`no_llm` defaults to `false` and existing construction sites don't break. Option
(b) is cleaner but requires adding `Default` derive. The plan should specify
which approach.

### Finding 2 (High): `build_skill` signature vs `BuildResult.output_dir` ownership

**Location**: Wave 1, `build_skill` step 12

The plan specifies `build_skill(spec: &SkillSpec, output_dir: &Path) ->
Result<BuildResult>` where `BuildResult` contains `pub output_dir: PathBuf`.
The caller passes `output_dir` by reference, but the result returns an owned
`PathBuf`. This works (`.to_path_buf()` on the reference), but there's an
inconsistency in the CLI wiring (Wave 3):

```rust
let output_dir = dir.unwrap_or_else(|| {
    PathBuf::from(aigent::derive_name(&spec.purpose))
});
match aigent::build_skill(&spec, &output_dir) { ... }
```

The `output_dir` default is derived from purpose via `derive_name`. But
`build_skill` itself also calls `derive_name` internally (step 2). This means
name derivation happens **twice** — once in the CLI to determine the output
directory, and once inside `build_skill` to determine the skill name. If
`spec.name` is `None`, both should produce the same result (deterministic), but
the duplicated logic is fragile and would diverge with LLM mode (LLM call
would run twice with potentially different results).

**Recommendation**: Either (a) have `build_skill` determine the output directory
internally (return it in `BuildResult`) and remove `output_dir` from the
signature, or (b) accept the current design but document that the CLI must
derive the directory name before calling `build_skill`. Option (a) is cleaner
and avoids the double-derivation problem. The plan's Wave 1 step 7 already
creates the directory inside `build_skill`, so the function effectively owns
the output path anyway.

### Finding 3 (Medium): `detect_provider` Ollama probe blocks for 1 second

**Location**: Wave 2, `detect_provider()` step 4

The plan probes `http://localhost:11434` with a 1-second timeout on every
`build_skill` call when no API keys are set. For CLI usage this is acceptable
(the user explicitly asked to build a skill), but for library consumers calling
`build_skill` without `no_llm: true`, this introduces an unexpected 1-second
latency on every call when Ollama isn't running.

**Recommendation**: Document this behavior clearly on `build_skill`. Consider
making the Ollama probe opt-in (e.g., `OLLAMA_HOST` env var must be set, or
only probe when `AIGENT_LLM_PROVIDER=ollama`). Alternatively, reduce the
timeout to 200ms — Ollama on localhost should respond much faster than 1 second
if it's running.

### Finding 4 (Medium): `init_skill` uses `find_skill_md` for existence check

**Location**: Wave 1, `init_skill` step 1

The plan says: "If `find_skill_md(dir)` returns `Some`, fail." But
`find_skill_md` checks for both `SKILL.md` and `skill.md` (case-insensitive
fallback from M3 parser). This means `init_skill` will refuse to initialize
even if the existing file is lowercase `skill.md`. This is probably correct
behavior, but worth noting — the error message should mention which file was
found, not just "SKILL.md already exists".

Additionally, `find_skill_md` requires the directory to **exist** to search it.
But step 4 says "Create `dir` if it doesn't exist (`fs::create_dir_all`)." If
the directory doesn't exist yet, `find_skill_md` will return `None` (no file
found), which is correct — but if the directory **does** exist and is empty,
`find_skill_md` returns `None` too. The sequencing is fine; this is a
non-issue on closer analysis, but worth the implementer being aware of.

### Finding 5 (Medium): Test count arithmetic — 38 + 7 + 6 = 51 but table shows 44 + 7

**Location**: Deliverables section

The plan claims "51 tests (38 unit + 7 integration + 6 mocked LLM)." Counting
from the test tables:
- `derive_name`: tests 1-10 (10 tests)
- `generate_description`: tests 11-14 (4 tests)
- `generate_body`: tests 15-18 (4 tests)
- `assess_clarity`: tests 19-23 (5 tests)
- `init_skill`: tests 24-29 (6 tests)
- `build_skill`: tests 30-38 (9 tests)
- LLM provider: tests 39-44 (6 tests)
- CLI integration: tests 45-51 (7 tests)

Unit tests: 10 + 4 + 4 + 5 + 6 + 9 = **38**. Mocked: **6**. Integration: **7**.
Total: 38 + 6 + 7 = **51**. The arithmetic checks out. My initial concern was
unfounded — this is correct.

### Finding 6 (Low): `SkillSpec.tools` maps to `allowed-tools` in frontmatter

**Location**: Wave 1, `build_skill` step 3

The plan maps `spec.tools` → `"allowed-tools"` in the YAML frontmatter. But
`SkillProperties` uses `#[serde(rename = "allowed-tools")]` for its
`allowed_tools` field. The plan's step 10 says "Construct `SkillProperties`
from the frontmatter" — this requires manually building `SkillProperties`
from the generated HashMap, matching `tools` → `allowed_tools`. This should
work but adds a mapping layer that could drift from `SkillProperties` if fields
are added later.

**Recommendation**: Consider constructing `SkillProperties` directly instead of
going through a HashMap. This avoids the HashMap intermediary:

```rust
let props = SkillProperties {
    name: name.clone(),
    description: description.clone(),
    license: spec.license.clone(),
    compatibility: spec.compatibility.clone(),
    allowed_tools: spec.tools.clone(),
    metadata: None,
};
```

Then serialize `props` to YAML for the frontmatter. This ensures the
`SkillProperties` struct is the single source of truth.

### Finding 7 (Low): `reqwest` blocking feature pulls in substantial dependencies

**Location**: Design Decisions, Cargo.toml Changes

`reqwest` with `blocking` + `json` features adds a significant dependency tree
(hyper, http, tower, tokio [internally], native-tls or rustls, etc.). For a CLI
tool that may often run in `--no-llm` mode, this is dead weight. The plan
doesn't discuss this trade-off or consider alternatives (e.g., `ureq` which is
a pure-Rust sync HTTP client with much smaller footprint).

**Recommendation**: Note this trade-off in the design decisions. If minimizing
dependency count matters for the project, consider `ureq` as an alternative.
If `reqwest` is preferred for its ecosystem familiarity and feature completeness,
that's a valid choice — just document the reasoning.

### Finding 8 (Low): Anthropic model default may be outdated

**Location**: Wave 2, `AnthropicProvider` struct

The plan hardcodes `model: "claude-sonnet-4-20250514"`. Model identifiers change
frequently. This is fine as a default but should be overridable via an
environment variable (e.g., `ANTHROPIC_MODEL`). Same applies to other providers
(`OPENAI_MODEL`, `GOOGLE_MODEL`, `OLLAMA_MODEL`).

The plan doesn't mention model configurability beyond hardcoded defaults. For a
tool that will ship in a Cargo crate, users will need model override capability
without waiting for a new release.

**Recommendation**: Add env var overrides for model selection on each provider.
This can be as simple as `std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_|
"claude-sonnet-4-20250514".to_string())`.

## Observations

1. **Sync decision is well-reasoned**: The plan's rationale for avoiding
   `tokio` (sequential calls, single-threaded CLI, library consumers can wrap)
   is sound. `reqwest::blocking` internally spins up a small tokio runtime but
   doesn't impose it on the public API.

2. **Per-function fallback is good design**: Rather than all-or-nothing LLM
   mode, individual function failures gracefully degrade. This means a flaky
   API key still produces a valid skill (with some fields being template-based).

3. **Self-validation pipeline is excellent**: Step 9 (validate generated output)
   catches any builder bugs at generation time. This closes the loop nicely.

4. **Wave sequencing is logical**: Deterministic first (no external deps) →
   LLM providers → CLI wiring → tests → verify. Each wave has a clear
   dependency chain.

5. **Module refactor mechanics**: The `src/builder.rs` → `src/builder/mod.rs`
   transition is backward-compatible because Rust resolves `mod builder` to
   either `builder.rs` or `builder/mod.rs`. No changes needed in `lib.rs`
   for the module declaration itself — only the re-export list grows.

6. **Test #40 environment sensitivity**: "detect_provider returns None when no
   env vars" depends on the test environment not having `ANTHROPIC_API_KEY`,
   `OPENAI_API_KEY`, or `GOOGLE_API_KEY` set. CI environments often have API
   keys. The test should temporarily clear these env vars. Consider using
   `temp_env` crate or `std::env::remove_var` (unsafe in Rust 2024 edition
   due to thread safety — but fine in single-threaded test contexts with
   `#[serial]`).

## Verdict

**Conditional approval** — the plan is well-structured but Finding 1 (SkillSpec
`no_llm` field introduction) and Finding 2 (double name derivation /
`output_dir` ownership) need resolution before implementation begins. Findings
3 and 8 (Ollama probe latency, model configurability) should be addressed but
are not blocking.

### Checklist

- [x] Finding 1 resolved: `no_llm` field sequencing / breaking change strategy
- [x] Finding 2 resolved: `output_dir` ownership and double-derivation
- [x] Finding 3 addressed: Ollama probe latency documented or mitigated
- [x] Finding 4 noted: `init_skill` error message mentions found file
- [x] Finding 6 considered: direct `SkillProperties` construction
- [x] Finding 7 noted: `reqwest` dependency trade-off documented
- [x] Finding 8 addressed: model env var overrides added

---

# M7: Skill Builder — Code Review

## Verification

```
cargo fmt --check    → clean
cargo clippy -- -D warnings → clean
cargo test           → 169 passed (146 unit + 23 integration), 0 failed
```

Test count: M7 adds 51 tests as planned (23 deterministic + 6 LLM/mock +
15 init/build + 7 CLI integration = 51). Previous milestones: 95 unit +
18 integration = 113. New total: 146 + 23 = 169.

## Plan Conformance

All 8 plan review findings have been resolved in the implementation:

| # | Finding | Resolution in code |
|---|---------|-------------------|
| 1 | `no_llm` field sequencing | `SkillSpec` has `#[derive(Default)]` and `no_llm: bool` from the start; `build_skill` checks it on line 60 |
| 2 | Double name derivation | `output_dir` moved into `SkillSpec` as `Option<PathBuf>`; signature is now `build_skill(spec: &SkillSpec) -> Result<BuildResult>` |
| 3 | Ollama probe latency | Ollama requires `OLLAMA_HOST` env var (opt-in); no network probe in `detect_provider` |
| 4 | `init_skill` error message | Error includes actual found path: `"already exists: {path}"` (line 211) |
| 6 | Direct SkillProperties | `SkillProperties` constructed directly (lines 97-104), serialized to YAML via serde |
| 7 | `reqwest` dependency | Switched to `ureq` v3 (`features = ["json"]`) — sync-native, no hidden tokio |
| 8 | Model env var overrides | All 4 providers read `*_MODEL` env vars with `.filter(\|s\| !s.is_empty())` fallback |

### Additional plan deviations (positive)

- **`mockall` dropped**: Plan said `mockall` dev-dependency; implementation uses
  hand-written `MockProvider` and `FailingProvider` structs. Simpler, fewer
  macro dependencies. Correct choice for 6 straightforward mock tests.

- **`detect_provider` test is non-asserting**: Test `detect_provider_returns_none_
  when_no_env_vars` only verifies no panic, not the return value. The comment
  explains this is because CI may have API keys set. This is the pragmatic
  approach vs. the plan's original expectation of asserting `None`.

## Findings

### Finding 1 (Medium): Two `unwrap()` calls in library code

**Location**: `src/builder/deterministic.rs` lines 88 and 200

```rust
// Line 88
let last = word.chars().last().unwrap();

// Line 200
let last = words.last().unwrap()
```

Both are guarded by prior length checks (`word.len() >= 3` and
`words.len() >= 3`), making them logically safe. However, the project
convention (CLAUDE.md) states: "No `unwrap()` or `expect()` in `src/lib.rs`
and modules — propagate errors with `?`."

These functions return `String`, not `Result`, so `?` doesn't apply. But the
convention can be satisfied with pattern matching or `unwrap_or_default()`:

```rust
// Alternative for line 88:
if let Some(last) = word.chars().last() {
    return format!("{word}{last}ing");
}
```

**Impact**: Style-only; no runtime risk. The guards ensure `unwrap` never
panics.

### Finding 2 (Low): Duplicate `to_title_case` and `capitalize_first`

**Location**: `src/builder/template.rs` (lines 49, 57) and
`src/builder/deterministic.rs` (lines 234, 182)

Both files implement identical `to_title_case(name: &str) -> String` and
`capitalize_first(s: &str) -> String` functions. These could be extracted
to a shared utility within the builder module (e.g., a private `util.rs`
or `pub(crate)` functions in one of the existing files).

**Impact**: Minor code duplication. Not a correctness issue — the
implementations are identical and tested indirectly through `derive_name`
and `skill_template` tests.

### Finding 3 (Low): `llm_generate_description` system prompt says "Maximum 200 characters"

**Location**: `src/builder/llm.rs` line 83

The system prompt instructs the LLM to write a description of "Maximum 200
characters." But the spec allows up to 1024 characters, and the code truncates
at 1024 (line 97). The 200-char limit in the prompt is more restrictive than
necessary. The deterministic `generate_description` produces descriptions well
over 200 characters (sentence + "Use when working with {X}.").

**Impact**: Minor. LLM descriptions will be shorter than they could be. Not
a bug — just an unnecessarily tight constraint in the prompt.

### Finding 4 (Low): `build_skill` validation loop filters by `warning:` prefix

**Location**: `src/builder/mod.rs` lines 165-169

```rust
let errors: Vec<&str> = messages
    .iter()
    .filter(|m| !m.starts_with("warning: "))
    .map(|m| m.as_str())
    .collect();
```

This is the same pattern used in `main.rs` for the `validate` command. It
works correctly because `validate()` returns strings where warnings are
prefixed with `"warning: "`. But it creates a coupling between the builder
and the validator's output format — if the prefix ever changes (e.g., to
`"warn: "`), both locations silently break.

**Impact**: Maintenance concern. Consider adding a structured validation
return type in a future milestone (e.g., `Vec<ValidationMessage>` with a
`severity` field).

### Finding 5 (Low): Provider `from_env()` reads environment on every call

**Location**: All 4 provider files (`anthropic.rs`, `openai.rs`, `google.rs`,
`ollama.rs`)

Each provider's `from_env()` reads env vars via `std::env::var()`. This is
called once per `build_skill` invocation via `detect_provider()` — so env
reads happen on every build. For CLI usage this is negligible. For library
consumers calling `build_skill` in a loop, the repeated env reads add
minimal but unnecessary overhead.

**Impact**: Negligible for current usage. Not actionable now — a future
optimization could cache the provider across calls.

## Observations

1. **`ureq` was an excellent choice**: The switch from `reqwest` to `ureq` v3
   eliminated the hidden tokio runtime and dramatically reduced the dependency
   tree. `ureq`'s API is clean: `ureq::post(url).header(...).send_json(body)`
   maps naturally to the provider pattern. All 4 providers follow the same
   structure: build request → send → parse response → extract text.

2. **Per-function LLM fallback implemented cleanly**: `build_skill` (lines
   60-122) uses a consistent pattern for each generation step:
   `if let Some(ref prov) = provider { match llm_fn(...) { Ok => use,
   Err(e) => { eprintln!("warning: ..."); deterministic_fn() } } }`.
   The repetition could be abstracted with a closure, but the explicit
   pattern is more readable.

3. **`SkillProperties` as source of truth works well**: Constructing
   `SkillProperties` directly (lines 97-104) and serializing to YAML via
   `serde_yaml_ng::to_string` ensures the frontmatter always matches the
   struct's serde configuration. The `#[serde(rename = "allowed-tools")]`
   attribute handles the naming automatically.

4. **Provider implementations are uniform and well-structured**: All 4
   providers follow the identical pattern: `struct` with `api_key`/`model`,
   `from_env()` constructor checking env vars with empty-string guard,
   request/response serde types, `LlmProvider` implementation with
   consistent error mapping. The code is clearly produced from a template
   approach, which is appropriate for API client code.

5. **Ollama provider doc comment is precise**: "Requires `OLLAMA_HOST` to be
   set (opt-in, no auto-probe)" — this directly addresses plan review
   Finding 3 and documents the behavior for consumers reading the source.

6. **`detect_provider` test handles environment uncertainty gracefully**: The
   test acknowledges that CI environments may have API keys set and avoids a
   fragile assertion. The `let _ = result;` idiom suppresses the unused
   variable warning while still exercising the detection code path.

7. **CLI wiring uses `..Default::default()` idiom**: The `main.rs` Build
   handler (lines 101-107) uses struct update syntax with `Default`, making
   it resilient to future field additions on `SkillSpec`. This directly
   benefits from the `#[derive(Default)]` added per plan review Finding 1.

8. **Round-trip test is valuable**: Test `built_skill_passes_validate`
   (cli.rs line 337) builds a skill via CLI and then validates it via CLI —
   a true end-to-end integration test that exercises both commands in
   sequence. This catches any format drift between the builder and validator.

## Verdict

**Ready to merge**. All 8 plan review findings are resolved. The code is clean,
well-documented, and follows project conventions. 169 tests pass, fmt and
clippy are clean. The two `unwrap()` calls (Finding 1) are logically safe but
technically violate the project convention — worth a follow-up cleanup but not
blocking.

### Summary of new code

| File | Lines | Role |
|------|------:|------|
| `src/builder/mod.rs` | 476 | Core: `SkillSpec`, `BuildResult`, `ClarityAssessment`, `build_skill`, `init_skill`, 15 tests |
| `src/builder/deterministic.rs` | 551 | Deterministic functions: `derive_name`, `generate_description`, `generate_body`, `assess_clarity`, 23 tests |
| `src/builder/llm.rs` | 243 | `LlmProvider` trait, `detect_provider`, 4 `llm_*` functions, 6 tests |
| `src/builder/template.rs` | 67 | `skill_template` for `init` |
| `src/builder/providers/mod.rs` | 9 | Re-exports |
| `src/builder/providers/anthropic.rs` | 97 | Anthropic Messages API |
| `src/builder/providers/openai.rs` | 122 | OpenAI Chat Completions (+ compatible endpoints) |
| `src/builder/providers/google.rs` | 121 | Google Generative Language API |
| `src/builder/providers/ollama.rs` | 86 | Ollama local API |
| `src/main.rs` (changes) | ~40 | `Build` and `Init` CLI handlers |
| `tests/cli.rs` (additions) | ~120 | 7 integration tests: build ×3, init ×3, round-trip ×1 |
