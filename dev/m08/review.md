# M8: Main Module & Documentation — Plan Review

## Overall Assessment

The M8 plan covers API audit, documentation (README, CHANGES.md), and a release
workflow. It is well-structured, appropriately scoped, and follows the
established wave pattern. The deliverables are documentation-heavy rather than
code-heavy, which is appropriate for a late-stage milestone.

However, the plan's "current state" section is partially outdated — it describes
a pre-M7 state for some items while assuming a post-M7 state for others. The
actual codebase reveals M7 was only partially integrated (LLM functions exist
but `build_skill` never calls them), which the lib.rs audit needs to account for.

## Plan Conformance

### Issues Addressed

- [x] #26 — lib.rs public API audit, re-exports, crate-level docs (Wave 1)
- [x] #27 — README with usage examples, badges, CLI docs (Wave 2)
- [x] #28 — CHANGES.md for v0.1.0 (Wave 1)
- [x] #29 — GitHub Actions release workflow (Wave 3)
- [x] #30 — Spec compliance comparison table in README (Wave 2)

### Issue Deviations

1. **Issue #26 lists specific re-exports**: The issue shows `pub use builder::
   {build_skill, BuildResult, SkillSpec}` — notably omitting `derive_name`,
   `assess_clarity`, `ClarityAssessment`, `init_skill`, and `LlmProvider`. The
   plan correctly expands beyond the issue's list. This is a positive deviation
   — the issue was written early; the plan reflects the actual M7 API surface.

2. **Issue #27 mentions "API reference (link to docs.rs or inline)"**: The plan
   handles this via `#[doc(inline)]` attributes and the crate-level doc
   comment with `rust,no_run` examples, but does not include a dedicated "API
   Reference" section in the README. The "Library Usage" section partially
   fills this role. Given that `docs.rs` auto-generates API docs from Cargo
   publish, a README link to `docs.rs/aigent` would satisfy this requirement.

3. **Issue #30 table differs from plan table**: Issue #30 has 9 rows; the plan
   has 12 rows (adds "Compatibility ≤ 500 chars", "Path canonicalization",
   "Post-build validation"). The extra rows improve completeness — this is a
   positive deviation.

## Findings

### Finding 1 (High): `build_skill` does not use LLM mode — dead code in API

**Location**: Design Decisions, lib.rs Audit Scope

The plan assumes M7 is complete and the API is settled. However, `build_skill`
(lines 58–143 of `src/builder/mod.rs`) always uses the deterministic path. It
never calls `detect_provider()`, never checks `spec.no_llm`, and never invokes
`llm_derive_name`, `llm_generate_description`, or `llm_generate_body` — even
though all these functions are implemented and imported (line 16).

This means:
- The `no_llm` field on `SkillSpec` has no runtime effect
- `detect_provider()` is never called
- All LLM provider implementations (`AnthropicProvider`, `OpenAiProvider`,
  `GoogleProvider`, `OllamaProvider`) are dead code
- The `llm_*` functions in `llm.rs` are dead code

The M8 lib.rs audit must decide: is this intentional (M7 Wave 2 integration
deferred) or an oversight? The plan's re-export table considers `LlmProvider`
for re-export — but re-exporting dead code sends the wrong signal to library
consumers.

**Recommendation**: Before the M8 audit, either (a) complete the M7 LLM
integration so `build_skill` actually uses providers when `no_llm` is false,
or (b) explicitly document the `LlmProvider` re-export with a doc comment
noting it is "available for custom use but not yet called by `build_skill`."
Option (a) is strongly preferred — shipping 0.1.0 with dead LLM code and a
`no_llm` flag that does nothing would confuse users.

### Finding 2 (Medium): `LlmProvider` re-export path unclear

**Location**: Wave 1, Re-export decisions table

The plan says `LlmProvider` → "Yes — Enables custom provider implementations."
But `LlmProvider` lives in `builder::llm::LlmProvider`. The current `lib.rs`
re-export block only imports from `builder::*`, not `builder::llm::*`:

```rust
pub use builder::{
    assess_clarity, build_skill, derive_name, init_skill,
    BuildResult, ClarityAssessment, SkillSpec,
};
```

To re-export `LlmProvider` at the crate root, either:
- `builder/mod.rs` must re-export it: `pub use llm::LlmProvider;`
- Or `lib.rs` must import directly: `pub use builder::llm::LlmProvider;`

The plan doesn't specify which approach. The former is cleaner (keeps `lib.rs`
importing only from `builder::*`). The plan should note this.

Additionally, if `LlmProvider` is re-exported, consumers need access to
`Result<String>` (the return type) — which is already `crate::errors::Result`,
re-exported as `aigent::Result`. This is fine.

