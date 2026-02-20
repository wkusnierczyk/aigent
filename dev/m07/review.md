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

- [ ] Finding 1 resolved: `no_llm` field sequencing / breaking change strategy
- [ ] Finding 2 resolved: `output_dir` ownership and double-derivation
- [ ] Finding 3 addressed: Ollama probe latency documented or mitigated
- [ ] Finding 4 noted: `init_skill` error message mentions found file
- [ ] Finding 6 considered: direct `SkillProperties` construction
- [ ] Finding 7 noted: `reqwest` dependency trade-off documented
- [ ] Finding 8 addressed: model env var overrides added
