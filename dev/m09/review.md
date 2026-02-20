# M9: Claude Code Plugin — Plan Review

## Overall Assessment

The M9 plan packages aigent as a Claude Code plugin with two skills (builder,
validator), a plugin manifest, an install script, and 13 tests. The plan is
well-structured and the hybrid mode design (CLI vs prompt-only) is
architecturally sound. The "dogfooding" aspect — validating the plugin's own
skills with aigent's own validator — is particularly strong.

The plan is compact (614 lines, 4 waves, 5 agents) and appropriately scoped
for what is primarily a packaging and distribution milestone.

## Plan Conformance

### Issues Addressed

- [x] #31 — aigent-builder skill (Wave 1)
- [x] #32 — aigent-validator skill (Wave 1)
- [x] #33 — plugin.json manifest (Wave 2)
- [x] #34 — plugin tests (Wave 3)
- [x] #35 — standalone binary / release (prerequisite from M8, referenced)
- [x] #36 — install script (Wave 2)

### Issue Deviations

1. **Issue #34 mentions integration tests for builder/validator invocation**:
   The issue says "Integration tests: invoke builder with a sample
   description; invoke validator on known-good and known-bad skills." The plan
   tests skill *content* (frontmatter fields, names) and self-validation
   (aigent validate on the skills), but does not test the skills as Claude
   Code skills (i.e., simulating Claude invoking the skill). This is a
   reasonable omission — testing Claude Code skill invocation requires a Claude
   Code runtime, which is not available in CI.

