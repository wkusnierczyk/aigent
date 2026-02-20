# M8: Main Module & Documentation — Work Plan

## Overview

Finalize the public API surface (`lib.rs` audit), write user-facing
documentation (README.md, CHANGES.md, spec compliance table), and add a
GitHub Actions release workflow for cross-platform binary builds and
crates.io publishing.

Issues: #26, #27, #28, #29, #30.

## Branch Strategy

- **Dev branch**: `dev/m08` (created from `main`)
- **Task branches**: `task/m08-<name>` (created from `dev/m08`)
- After each wave, task branches merge into `dev/m08`
- After all waves, PR from `dev/m08` → `main`
- `main` is never touched directly
- PR body uses `Closes #N` in the Summary section to auto-close issues on merge

## Dependencies

- All library modules completed (M2–M7)
- CLI fully wired (M6–M7)
- CI workflow (`ci.yml`) already in place (M1)

## Current State

`src/lib.rs` has been incrementally built across M1–M7. It currently exports:

```rust
pub mod builder;
pub mod errors;
pub mod models;
pub mod parser;
pub mod prompt;
pub mod validator;

pub use errors::{AigentError, Result};
pub use models::SkillProperties;
pub use parser::{find_skill_md, parse_frontmatter, read_properties, KNOWN_KEYS};
pub use prompt::to_prompt;
pub use validator::{validate, validate_metadata};
pub use builder::{
    assess_clarity, build_skill, derive_name, BuildResult, ClarityAssessment, SkillSpec,
};
```

After M7, `init_skill` and `LlmProvider` (plus provider types) will also
need re-export consideration. The `xml_escape` function from `prompt.rs` is
`pub` but not re-exported at crate root.

`README.md` is a stub: one line with the crate description.

`CHANGES.md` does not exist.

`.github/workflows/ci.yml` exists (multi-OS matrix: ubuntu, macos, windows;
runs fmt, clippy, test, build). No release workflow exists.

---

## Design Decisions

### `lib.rs` Audit Scope

Issue #26 asks to "implement main module exports." The module structure and
most re-exports are already in place. The M8 task is an **audit**, not a
rewrite:

1. Verify all public functions/types intended for library consumers are
   re-exported at crate root
2. Add any missing re-exports from M7 (`init_skill`, potentially
   `LlmProvider`)
3. Remove any re-exports that shouldn't be public (e.g., `KNOWN_KEYS` is
   an implementation detail — consider whether it belongs in the public API)
4. Add a crate-level doc comment (`//!`) describing the library

#### Re-export decisions

| Symbol | Re-export? | Rationale |
|--------|-----------|-----------|
| `init_skill` | Yes | Public CLI feature, library consumers need it |
| `LlmProvider` | Yes | Enables custom provider implementations |
| `xml_escape` | No | Internal utility; consumers use `to_prompt` |
| `KNOWN_KEYS` | Yes | Useful for consumers building custom validators |
| `detect_provider` | No | Internal; consumers construct providers directly |
| Provider structs | No | Accessed via `builder::providers::*` submodule |

The exact M7 additions depend on the final M7 implementation — the audit
adjusts accordingly.

### README Structure

Follow the structure from issue #27:

1. **Header** — project name, badges (CI, crate version, docs.rs, license)
2. **Overview** — one paragraph: what aigent is, what it does
3. **Installation** — `cargo install aigent` + `cargo install --path .`
4. **Quick Start** — minimal CLI examples for each subcommand
5. **Library Usage** — Rust code examples (validate, read-properties, to-prompt,
   build)