### Finding 3 (Medium): Spec compliance table missing rows present in codebase

**Location**: Design Decisions, Spec Compliance Table

The table has 12 rows but the actual validator (`src/validator.rs`) implements
additional checks not represented:

- **Description: third-person voice** (checked in `validate_metadata` — though
  this may be a heuristic not worth claiming in a compliance table)
- **Metadata key validation** (nested key format, unknown metadata keys as
  warnings)
- **Frontmatter required fields** (name + description present)

These are important validator features. The table should either include them or
the table header should clarify it shows "key differentiating rules" rather
than all rules.

Also: the "Compatibility ≤ 500 chars" row shows `❌` for Python Ref. The
Python reference implementation may actually check this — worth verifying
before publishing. An inaccurate compliance table undermines credibility.

### Finding 4 (Medium): Release workflow `sed` extraction fragile

**Location**: Wave 3, Job 3 steps

The changelog extraction command:
```bash
sed -n "/^## \[${VERSION}\]/,/^## \[/{/^## \[${VERSION}\]/d;/^## \[/d;p;}" CHANGES.md
```

This is fragile:
- `VERSION` may contain dots that `sed` interprets as regex wildcards
  (`0.1.0` matches `0X1Y0` in regex). The dots should be escaped: `0\.1\.0`.
- If the version is the last section (no next `## [` header), the `sed` range
  will extend to EOF — but the inner deletion pattern `{/^## \[/d;p;}` won't
  match, so the content still gets printed. This actually works correctly.
- If `CHANGES.md` uses `## [Unreleased]` as the section header, the version
  won't match. The plan says to use either `Unreleased` or the planned date
  during M8 — this must be finalized to the actual version before tagging.

**Recommendation**: Use a simpler `awk`-based extraction or a dedicated
changelog tool (`changelog-extract`, `git-cliff`). Alternatively, escape the
dots: `VERSION_ESCAPED=$(echo "$VERSION" | sed 's/\./\\./g')`.

### Finding 5 (Medium): `cargo doc --no-deps` in Wave 4 but no `#![warn(missing_docs)]`

**Location**: Wave 4, Verify

The plan adds `cargo doc --no-deps` to the verification suite. This catches
broken doc links and doc-test failures, but does NOT catch missing doc comments.
For a library publishing to crates.io, adding `#![warn(missing_docs)]` to
`lib.rs` would enforce documentation coverage on all public items.

Currently, some public items lack doc comments (e.g., individual fields on
`SkillSpec`, `BuildResult`, `ClarityAssessment`). Adding `missing_docs` as a
warning (not deny) would surface these during `cargo doc`.

**Recommendation**: Add `#![warn(missing_docs)]` to `lib.rs` as part of the
audit in Wave 1. This aligns with the project convention of documenting public
items (CLAUDE.md: "Public items must have doc comments").

### Finding 6 (Low): README code examples use `?` operator without context

**Location**: Wave 2, Library Usage section

The plan shows:
```rust
let props = aigent::read_properties(Path::new("my-skill"))?;
```

The `?` operator requires a function that returns `Result`. In a README code
block, this needs either `fn main() -> Result<(), Box<dyn std::error::Error>>`
wrapping or `# use` hidden lines. The plan uses `rust,no_run` in the lib.rs
doc comment (Agent A), but the README examples (Agent C) don't specify fencing
attributes.

Since README code blocks aren't compiled by `cargo test`, this is purely a
readability issue. But using `?` without showing the `main` wrapper can confuse
newcomers.

**Recommendation**: Either wrap in a `main` function or use `.unwrap()` for
README examples (cleaner for quick-start context). Reserve `?` for the lib.rs
doc-tests which are compiled.

### Finding 7 (Low): CHANGES.md has no "Changed" or "Fixed" sections

**Location**: Wave 1, Agent B

The plan's CHANGES.md only has an "Added" section for v0.1.0. This is correct
for an initial release (nothing to change or fix). However, the design
decisions section says "Each version section has categories: Added, Changed,
Fixed" — the template should include a note or comment about adding these
sections as needed in future releases, or simply omit the description since
the template speaks for itself.

Minor point — not actionable unless the team wants placeholder sections.

### Finding 8 (Low): `cross` version not pinned in release workflow

**Location**: Wave 3, Job 2 steps

The plan installs `cross` for aarch64 Linux builds but doesn't pin its
version. `cross` is installed via `cargo install cross`, which pulls the latest
version. CI workflows should pin tool versions for reproducibility:
```yaml
- run: cargo install cross --version 0.2.5
```