2. **Issue #35 already covered by M8**: The plan correctly notes that M8's
   release workflow produces the binaries. Issue #35 is listed as a dependency,
   not a deliverable. However, the plan lists it in the issues line (#35)
   without noting that it's a prerequisite rather than in-scope work.

3. **Issue #36 mentions `/usr/local/bin`**: The issue says "install to
   `~/.local/bin` or `/usr/local/bin`." The plan only targets `~/.local/bin`
   (no sudo). This is the right choice for a user-level install script, but
   deviates from the issue's alternative.

## Findings

### Finding 1 (Medium): `allowed-tools` pattern `Bash(aigent *)` may be too broad

**Location**: Both skill frontmatter sections

The `allowed-tools: Bash(aigent *), Bash(command -v *), Write, Read, Glob`
pattern permits any `aigent` subcommand. This is intentionally broad for the
builder (which needs `build` and `validate`) but may be overly permissive for
the validator (which only needs `validate`).

For the validator skill, a tighter pattern would be:
```yaml
allowed-tools: Bash(aigent validate *), Bash(command -v *), Read, Glob
```

This follows the principle of least privilege — the validator skill shouldn't
be able to invoke `aigent build` or `aigent init`.

**Recommendation**: Tighten the validator's `allowed-tools` to
`Bash(aigent validate *)`. Keep the builder's broader `Bash(aigent *)` since
it needs both `build` and `validate`.

### Finding 2 (Medium): `install.sh` uses GitHub API without authentication

**Location**: Wave 2, install script

The script calls:
```bash
curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest"
```

The GitHub API has a rate limit of 60 requests/hour for unauthenticated
requests. In CI or automated environments, this can be exhausted quickly.
More importantly, if the user runs the install script behind a corporate
proxy or in a restricted environment, the API call may fail silently.

**Recommendation**: Add a fallback mechanism. If the API call fails, attempt
to use the known latest version from the script itself (hardcoded, updated at
release time). Alternatively, use the GitHub releases redirect URL:
`https://github.com/${REPO}/releases/latest/download/${ASSET}` — which
doesn't require API access but does require knowing the asset name pattern.

### Finding 3 (Medium): Version sync test uses string parsing instead of `toml` crate

**Location**: Wave 3, test #7

The plan extracts the version from `Cargo.toml` using:
```rust
let cargo_version = cargo.lines()
    .find(|l| l.starts_with("version"))
    .and_then(|l| l.split('"').nth(1))
    .unwrap();
```

This is fragile — it matches any line starting with `version`, including
lines in `[dependencies]` sections (e.g., `version = "0.12"` for a
dependency). In the current `Cargo.toml`, the `[package]` section's `version`
is on line 3, before any dependencies, so it works. But adding a workspace or
reordering sections would break it.

**Recommendation**: Either (a) anchor to the `[package]` section first, then
find `version`, or (b) use a more robust regex:
```rust
let re = regex::Regex::new(r#"(?m)^\[package\].*?^version\s*=\s*"([^"]+)""#).unwrap();
```
Or simply use `env!("CARGO_PKG_VERSION")` in the test — the Rust compiler
already parses `Cargo.toml` at build time:
```rust
let cargo_version = env!("CARGO_PKG_VERSION");
```
This is simpler, guaranteed correct, and requires no file parsing.

### Finding 4 (Low): Builder skill defaults output to `.claude/skills/<name>/`

**Location**: Design Decisions, Builder Skill body

The skill body says to write to `.claude/skills/<name>/SKILL.md`. This is a
Claude Code-specific path. The `aigent build` CLI defaults to `./<name>/` in
the current directory. The skill overrides this with `--dir .claude/skills/<name>/`.

This is the correct behavior for a Claude Code plugin (skills should go in
`.claude/skills/`), but the discrepancy between CLI default and skill default
should be noted in the skill body so users understand the override.

### Finding 5 (Low): `install.sh` doesn't verify checksums

**Location**: Wave 2, install script

Issue #36 mentions "optionally verifies checksum." The plan's script doesn't
include any checksum verification. For a user-level install script that
downloads binaries from the internet, this is a security gap.

**Recommendation**: At minimum, document that checksum verification is not
implemented. Ideally, the release workflow (M8) should generate SHA256 sums
alongside the archives, and the install script should verify them. This can
be deferred to a follow-up, but should be noted.

### Finding 6 (Low): Self-validation tests depend on `aigent` binary being built

**Location**: Wave 3, tests #1-2

The self-validation tests run `aigent validate skills/aigent-builder/`. This
uses the CLI binary, which must be built before the test runs. `assert_cmd`
handles this via `cargo_bin_cmd!`, but the test needs the binary to be
compiled. This is standard for integration tests and will work — but the
tests also depend on the skills files existing at the hardcoded relative
paths `skills/aigent-builder/` and `skills/aigent-validator/`. These paths
are relative to the test's working directory, which `cargo test` sets to the
project root. This is correct but worth noting.

## Observations

1. **Hybrid mode is excellent architecture**: The skill works without the
   CLI (prompt-only mode using Claude's spec knowledge) while being strictly
   better with it (authoritative validation, deterministic output). This
   means the plugin is immediately usable by anyone who installs the Claude
   Code plugin — no Rust toolchain needed.

2. **Dogfooding is a strong testing pattern**: Tests #1-2 validate the
   plugin's own skills with the project's own validator. If this fails,
   either the skill or the validator has a bug — both are valuable signals.

3. **POSIX `sh` is the right choice**: Using `#!/bin/sh` instead of
   `#!/bin/bash` maximizes portability across Linux, macOS, and minimal
   Docker containers. The script avoids bashisms (`[[ ]]`, arrays, etc.).

4. **Wave parallelism is well-utilized**: Wave 1 creates both skills in
   parallel. Wave 2 creates the manifest and install script. This is
   efficient since the skills don't depend on each other.

5. **No `mockall` or complex test infrastructure**: The 13 tests are
   straightforward file checks and CLI invocations. No mocking needed since
   the tests verify concrete artifacts.

6. **`argument-hint` in frontmatter**: The `argument-hint` field tells
   Claude Code what kind of argument the skill expects. This is a nice UX
   touch — `/aigent:aigent-builder [skill-description]` guides the user.

## Verdict

**Approved** — the plan is well-structured and appropriately scoped. Finding 1
(validator `allowed-tools` scope) and Finding 3 (version sync parsing) should
be addressed but are not blocking. Finding 2 (API rate limiting) is a
production concern worth noting.

### Checklist

- [x] Finding 1 considered: validator `allowed-tools` scoped to `aigent validate *`
- [x] Finding 2 noted: install script API rate limit fallback
- [x] Finding 3 resolved: use `env!("CARGO_PKG_VERSION")` for version sync test
- [ ] Finding 5 noted: checksum verification documented or deferred

---

# M9: Claude Code Plugin — Code Review

## Verification

```
cargo fmt --check         # ✅ clean
cargo clippy -- -D warnings # ✅ clean
cargo test                # ✅ 183 passed (146 unit + 23 cli + 13 plugin + 1 doc-test)
cargo doc --no-deps       # ✅ clean, no warnings
```

Note: The test count increased from 170 (M8) to 183 — the new
`tests/plugin.rs` adds 13 integration tests. No Rust source files, `Cargo.toml`,
or `Cargo.lock` were modified; M9 is purely additive (5 new files).

## Changed Files

| File | Lines | Type |
|------|-------|------|
| `skills/aigent-builder/SKILL.md` | 92 | New — builder skill |
| `skills/aigent-validator/SKILL.md` | 61 | New — validator skill |
| `.claude-plugin/plugin.json` | 13 | New — plugin manifest |
| `install.sh` | 65 | New — binary install script |
| `tests/plugin.rs` | 134 | New — plugin integration tests |

No existing files were modified. Total: 365 lines added across 5 files.

## Plan Review Finding Resolution

All 6 plan review findings have been addressed or noted.

### Finding 1 (Medium): Validator `allowed-tools` — ✅ Resolved

The validator skill frontmatter (line 8) uses the tightened pattern:
```yaml
allowed-tools: Bash(aigent validate *), Bash(command -v *), Read, Glob
```

This scopes the validator to only `aigent validate` subcommands, following the
principle of least privilege. Additionally, `Write` was removed from the
validator's tool list (the builder retains it) — the validator only reads
and reports, it never writes files.

The builder retains the broader `Bash(aigent *)` since it needs both `build`
and `validate`.

### Finding 2 (Medium): Install script API rate limit — ✅ Resolved

The plan proposed using the GitHub API endpoint
(`api.github.com/repos/.../releases/latest`), which has a 60-req/hour rate
limit for unauthenticated requests. The implementation instead uses the
redirect-based approach (lines 30–31):

```bash
VERSION=$(curl -fsSI "https://github.com/${REPO}/releases/latest" \
  | grep -i '^location:' | sed 's|.*/tag/||;s/[[:space:]]*$//')
```

This sends a HEAD request to the GitHub releases page, which returns a
`302 Found` with a `Location` header pointing to the tagged release page.
The `sed` command extracts the tag name from the URL. This approach:
- Avoids the API entirely (no rate limiting)
- Works behind corporate proxies that block API access but allow web access
- Is simpler (no JSON parsing needed)

### Finding 3 (Medium): Version sync test — ✅ Resolved

The test (line 73) uses `env!("CARGO_PKG_VERSION")` instead of parsing
`Cargo.toml` manually:

```rust
let cargo_version = env!("CARGO_PKG_VERSION");
```

This is the ideal solution — `env!` evaluates at compile time using Cargo's
own TOML parser, so it's guaranteed correct regardless of `Cargo.toml` layout.
No fragile string parsing or additional dependencies needed.

### Finding 4 (Low): Builder output path discrepancy — ✅ Resolved

The builder skill body (lines 42–44) explicitly documents the `--dir` override:

> The CLI defaults to outputting in `./<name>/` in the current directory. The
> `--dir` flag above overrides this to place skills in `.claude/skills/`, which
> is the standard Claude Code skill location.

This makes the discrepancy between CLI default and skill default clear to users.

### Finding 5 (Low): Checksum verification — Not addressed

The install script does not include checksum verification, and no documentation
of the omission was added. The release workflow does not generate SHA256 sums.
This remains a future improvement.

### Finding 6 (Low): Self-validation test dependencies — ✅ Noted

The tests work correctly. `assert_cmd`'s `cargo_bin_cmd!` macro handles binary
compilation, and `cargo test` sets the working directory to the project root,
making the relative paths `skills/aigent-builder/` and `skills/aigent-validator/`
resolve correctly.

## Code Findings

### Finding 1 (Low): Install script `VERSION` variable retains `v` prefix — correct

**Location**: `install.sh` lines 30–31, 39

The `VERSION` variable extracted from the redirect URL retains the `v` prefix
(e.g., `v0.1.0`). The asset name is constructed as `aigent-${VERSION}-${TARGET}.tar.gz`,
producing `aigent-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`.

The release workflow (`release.yml` line 81) uses `${{ github.ref_name }}` in the
archive name, which is the full tag (e.g., `v0.1.0`). So the naming matches.

Notably, the plan's install script had `ASSET="aigent-${VERSION#v}-${TARGET}.tar.gz"`
which would have *stripped* the `v` prefix, creating a mismatch with the release
assets. The implementation correctly keeps the `v` prefix. Good catch during
implementation.

### Finding 2 (Low): Install script `tar xz` expects flat archive structure

**Location**: `install.sh` line 45

The install script runs:
```bash
curl -fsSL "$URL" | tar xz -C "$INSTALL_DIR"
```

This assumes the tarball contains the `aigent` binary at the archive root (no
subdirectory). The release workflow (`release.yml` lines 79–82) creates the
archive with:
```bash
cd target/${{ matrix.target }}/release
tar czf ../../../aigent-*.tar.gz aigent
```

The `cd` + bare `aigent` ensures the binary is at the archive root. The
assumption is correct, but fragile — if someone adds files to the archive or
changes the packaging step to include a directory prefix, the install script
would silently extract to wrong paths.

Not blocking — the current implementation is correct and the release workflow
is the only source of archives.

### Finding 3 (Low): Install script download error message is good

**Location**: `install.sh` lines 45–48

The implementation wraps the download in an `if ! ...; then` guard:
```bash
if ! curl -fsSL "$URL" | tar xz -C "$INSTALL_DIR"; then
  echo "Error: download failed — check that a release exists for ${TARGET}" >&2
  exit 1
fi
```

The plan's version used bare `curl | tar` with no error handling — if the
download failed, `tar` would get garbage input and produce a confusing error.
The implementation's explicit error message is a clear improvement.

### Finding 4 (Low): Plugin manifest `description` differs from plan

**Location**: `.claude-plugin/plugin.json` line 3

The plan specified:
> "AI agent skill builder and validator — create and validate SKILL.md files"

The implementation uses:
> "AI agent skill builder and validator — create and validate SKILL.md files following the Anthropic agent skill specification"

The implementation's version adds "following the Anthropic agent skill
specification" — a useful clarification that helps users understand what
specification the tools enforce. This is a good deviation from the plan.

### Finding 5 (Low): Test helper `read_plugin_json()` is well-factored

**Location**: `tests/plugin.rs` lines 32–36

The helper function `read_plugin_json()` is used by 5 of the 13 tests. It
handles both file reading and JSON parsing in one place, with descriptive
`expect()` messages. This follows the same pattern as `fn aigent() -> Command`
in `tests/cli.rs`. Good test code organization.

### Finding 6 (Low): `#[cfg(unix)]` guard on executable permission test

**Location**: `tests/plugin.rs` lines 119–124

The executable permission check uses `#[cfg(unix)]`:
```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::metadata(path).unwrap().permissions();
    assert!(perms.mode() & 0o111 != 0, "install.sh should be executable");
}
```

This correctly handles cross-platform compilation — `PermissionsExt` and
`mode()` are Unix-only. On Windows, the test only checks `path.exists()`.
Since `install.sh` is a POSIX shell script and only relevant on Unix, this
is the right approach.

### Finding 7 (Info): Builder skill body matches plan verbatim

**Location**: `skills/aigent-builder/SKILL.md`

The builder skill body (93 lines) matches the plan's specification almost
exactly, with one addition: lines 42–44 explaining the `--dir` override
(addressing plan review Finding 4). All section headings, code blocks, and
rule lists are as planned.

### Finding 8 (Info): Validator skill body matches plan verbatim

**Location**: `skills/aigent-validator/SKILL.md`

The validator skill body (62 lines) matches the plan's specification exactly.
The `allowed-tools` was tightened per Finding 1 in the frontmatter, but the
body is unchanged.

## Observations

1. **Zero Rust code changes**: M9 is a pure packaging milestone — 5 new files,
   no modifications to existing source. This is the cleanest possible scope for
   a plugin milestone. The test count grows from 170 to 183, entirely from the
   new `tests/plugin.rs`.

2. **Dogfooding works**: Both `validate_builder_skill` and
   `validate_validator_skill` tests pass — aigent's own validator confirms the
   plugin's skills are spec-compliant. This is exactly the validation loop the
   plan intended.

3. **Version sync enforced at compile time**: The `env!("CARGO_PKG_VERSION")`
   approach is strictly better than the plan's string-parsing approach — it's
   computed at build time by the Rust compiler itself, so there's zero chance
   of parsing errors or section-matching bugs.

4. **Install script is better than the plan**: Three improvements over the plan:
   (a) redirect-based version detection avoids API rate limits,
   (b) download error handling with clear error message,
   (c) `VERSION` retains `v` prefix to match release asset naming.

5. **Skills are concise**: Builder at 93 lines and validator at 62 lines — both
   well under the 500-line recommendation. The hybrid mode sections are clear
   without being verbose.

6. **`argument-hint` works as documented**: Both skills have `argument-hint`
   fields that will display in Claude Code's skill picker, guiding users to
   provide the right input format.

7. **Plugin manifest is minimal and correct**: 13 lines of JSON, no unnecessary
   fields. Skills are auto-discovered from `skills/` — the manifest doesn't
   need to enumerate them.

## Verdict

**Ready to merge** — all deliverables match the plan, 4 of 6 plan review
findings resolved (Finding 5 deferred, Finding 6 noted), all verification
checks pass (fmt, clippy, 183 tests, doc). Eight code findings, all Low/Info
severity, none blocking.

### Checklist

- [x] Plan review findings 1–6 verified
- [x] `cargo fmt --check` clean
- [x] `cargo clippy -- -D warnings` clean
- [x] `cargo test` — 183 tests pass
- [x] `cargo doc --no-deps` clean, no warnings
- [x] Skills pass self-validation (dogfooding)
- [x] Plugin manifest valid JSON, version synced
- [x] Install script uses redirect (no API rate limit)
- [x] Version sync test uses `env!("CARGO_PKG_VERSION")`
- [x] Validator `allowed-tools` scoped to `aigent validate *`
- [x] No existing Rust source files modified
- [ ] Finding 5 deferred: checksum verification not implemented