6. **Builder Modes** — deterministic vs LLM-enhanced, provider auto-detection
7. **Spec Compliance** — three-way comparison table (issue #30)
8. **CLI Reference** — full subcommand/flag documentation
9. **License** — MIT with link

### Spec Compliance Table

Three-column comparison: Anthropic Spec vs aigent (Rust) vs Python Reference.
Sourced from issue #27 and #30:

| Rule | Anthropic Spec | aigent | Python Ref |
|------|:-:|:-:|:-:|
| Name ≤ 64 chars | ✅ | ✅ | ✅ |
| Name: lowercase + hyphens | ✅ | ✅ | ✅ |
| Name: no XML tags | ✅ | ✅ | ❌ |
| Name: no reserved words | ✅ | ✅ | ❌ |
| Name: Unicode NFKC | — | ✅ | ❌ |
| Description: non-empty | ✅ | ✅ | ✅ |
| Description ≤ 1024 chars | ✅ | ✅ | ✅ |
| Description: no XML tags | ✅ | ✅ | ❌ |
| Compatibility ≤ 500 chars | ✅ | ✅ | ❌ |
| Body ≤ 500 lines warning | ✅ | ✅ | ❌ |
| Prompt XML format | ✅ | ✅ | ✅ |
| Path canonicalization | — | ✅ | ✅ |
| Post-build validation | — | ✅ | ❌ |

This table goes in both the README (section 7) and is the deliverable for
issue #30.

### CHANGES.md Format

Keep-a-Changelog-inspired format, grouped by version. Each version section
has categories: Added, Changed, Fixed. The release workflow extracts the
section for the tagged version to populate GitHub Release notes.

```markdown
# Changes

## [0.1.0] — YYYY-MM-DD

### Added
- ...
```

The date is filled at release time. During M8, use `Unreleased` or the
planned date.

### Release Workflow Design

Triggered on tag push (`v*`). Steps:

1. **Test** — run full CI checks (fmt, clippy, test)
2. **Build** — cross-compile for 5 targets using `cross` or
   `cargo build --target`:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-pc-windows-msvc`
3. **Package** — create tarballs (`.tar.gz` for Linux/macOS, `.zip` for
   Windows) named `aigent-{version}-{target}.{ext}`
4. **Release** — create GitHub Release, extract changelog section, attach
   binary archives
5. **Publish** — `cargo publish` to crates.io (requires `CARGO_REGISTRY_TOKEN`
   secret)

Build strategy:
- Linux targets: use `ubuntu-latest` runner with `cross` for aarch64
- macOS targets: use `macos-latest` (Apple Silicon runner, supports both
  x86_64 via Rosetta and native aarch64)
- Windows: use `windows-latest` runner

The workflow uses a matrix for the build step and a separate job for release
creation (depends on all builds completing).

### Changelog Extraction

The release job extracts the version's changelog section using `sed` or a
small script:

```bash
sed -n "/^## \[${VERSION}\]/,/^## \[/p" CHANGES.md | head -n -1
```

This extracts everything between the current version header and the next
version header (or EOF).

---

## Wave 1 — lib.rs Audit + CHANGES.md (parallel)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| A | `task/m08-lib` | #26 | Audit and finalize `src/lib.rs` exports |
| B | `task/m08-changes` | #28 | Create `CHANGES.md` |

**Merge**: A, B → `dev/m08`. Checkpoint with user.

### Agent A — lib.rs Audit (#26)

#### 1. Add crate-level documentation

Add a `//!` doc comment at the top of `src/lib.rs`:

```rust
//! # aigent
//!
//! A Rust library for managing AI agent skill definitions (SKILL.md files).
//!
//! Implements the [Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
//! with validation, prompt generation, and skill building capabilities.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! // Validate a skill directory
//! let errors = aigent::validate(std::path::Path::new("my-skill"));
//! assert!(errors.is_empty());
//!
//! // Read skill properties
//! let props = aigent::read_properties(std::path::Path::new("my-skill")).unwrap();
//! println!("{}", props.name);
//! ```
```

#### 2. Verify re-exports

Check each public function/type across all modules. Add missing re-exports
from M7:

- Add `init_skill` (from `builder::init_skill`)
- Add `LlmProvider` (from `builder::llm::LlmProvider`) if it's a public
  trait consumers should implement

The exact list depends on M7's final API surface. Adjust as needed.

#### 3. Review existing re-exports

Confirm these are intentional public API:
- `KNOWN_KEYS` — useful for custom validator consumers → keep
- `parse_frontmatter` — low-level parser API → keep (advanced users)
- `validate_metadata` — accepts raw HashMap → keep (advanced users)
- `find_skill_md` — useful for tooling → keep

#### 4. Add `#[doc(inline)]` where helpful

For re-exported types that have significant doc comments, add
`#[doc(inline)]` to pull their docs into the crate-level documentation:

```rust
#[doc(inline)]
pub use models::SkillProperties;
```

This is optional polish — apply to key types (`SkillProperties`,
`AigentError`, `SkillSpec`, `BuildResult`).

### Agent B — CHANGES.md (#28)

Create `CHANGES.md` at the repository root:

```markdown
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
```

---

## Wave 2 — README + Compliance Table (depends on Wave 1)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| C | `task/m08-readme` | #27, #30 | Write README.md with spec compliance table |

**Merge**: C → `dev/m08`. Checkpoint with user.

### Agent C — README.md (#27, #30)

Replace the stub README with a complete document. Structure:

#### Badges

```markdown
[![CI](https://github.com/wkusnierczyk/aigent/actions/workflows/ci.yml/badge.svg)](https://github.com/wkusnierczyk/aigent/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aigent)](https://crates.io/crates/aigent)
[![docs.rs](https://docs.rs/aigent/badge.svg)](https://docs.rs/aigent)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
```

#### Overview

One paragraph: aigent is a Rust library, CLI, and Claude Code plugin for
managing AI agent skill definitions (SKILL.md files). Implements the Anthropic
agent skill specification with validation, prompt generation, and skill
building.

#### Installation

```bash
# From crates.io
cargo install aigent

# From source
cargo install --path .
```

#### Quick Start — CLI

One example per subcommand:

```bash
# Initialize a new skill
aigent init my-skill/

# Build a skill from a description
aigent build "Process PDF files and extract text" --no-llm

# Validate a skill directory
aigent validate my-skill/

# Read skill properties as JSON
aigent read-properties my-skill/

# Generate XML prompt for LLM injection
aigent to-prompt my-skill/ other-skill/
```

#### Library Usage

Rust code examples:

```rust
use std::path::Path;

// Validate
let errors = aigent::validate(Path::new("my-skill"));

// Read properties
let props = aigent::read_properties(Path::new("my-skill"))?;

// Generate prompt XML
let xml = aigent::to_prompt(&[Path::new("skill-a"), Path::new("skill-b")]);

// Build a skill
let spec = aigent::SkillSpec {
    purpose: "Process PDF files".to_string(),
    no_llm: true,
    ..Default::default()
};
let result = aigent::build_skill(&spec)?;
```

#### Builder Modes

Describe deterministic vs LLM-enhanced modes:
- Deterministic: always available, zero configuration, formulaic output
- LLM-enhanced: auto-detected via API key env vars, richer output,
  per-function fallback to deterministic on failure

Provider detection order:
1. `ANTHROPIC_API_KEY`
2. `OPENAI_API_KEY`
3. `GOOGLE_API_KEY`
4. `OLLAMA_HOST`

Model override env vars: `ANTHROPIC_MODEL`, `OPENAI_MODEL`, `GOOGLE_MODEL`,
`OLLAMA_MODEL`.

#### Spec Compliance

Include the three-way comparison table from the Design Decisions section.
Reference the Anthropic best-practices URL. Note where aigent exceeds the
spec (NFKC normalization, XML tag rejection in names, reserved word checks,
post-build validation).

#### CLI Reference

Concise table or list of all subcommands and flags:

| Command | Description |
|---------|-------------|
| `validate <dir>` | Validate a skill directory; exit 0 if valid |
| `read-properties <dir>` | Output skill properties as JSON |
| `to-prompt <dirs...>` | Generate `<available_skills>` XML block |
| `build <purpose>` | Build a skill from natural language |
| `init [dir]` | Create a template SKILL.md |

Flags: `--name`, `--dir`, `--no-llm`, `--about`, `--version`, `--help`.

#### License

MIT. Link to `LICENSE` file.

---

## Wave 3 — Release Workflow (depends on Wave 1)

| Agent | Branch | Issue | Task |
|-------|--------|-------|------|
| D | `task/m08-release` | #29 | Add `.github/workflows/release.yml` |

**Merge**: D → `dev/m08`. Checkpoint with user.

### Agent D — Release Workflow (#29)

Create `.github/workflows/release.yml`:

#### Trigger

```yaml
on:
  push:
    tags: ['v*']
```

#### Job 1: `test`

Run full CI checks before building release artifacts:

```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    - uses: Swatinem/rust-cache@v2
    - run: cargo fmt --check
    - run: cargo clippy -- -D warnings
    - run: cargo test
```

#### Job 2: `build` (matrix, depends on `test`)

Build release binaries for all targets:

```yaml
build:
  needs: test
  strategy:
    matrix:
      include:
        - target: x86_64-unknown-linux-gnu
          os: ubuntu-latest
          archive: tar.gz
        - target: aarch64-unknown-linux-gnu
          os: ubuntu-latest
          archive: tar.gz
          cross: true
        - target: x86_64-apple-darwin
          os: macos-latest
          archive: tar.gz
        - target: aarch64-apple-darwin
          os: macos-latest
          archive: tar.gz
        - target: x86_64-pc-windows-msvc
          os: windows-latest
          archive: zip
```

Steps per matrix entry:
1. Checkout
2. Install Rust stable
3. Install `cross` if `matrix.cross` is true
4. Build: `cross build --release --target ${{ matrix.target }}` or
   `cargo build --release --target ${{ matrix.target }}`
5. Package binary into archive:
   - Linux/macOS: `tar czf aigent-{tag}-{target}.tar.gz -C target/{target}/release aigent`
   - Windows: PowerShell `Compress-Archive` to `.zip`
6. Upload archive as artifact

#### Job 3: `release` (depends on `build`)

Create GitHub Release and attach all archives:

```yaml
release:
  needs: build
  runs-on: ubuntu-latest
  permissions:
    contents: write
```

Steps:
1. Checkout (for CHANGES.md access)
2. Download all build artifacts
3. Extract changelog section for the tag version:
   ```bash
   VERSION=${GITHUB_REF_NAME#v}
   sed -n "/^## \[${VERSION}\]/,/^## \[/{/^## \[${VERSION}\]/d;/^## \[/d;p;}" CHANGES.md
   ```
4. Create GitHub Release with `gh release create` or `softprops/action-gh-release`:
   ```yaml
   - uses: softprops/action-gh-release@v2
     with:
       body_path: release-notes.md
       files: artifacts/**/*
   ```

#### Job 4: `publish` (depends on `test`)

Publish to crates.io:

```yaml
publish:
  needs: test
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo publish
  env:
    CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

This job runs in parallel with `build` — it only needs tests to pass,
not the binary builds.

---

## Wave 4 — Verify (depends on Waves 1 + 2 + 3)

Single agent runs the full check suite on `dev/m08`.

| Agent | Branch | Task |
|-------|--------|------|
| E | `dev/m08` | `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release && cargo doc --no-deps` |

Adds `cargo doc --no-deps` to verify that crate-level docs and doc comments
compile cleanly. No doc-test failures should occur.

---

## Deliverables

- `src/lib.rs` — crate-level docs, re-export audit, `#[doc(inline)]` on key types
- `README.md` — full documentation with badges, examples, spec compliance table
- `CHANGES.md` — v0.1.0 changelog
- `.github/workflows/release.yml` — cross-platform release pipeline
- PR: `M8: Main Module & Documentation`

---

## Plan Review Resolution

Findings from `dev/m08/review.md` addressed below.

### Finding 1 (High): Dead LLM code — **Resolved: Not applicable**

The review was written against a stale snapshot (pre-M7 integration). The
final M7 implementation (`src/builder/mod.rs` lines 57–122) fully integrates
LLM support:

- `build_skill` calls `detect_provider()` on line 61
- `spec.no_llm` controls whether provider detection runs (line 57)
- `llm_derive_name`, `llm_generate_description`, `llm_generate_body` are all
  called with per-function fallback to deterministic on error
- All 4 provider implementations are reachable code

No dead code exists. The M8 lib.rs audit proceeds as originally planned.
`LlmProvider` re-export is appropriate — it enables custom providers.

### Finding 2 (Medium): `LlmProvider` re-export path — **Resolved**

Use the two-step approach: `builder/mod.rs` re-exports `LlmProvider` from its
`llm` submodule, then `lib.rs` imports from `builder::*`.

Concretely, add to `src/builder/mod.rs`:
```rust
pub use llm::LlmProvider;
```

Then in `lib.rs`:
```rust
pub use builder::{
    assess_clarity, build_skill, derive_name, init_skill,
    BuildResult, ClarityAssessment, LlmProvider, SkillSpec,
};
```

This keeps `lib.rs` importing only from `builder::*`, consistent with the
existing pattern.

### Finding 3 (Medium): Compliance table completeness — **Resolved**

The table in the plan focuses on **spec-mandated validation rules** — the rows
where aigent differs from the Python reference or adds rules beyond the spec.
Add a clarifying header note:

> Table shows key validation rules from the Anthropic spec. Additional checks
> (frontmatter structure, metadata keys, YAML syntax) are implemented but not
> listed as they are standard parser behavior.

Also add missing rows:
- **Frontmatter: `---` delimiters** — ✅/✅/✅ (standard, all implement)
- **Compatibility ≤ 500 chars** — verify Python reference before publishing

### Finding 4 (Medium): Changelog `sed` extraction fragility — **Resolved**

Escape dots in the version string and handle the last-section case explicitly:

```bash
VERSION=${GITHUB_REF_NAME#v}
VERSION_ESCAPED=$(echo "$VERSION" | sed 's/\./\\./g')
sed -n "/^## \[${VERSION_ESCAPED}\]/,/^## \[/{/^## \[/d;p;}" CHANGES.md > release-notes.md
```

If the file is empty after extraction (version not found), fail the job with
a clear error rather than creating an empty release.

### Finding 5 (Medium): `#![warn(missing_docs)]` — **Accepted**

Add `#![warn(missing_docs)]` to `src/lib.rs` as part of Wave 1 Agent A. This
surfaces undocumented public items during `cargo doc`. Add doc comments to any
public fields on `SkillSpec`, `BuildResult`, and `ClarityAssessment` that
currently lack them.

This aligns with CLAUDE.md: "Public items must have doc comments."

### Finding 6 (Low): README code examples error handling — **Accepted**

README examples will use `.unwrap()` instead of `?` for clarity. Reserve `?`
for the lib.rs doc-tests which are compiled. Example:

```rust
let props = aigent::read_properties(Path::new("my-skill")).unwrap();
```

### Finding 7 (Low): CHANGES.md no "Changed"/"Fixed" sections — **Noted**

Correct for initial release. Only include "Added" section. Future releases
add "Changed" and "Fixed" sections as needed. No action required.

### Finding 8 (Low): `cross` version not pinned — **Accepted**

Pin `cross` to a specific version in the release workflow:
```yaml
- run: cargo install cross --version 0.2.5
```

### Finding 9 (Low): `publish` parallel to `build` — **Noted**

Accepted as-is. `cargo publish` publishes source code to crates.io, not
binaries. A cross-compilation failure would not affect the published crate.
If a platform-specific issue is discovered post-publish, a patch release
(`0.1.1`) can be issued. The efficiency gain from parallelism outweighs the
risk for an initial release.

### Review Checklist

- [x] Finding 1 addressed: LLM code is NOT dead — review was based on stale snapshot
- [x] Finding 2 resolved: `LlmProvider` re-exported via `builder/mod.rs`
- [x] Finding 3 considered: compliance table header clarified, rows verified
- [x] Finding 4 resolved: `sed` extraction uses escaped dots, empty-file guard
- [x] Finding 5 considered: `#![warn(missing_docs)]` added to Wave 1
- [x] Finding 6 noted: README uses `.unwrap()`, lib.rs doc-tests use `?`
- [x] Finding 8 noted: `cross` version pinned to 0.2.5