Without pinning, a breaking `cross` release could silently break the release
workflow.

### Finding 9 (Low): `publish` job runs parallel to `build` — no binary validation

**Location**: Wave 3, Job 4

The plan notes: "This job runs in parallel with `build` — it only needs tests
to pass, not the binary builds." This is efficient but means `cargo publish`
can succeed even if cross-compilation fails. For v0.1.0, this is probably fine
(the crate publishes source, not binaries). But worth noting: if a
cross-compilation issue reveals a platform-specific bug, the crate will already
be published.

## Observations

1. **Documentation milestone is well-scoped**: The plan correctly identifies
   this as an audit + docs milestone, not a feature milestone. The work is
   mostly additive (new files, expanded existing file) with minimal risk to
   working code.

2. **Wave parallelism is good**: Waves 1A (lib.rs) and 1B (CHANGES.md) can
   run in parallel. Wave 3 (release workflow) is independent of Wave 2
   (README). This maximizes throughput.

3. **`#[doc(inline)]` is tasteful**: Pulling key type docs into the crate root
   page improves the docs.rs experience without duplicating documentation. The
   selective application (only key types, not everything) shows restraint.

4. **Spec compliance table is a strong differentiator**: The three-way
   comparison (spec vs aigent vs Python ref) clearly communicates aigent's
   value. Having it in both the README and as a standalone issue deliverable
   (#30) ensures visibility.

5. **CI ↔ Release workflow overlap**: The CI workflow (`ci.yml`) runs on PR/push
   to main. The release workflow (`release.yml`) also runs `fmt + clippy + test`
   in its test job. This duplication is intentional (defense in depth) — a
   tagged release should pass all checks regardless of CI history.

6. **No test changes**: M8 adds no new tests. This is appropriate for a
   documentation milestone. The `cargo doc --no-deps` addition verifies doc
   compilation, which is the right level of testing for this scope.

## Verdict

**Conditional approval** — Finding 1 (dead LLM code) is a prerequisite
concern from M7, not an M8 plan issue per se, but the M8 plan should
acknowledge it and specify how the audit handles it. Finding 2 (LlmProvider
re-export path) and Finding 4 (sed extraction fragility) should be resolved.
Finding 5 (`missing_docs`) is recommended but not blocking.

If Finding 1 is out of scope for M8 (i.e., LLM integration is intentionally
deferred), the plan should add a note in the lib.rs audit acknowledging the
dead code and deciding whether to (a) re-export `LlmProvider` with a
"not yet integrated" doc comment, (b) omit the re-export until integration
is complete, or (c) suppress dead-code warnings with `#[allow(unused)]`.

### Checklist

- [ ] Finding 1 addressed: dead LLM code acknowledged and audit strategy defined
- [ ] Finding 2 resolved: `LlmProvider` re-export mechanism specified
- [ ] Finding 3 considered: compliance table completeness verified
- [ ] Finding 4 resolved: changelog extraction robustness improved
- [ ] Finding 5 considered: `#![warn(missing_docs)]` for public API coverage
- [ ] Finding 6 noted: README code examples use appropriate error handling
- [ ] Finding 8 noted: `cross` version pinned in release workflow

---

# M8: Main Module & Documentation — Code Review

## Verification

```
cargo fmt --check         # ✅ clean
cargo clippy -- -D warnings # ✅ clean
cargo test                # ✅ 170 passed (146 unit + 23 integration + 1 doc-test)
cargo doc --no-deps       # ✅ clean, no warnings
```

Note: The test count increased from 169 to 170 — the new doc-test in the
crate-level `//!` comment (`src/lib.rs` line 10) now compiles as part of
`cargo test`, adding 1 doc-test.

## Plan Review Finding Resolution

All 9 plan review findings have been addressed. Verification below.

### Finding 1 (High): Dead LLM code — ✅ Resolved

The plan review was written against a stale snapshot. The current
`build_skill` (lines 74–218 of `src/builder/mod.rs`) fully integrates LLM
support:

- Line 76: `detect_provider()` called unless `spec.no_llm` is true
- Lines 79–91: `llm_derive_name` with fallback to `deterministic::derive_name`
- Lines 100–110: `llm_generate_description` with fallback
- Lines 123–138: `llm_generate_body` with fallback
- All 4 provider implementations are reachable code

No dead code exists. `LlmProvider` re-export is appropriate.

### Finding 2 (Medium): `LlmProvider` re-export path — ✅ Resolved

Two-step approach implemented:
- `builder/mod.rs` line 11: `pub use llm::LlmProvider;`
- `lib.rs` line 49: `LlmProvider` included in `pub use builder::{...}`

This keeps `lib.rs` importing only from `builder::*`, consistent with the
existing pattern.

### Finding 3 (Medium): Compliance table completeness — ✅ Resolved

The README's spec compliance table (lines 92–117) includes:
- 14 rows (2 more than the plan's 12)
- Added "Frontmatter `---` delimiters" row per review recommendation
- Clarifying header note: "Table shows key validation rules from the
  Anthropic spec. Additional checks... are implemented but not listed."

### Finding 4 (Medium): `sed` extraction fragility — ✅ Resolved

`release.yml` lines 113–119 implement all recommended improvements:
- `VERSION_ESCAPED` with dots escaped (`sed 's/\./\\./g'`)
- Empty-file guard: `if [ ! -s release-notes.md ]` fails the job with
  a clear error message
- Version header deletion in `sed` pattern corrected

### Finding 5 (Medium): `#![warn(missing_docs)]` — ✅ Resolved

`lib.rs` line 22: `#![warn(missing_docs)]` added. All public items now
have doc comments:

- `SkillProperties` fields: `name`, `description`, `license`,
  `compatibility`, `allowed_tools`, `metadata` — all documented
- `SkillSpec` fields: `purpose`, `name`, `tools`, `compatibility`,
  `license`, `extra_files`, `output_dir`, `no_llm` — all documented
- `BuildResult` fields: `properties`, `files`, `output_dir` — all documented
- `ClarityAssessment` fields: `clear`, `questions` — all documented
- `AigentError` variants: `Parse`, `Validation`, `Io`, `Yaml`, `Build` —
  all documented with variant field docs

`cargo doc --no-deps` produces no warnings, confirming coverage is complete.

### Finding 6 (Low): README error handling — ✅ Resolved

README Library Usage section (lines 44–63) uses `.unwrap()` instead of `?`
for both `read_properties` and `build_skill` calls. This is cleaner for
quick-start context and doesn't require a `main()` wrapper.

### Finding 7 (Low): CHANGES.md sections — ✅ Noted

Only "Added" section present for v0.1.0, as expected for initial release.

### Finding 8 (Low): `cross` version pinned — ✅ Resolved

`release.yml` line 67: `cargo install cross --version 0.2.5`

### Finding 9 (Low): `publish` parallel to `build` — ✅ Noted

Accepted as-is. The `publish` job depends only on `test`, running parallel
to `build`. This is documented in the plan resolution.

## Code Findings

### Finding 1 (Low): Doc-test uses `no_run` — compilation-only testing

**Location**: `src/lib.rs` lines 10–20

The crate-level doc-test uses `rust,no_run`:
```rust
//! ```rust,no_run
//! let errors = aigent::validate(Path::new("my-skill"));
//! assert!(errors.is_empty());
//! ```
```

This compiles but doesn't execute. The `assert!(errors.is_empty())` line
would panic at runtime since "my-skill" doesn't exist — but because of
`no_run`, this is never discovered. The example is slightly misleading:
a reader might expect `validate` on a non-existent path to return an empty
vec, when it actually returns `["SKILL.md not found"]`.

**Recommendation**: Either remove the `assert!` (since it won't run anyway)
or change to a more honest example:
```rust
//! let errors = aigent::validate(Path::new("my-skill"));
//! // errors will contain diagnostics if the skill is invalid
//! ```

Not blocking — the doc-test compiles cleanly and `no_run` is appropriate
for examples that require filesystem state.

### Finding 2 (Low): `CHANGES.md` still says "Unreleased"

**Location**: `CHANGES.md` line 3

The changelog header reads `## [0.1.0] — Unreleased`. Before tagging
`v0.1.0`, this must be updated to the actual release date. The release
workflow's `sed` extraction matches `[0.1.0]`, not `[Unreleased]`, so
the current header will work — but "Unreleased" in the GitHub Release
notes looks unfinished.

**Recommendation**: Update to the actual date before creating the release
tag. This is a manual step that should be part of the release checklist.

### Finding 3 (Low): `release.yml` test job runs only on `ubuntu-latest`

**Location**: `.github/workflows/release.yml` lines 11–26

The release workflow's `test` job runs on `ubuntu-latest` only, while the
CI workflow (`ci.yml`) runs on a 3-OS matrix (ubuntu, macos, windows).
This means a platform-specific test failure could slip through a release
if it only fails on macOS or Windows.

The build matrix does compile on all platforms, so compilation issues are
caught. But test failures (e.g., path handling differences on Windows) are
only caught on Linux in the release flow.

**Recommendation**: Either expand the release test job to match CI's matrix,
or document that the release flow relies on CI having already passed for
the tagged commit. Since tags are typically created from `main` (which
requires CI to pass via branch protection), this is an acceptable tradeoff.

### Finding 4 (Low): README badges may show "not found" initially

**Location**: `README.md` lines 3–6

The crates.io and docs.rs badges link to `crates.io/crates/aigent` and
`docs.rs/aigent` — these won't resolve until the crate is actually
published. Before the first `cargo publish`, these badges will show
"not found" or error states.

This is expected for pre-release READMEs. After the first publish, they'll
work automatically. No action needed — just noting for awareness.

### Finding 5 (Low): Module doc comments added throughout

**Location**: `src/lib.rs`, `src/builder/mod.rs`, `src/builder/providers/mod.rs`

M8 added `///` doc comments on all `pub mod` declarations in `lib.rs`
(lines 24–35) and `//!` module-level docs on `providers/mod.rs` (lines
1–4). All builder submodules (`deterministic`, `llm`, `providers`,
`template`) have doc comments. The `util` module is correctly `mod util`
(not `pub mod`), keeping it crate-private.

The documentation is concise, accurate, and consistent in style. Good
polish work.

### Finding 6 (Low): `#[doc(inline)]` applied selectively

**Location**: `src/lib.rs` lines 38–50

`#[doc(inline)]` is applied to:
- `AigentError`, `Result` (line 38–39)
- `SkillProperties` (line 41)
- Builder types block (lines 46–50): `BuildResult`, `ClarityAssessment`,
  `LlmProvider`, `SkillSpec`

Not applied to: `parse_frontmatter`, `find_skill_md`, `read_properties`,
`KNOWN_KEYS`, `to_prompt`, `validate`, `validate_metadata`.

This is the right selection — key types get inline docs, while function
re-exports are fine as links. The asymmetry between `#[doc(inline)]` on
the builder block but not the parser/validator re-exports is deliberate:
the parser and validator functions are well-understood from their names,
while the builder types need their field documentation visible at the
crate root.

## Observations

1. **M7 code review findings addressed in M8**: The `unwrap()` calls in
   `deterministic.rs` (flagged as Medium in M7 code review) have been
   removed. The duplicate `capitalize_first`/`to_title_case` functions
   (flagged as Low) have been extracted to `builder/util.rs` and both
   `deterministic.rs` and `template.rs` import from it. Good cleanup.

2. **Google provider API key header change**: `google.rs` now sends the
   API key via the `x-goog-api-key` HTTP header instead of as a URL query
   parameter. This is a security improvement — API keys in URLs can leak
   through server logs, referrer headers, and browser history. The change
   is consistent with Google's own documentation.

3. **CHANGES.md content is accurate**: All listed features correspond to
   actual implemented functionality. The dual-mode builder, per-function
   fallback, all 4 LLM providers, cross-platform CI — all verified in the
   codebase.

4. **README structure matches the plan**: Badges, overview, installation,
   quick start, library usage, builder modes, spec compliance, CLI
   reference, license — all present and in the planned order. The provider
   detection table (lines 79–85) is a clear addition beyond the plan.

5. **Release workflow is well-structured**: 4 jobs (`test → build →
   release`, `test → publish`), `fail-fast: false` on the build matrix,
   artifact upload/download pattern, PowerShell for Windows packaging.
   The `merge-multiple: true` on `download-artifact` is a nice v4 feature
   that avoids nested directories.

6. **`KNOWN_KEYS` kept as public**: The plan review's re-export table
   said "Yes — useful for consumers building custom validators." Confirmed
   present in `lib.rs` line 42 via `pub use parser::{..., KNOWN_KEYS}`.

7. **Cargo.toml metadata complete for publishing**: `keywords`, `categories`,
   `repository`, `authors`, `license`, `description` — all filled in. The
   package is ready for `cargo publish`.

## Verdict

**Ready to merge** — all 9 plan review findings resolved, all verification
checks pass (fmt, clippy, 170 tests, doc), and the deliverables match the
plan. Six Low-severity code findings noted, none blocking.

### Checklist

- [x] Plan review findings 1–9 verified as resolved
- [x] `cargo fmt --check` clean
- [x] `cargo clippy -- -D warnings` clean
- [x] `cargo test` — 170 tests pass
- [x] `cargo doc --no-deps` clean, no warnings
- [x] `#![warn(missing_docs)]` active with full doc coverage
- [x] `LlmProvider` re-exported via two-step path
- [x] README structure matches plan
- [x] CHANGES.md content accurate
- [x] Release workflow implements all plan specs
- [ ] Finding 2 noted: update CHANGES.md date before release tag
