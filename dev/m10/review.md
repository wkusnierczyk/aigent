# M10: Improvements and Extensions — Plan Review

## Overall Assessment

M10 is a large scope expansion: 22 issues, 7 waves, 15 agents, and roughly
17 features. At 1151 lines, it is nearly twice the size of any prior milestone
plan. The plan is well-structured and the dependency ordering is sound — the
`Diagnostic` type (X1) correctly precedes everything that depends on structured
output. The wave decomposition reflects real dependency constraints.

However, the scope is a concern. M10 combines a deep cross-cutting refactor
(X1: `Vec<String>` → `Vec<Diagnostic>`, touching 14+ call sites), a new
module (linter), a new subcommand surface (lint, score, doc), CLI flag
proliferation (7+ new flags on `validate` alone), template expansion, prompt
format redesign, plugin hooks, and watch mode. Any one of these — especially
X1 — would be a meaningful milestone on its own.

The plan does acknowledge this by deferring 5 issues to Wave 6, but the
remaining scope (Waves 1–5) is still substantial: 4 new modules
(`diagnostics.rs`, `linter.rs`, `fixer.rs`, `scorer.rs`), 3 new subcommands,
and ~10 new CLI flags across existing subcommands.

## Plan Conformance

### Issues Addressed

- [x] #49 — Checksum verification (Wave 3)
- [x] #50 — Semantic linting (Wave 2)
- [x] #51 — Claude Code field awareness (Wave 1)
- [x] #52 — Batch validation (Wave 2)
- [x] #53 — Fix-it suggestions (Wave 2)
- [x] #54 — Directory structure validation (Wave 6, deferred)
- [x] #55 — Token budget estimation (Wave 3)
- [x] #56 — Multi-format prompt output (Wave 3)
- [x] #57 — Diff-aware prompt updates (Wave 5)
- [x] #58 — Template system (Wave 3)
- [x] #59 — Quality assessment / score (Wave 5)
- [x] #60 — Interactive build mode (Wave 5)
- [x] #61 — Skill upgrade command (Wave 6, deferred)
- [x] #62 — Hooks for continuous validation (Wave 4)
- [x] #63 — Scorer skill (Wave 4)
- [x] #64 — Skill tester (Wave 6, deferred)
- [x] #65 — context:fork for builder (Wave 4)
- [x] #66 — Structured diagnostics (Wave 1)
- [x] #67 — Documentation generation (Wave 5)
- [x] #68 — Watch mode (Wave 5)
- [x] #69 — Cross-skill conflict detection (Wave 6, deferred)
- [x] #70 — README improvements (Wave 6, deferred)

All 22 issues accounted for. 17 in Waves 1–5, 5 deferred to Wave 6.

### Issue Deviations

1. **Issue #54 (directory structure validation) deferred**: The issue says
   "Validate file references, script permissions, and nesting depth." The plan
   defers this entirely to Wave 6. This is reasonable — it depends on X1 and
   has lower priority — but it means the directory validator feature set is
   incomplete. The validator currently only checks frontmatter, not the
   directory structure.

2. **Issue #61 (skill upgrade) deferred**: The plan notes it "overlaps with
   fix-it" (V4). This is a correct observation — `--apply-fixes` on validate
   covers most of the same ground. Deferring avoids redundancy.

3. **Issue #64 (skill tester) marked "highest effort"**: The plan defers it
   with "needs design work." This is the right call — skill testing requires
   simulating Claude Code's discovery mechanism, which is underspecified.

## Findings

### Finding 1 (High): Milestone scope is too large for a single PR

**Location**: Entire plan

22 issues, 17 in-scope features, 4 new modules, 3 new subcommands, ~10 new
CLI flags. For context: M7 (builder) had 6 issues and was the largest
milestone so far. M10 is nearly 4× that scope.

Risks:
- A single PR with this scope is difficult to review. The `dev/m10` branch
  will have dozens of commits touching nearly every file.
- The cross-cutting X1 refactor (diagnostic migration) changes the validator
  return type, error types, CLI handler, builder module, and all validator
  tests. If any Wave 2–5 feature introduces a bug in the diagnostics layer,
  the blast radius is large.
- Wave 5 features (interactive mode, scorer, watch, doc gen) are independent
  enough to be their own milestones.

**Recommendation**: Split M10 into 2–3 milestones:
- **M10a**: Waves 1–2 (diagnostics, target profiles, linting, batch, fix-it)
  — this is the "validator evolution" milestone and has clear dependency order.
- **M10b**: Waves 3–4 (templates, prompt format, budget, checksum, hooks,
  scorer, fork) — "builder & plugin enhancements."
- **M10c**: Wave 5 (interactive, score, doc, watch, diff output) — "polish &
  tooling."

Each would be a substantial but reviewable milestone with a coherent theme.

If the user prefers a single milestone, the plan's wave structure supports
incremental merges into `dev/m10`, but the final PR will be very large.

### Finding 2 (High): `Diagnostic` migration changes the public API — semver concern

**Location**: Wave 1, Agent A — steps 3, 4, 8

The plan changes:
- `validate()` return type: `Vec<String>` → `Vec<Diagnostic>`
- `validate_metadata()` return type: `Vec<String>` → `Vec<Diagnostic>`
- `AigentError::Validation { errors: Vec<String> }` → `Vec<Diagnostic>`

These are **breaking changes** to the library's public API. The crate is at
`0.1.0`, so semver allows breaking changes (pre-1.0), but the plan does not
acknowledge this. Any downstream code (including the plugin skills and tests)
that calls `validate()` and inspects the returned `Vec<String>` will break.

The migration strategy (step 9: "Update all tests") handles in-tree code, but
external consumers would break silently on upgrade.

**Recommendation**: Either (a) acknowledge this as a breaking change and bump
to `0.2.0`, or (b) provide a backward-compatible wrapper:
```rust
pub fn validate(dir: &Path) -> Vec<Diagnostic> { ... }
pub fn validate_strings(dir: &Path) -> Vec<String> {
    validate(dir).iter().map(|d| d.to_string()).collect()
}
```
Option (a) is simpler and appropriate for a 0.x crate.

### Finding 3 (Medium): `--apply-fixes` writes to SKILL.md — dangerous without preview

**Location**: Wave 2, Agent E — Fix-It Suggestions

The plan says `--apply-fixes` will "apply fixable changes and write back" to
SKILL.md. There is no mention of:
- A preview mode showing what would change (dry run)
- A backup of the original file
- Confirmation prompt before writing

Auto-modifying user files is risky, especially for an early-stage tool. If the
fix logic has a bug (e.g., incorrectly truncating a name), the user loses
their original content.

**Recommendation**: Add `--dry-run` or make `--apply-fixes` show a diff first
and require `--apply-fixes --confirm` to actually write. At minimum, log a
warning before overwriting.

### Finding 4 (Medium): Watch mode adds `notify` dependency for a low-priority feature

**Location**: Wave 5, Agent N — X3: Watch Mode

The `notify` crate (version 8) is a non-trivial dependency:
- It adds native file system watcher backends for each platform
- It pulls in several transitive dependencies
- It requires platform-specific testing

The plan lists watch mode as "Low priority, independent of other work" and
places it in Wave 5 (the last implementation wave). Adding a significant
dependency for a low-priority feature is questionable — especially when the
same effect can be achieved with a simple `watch` or `fswatch` shell command
outside the tool.

**Recommendation**: Defer watch mode to a separate milestone or implement it
as an external script (e.g., `fswatch -r skills/ | xargs aigent validate`).
If included, gate it behind a cargo feature flag so the dependency is opt-in.

### Finding 5 (Medium): Hook shell command is fragile and hard to test

**Location**: Wave 4, Agent I — Hooks

The hook command is a single-line shell script:
```bash
file="$(echo '$TOOL_INPUT' | jq -r '.file_path // .file_path // empty')" && \
[ -n "$file" ] && echo "$file" | grep -q 'SKILL\.md$' && \
command -v aigent >/dev/null 2>&1 && aigent validate "$(dirname "$file")" 2>&1 || true
```

Issues:
- Depends on `jq` being installed (not guaranteed, especially on Windows)
- The `.file_path // .file_path` is a typo — it accesses the same field twice.
  The Edit tool uses `file_path` but the Write tool also uses `file_path`, so
  this might be intentional, but the duplicate `//` fallback serves no purpose.
- The `|| true` at the end means validation failures are swallowed — the user
  sees no output unless they check stderr.
- Shell quoting is complex and error-prone in JSON-embedded commands.

**Recommendation**: Address the `jq` dependency. The Claude Code `$TOOL_INPUT`
variable contains JSON, so `jq` is a reasonable dependency on Unix, but the
hook should handle missing `jq` gracefully. Fix the duplicate `.file_path`.

### Finding 6 (Medium): Scorer skill's `allowed-tools` should be tightened

**Location**: Wave 4, Agent J — Scorer Skill

The scorer skill uses `Bash(aigent *)` — the broad pattern. Per the M9 review
Finding 1 (which was resolved for the validator), the scorer only needs
`aigent validate --lint` and potentially `aigent score`. It should not be able
to invoke `aigent build` or `aigent init`.

**Recommendation**: Use `Bash(aigent validate *), Bash(aigent score *)`
for the scorer's `allowed-tools`.

### Finding 7 (Medium): Token budget heuristic `chars / 4` is inaccurate for YAML/XML

**Location**: Wave 3, Agent G — Token Budget Estimation

The plan uses `s.len() / 4` as a token estimate. This is a common heuristic
for English prose, but SKILL.md content includes:
- YAML frontmatter (high token density — `---`, `name:`, `description:` are
  each a token)
- XML tags in prompt output (each `<skill>`, `<name>`, `</name>` is multiple
  tokens)
- Code blocks (code tokenization differs from prose)

For the prompt output specifically, the estimate could be off by 30–50%.

**Recommendation**: Document the heuristic's limitations in the budget output.
Consider a slightly higher ratio (chars / 3.5) for technical content, or
measure against actual tokenizer output for calibration.

### Finding 8 (Low): `SkillTemplate` replaces `template.rs` with `templates.rs`

**Location**: Wave 3, Agent F

The plan says "Create `src/builder/templates.rs` (replace existing
`template.rs`)." The current `template.rs` has `skill_template()` and
`to_kebab_case()`. The replacement changes the file name (singular →
plural) and expands the content.

Renaming files in a git history can make `git blame` harder to follow. A
less disruptive approach would be to extend the existing `template.rs` file
with the new template variants rather than replacing it.

**Recommendation**: Consider keeping the file name `template.rs` and
extending it, or ensure the rename uses `git mv` for proper history tracking.

### Finding 9 (Low): `KNOWN_KEYS` rename has backward-compatibility risk

**Location**: Wave 1, Agent B

The plan renames `KNOWN_KEYS` → `STANDARD_KEYS` and adds `CLAUDE_CODE_KEYS`.
`KNOWN_KEYS` is currently a public constant exported from `parser.rs` and
re-exported from `lib.rs`. Renaming it is a breaking change (see Finding 2).

The plan mentions "Keep `KNOWN_KEYS` as a deprecated alias for backward
compatibility" — this is the right approach but should be called out more
prominently since it affects the public API.

**Recommendation**: Ensure `KNOWN_KEYS` remains exported (deprecated) and
document the rename in CHANGES.md.

### Finding 10 (Low): `ValidationTarget` placement unclear

**Location**: Wave 1, Agent B — Design Decisions

The plan shows `ValidationTarget` in the Design Decisions section as a
standalone enum, and Agent B says to add it to `validate_with_target()`. But
it's not clear which module owns the enum. It could live in:
- `src/validator.rs` (next to the validation functions)
- `src/diagnostics.rs` (alongside the diagnostic types)
- `src/parser.rs` (alongside `KNOWN_KEYS`)

**Recommendation**: Place `ValidationTarget` in `src/diagnostics.rs` alongside
the other validation-related types (`Severity`, `Diagnostic`), since it's used
by the validation pipeline. Re-export from `src/lib.rs`.

### Finding 11 (Low): Interactive mode stdin reading is untestable in CI

**Location**: Wave 5, Agent L

The plan says "Read confirmation from stdin" and "Interactive mode tested with
piped stdin." Piped stdin works for simple yes/no, but complex interactive
flows (multiple prompts, name confirmation, body preview) are hard to test
reliably with piped input.

**Recommendation**: Abstract the IO behind a trait or inject a reader, making
the interactive flow testable without actual stdin. Example:
```rust
fn interactive_build(spec: &SkillSpec, reader: &mut dyn BufRead) -> Result<BuildResult>
```

### Finding 12 (Low): Batch validate JSON output structure not fully specified

**Location**: Wave 2, Agent D

The plan says JSON mode outputs "Array of per-skill results" and the Design
Decisions section shows `{ "path": "...", "diagnostics": [...] }`. But the
text mode summary (PASS/FAIL with counts) has no JSON equivalent specified.

**Recommendation**: Include the summary in the JSON output:
```json
{
  "results": [...],
  "summary": { "total": 3, "passed": 2, "failed": 1 }
}
```

## Observations

1. **X1 (Structured Diagnostics) is the right foundation**: The current
   `Vec<String>` / `starts_with("warning: ")` pattern appears 14 times and is
   fragile. Migrating to `Diagnostic` is the single highest-value change in
   the plan. Every subsequent feature (linting, scoring, fix-it, JSON output,
   batch mode) becomes simpler with structured data.

2. **Wave ordering reflects true dependencies**: Wave 1 (diagnostics + target)
   must precede Wave 2 (linting + fix-it). Wave 2's linting must precede
   Wave 5's scoring. The plan correctly identifies these chains and serializes
   the agents accordingly.

3. **Lint checks are well-scoped**: The 5 lint checks (I001–I005) are all
   derivable from existing parsed data — no new parsing needed. Each is a
   pure function `(SkillProperties, body) → Vec<Diagnostic>`, which is easy
   to test and maintain.

4. **Error code registry is forward-compatible**: The E001–E013, W001–W002,
   I001–I005 scheme with "codes are stable, new codes appended" is the
   standard approach for diagnostic tools (cf. Rust's E0001–E0XXX, ESLint's
   rule names). This enables users to suppress specific codes in CI.

5. **The hook's `|| true` pattern is correct for PostToolUse**: Claude Code
   hooks should not block tool execution. A failing validation hook should
   inform, not prevent the write. The plan gets this right.

6. **Template system fills a real gap**: The current `init` produces only a
   minimal SKILL.md. The 6 templates (minimal, reference-guide,
   domain-specific, workflow, code-skill, claude-code) cover the main patterns
   from the Anthropic spec's "Progressive Disclosure of Information" section.

7. **`context: fork` is a one-line change with high value**: Adding fork
   isolation to the builder skill means Claude Code spawns a separate context
   for skill generation, preventing pollution of the user's current context
   with builder-specific exploration.

8. **The scorer's 60/40 weighting is reasonable**: 60 points for structural
   validity (must-pass) + 40 points for quality (nice-to-have) creates a
   useful gradient. A score of 60 means "valid but improvable," which matches
   intuition.

## Verdict

**Conditional approval** — the plan is technically sound but the scope should
be reconsidered. Finding 1 (scope size) is the primary concern. The plan is
executable as written but would benefit from splitting into 2–3 smaller
milestones (M10a/M10b/M10c).

If the user prefers a single M10, the wave structure supports it, but the
reviewer strongly recommends it be split for reviewability and risk management.

Finding 2 (semver/API breaking change) and Finding 3 (`--apply-fixes` safety)
should be addressed regardless of scope decision.

### Checklist

- [ ] Finding 1 considered: split M10 into 2–3 milestones
- [ ] Finding 2 resolved: acknowledge breaking API changes, bump version or add compat wrapper
- [ ] Finding 3 resolved: add dry-run / confirmation to `--apply-fixes`
- [ ] Finding 4 considered: defer watch mode or gate behind feature flag
- [ ] Finding 5 considered: fix hook `jq` dependency and duplicate `.file_path`
- [ ] Finding 6 resolved: tighten scorer `allowed-tools`
- [ ] Finding 7 noted: document token budget heuristic limitations
- [ ] Finding 9 resolved: keep `KNOWN_KEYS` as deprecated alias
- [ ] Finding 10 resolved: specify `ValidationTarget` module location

---

# M10: Improvements and Extensions — Code Review

## Verification

| Check               | Result |
|----------------------|--------|
| `cargo fmt --check`  | ✅ Clean |
| `cargo clippy -- -D warnings` | ✅ Clean |
| `cargo test`         | ✅ 268 passed (213 unit + 41 cli + 13 plugin + 1 doc-test) |
| `cargo doc --no-deps` | ✅ Clean |

Test count growth: 183 (M9) → 268 (M10) = **+85 tests**.

## Scope

The original M10 plan (22 issues, 7 waves, 15 agents) was split per plan review
Finding 1 into three milestones:

- **M10** (this branch): 5 issues (#50, #51, #52, #53, #66) — structured
  diagnostics, linting, batch validation, fix-it, Claude Code awareness.
- **M11**: 8 issues (deferred — templates, prompt format, budget, checksum,
  hooks, scorer, fork).
- **M12**: 9 issues (deferred — interactive, score, doc, watch, diff, directory
  structure, upgrade, tester, cross-skill, README).

The revised plan is appended to `dev/m10/plan.md` (lines 1152–1498).

## Changed Files

| File | Lines | Status | Summary |
|------|-------|--------|---------|
| `src/diagnostics.rs` | 293 | **New** | `Severity`, `Diagnostic`, error code constants, `ValidationTarget` |
| `src/linter.rs` | 401 | **New** | 5 semantic lint checks (I001–I005) |
| `src/fixer.rs` | 271 | **New** | Auto-fix for E002, E003, E006, E012 |
| `src/lib.rs` | 64 | Modified | New module declarations and re-exports |
| `src/errors.rs` | 186 | Modified | `Validation { errors: Vec<Diagnostic> }` |
| `src/main.rs` | 385 | Modified | `Validate` multi-dir, `--format/--target/--lint/--recursive/--apply-fixes`, new `Lint` subcommand |
| `src/parser.rs` | — | Modified | `require_string`/`optional_string` return `Diagnostic` |
| `src/validator.rs` | 1030 | Modified | Returns `Vec<Diagnostic>`, `ValidationTarget` support, `discover_skills()` |
| `src/builder/mod.rs` | — | Modified | Uses `d.is_error()` instead of string prefix |
| `tests/cli.rs` | +295 | Modified | 18 new integration tests |
| `dev/m10/plan.md` | 1498 | Modified | Revised plan appended |

## Plan Review Finding Resolution

| # | Severity | Finding | Status | Notes |
|---|----------|---------|--------|-------|
| 1 | High | Scope too large | ✅ Addressed | Split into M10/M11/M12 |
| 2 | High | Breaking API (`Vec<String>` → `Vec<Diagnostic>`) | ⚠️ Accepted | Version remains 0.1.0; semver allows pre-1.0 breaks |
| 3 | Medium | `--apply-fixes` needs preview/dry-run | ❌ Not addressed | No `--dry-run`, backup, or confirmation mechanism |
| 4 | Medium | Watch mode `notify` dependency | N/A | Deferred to M12 |
| 5 | Medium | Hook shell command fragility | N/A | Deferred to M11 |
| 6 | Medium | Scorer `allowed-tools` | N/A | Deferred to M11 |
| 7 | Medium | Token budget heuristic | N/A | Deferred to M11 |
| 8 | Low | `template.rs` rename | N/A | Deferred to M11 |
| 9 | Low | `KNOWN_KEYS` rename | ✅ Simpler approach | `KNOWN_KEYS` kept unchanged; `CLAUDE_CODE_KEYS` added in validator.rs — no break, no deprecated alias needed |
| 10 | Low | `ValidationTarget` module location | ✅ Addressed | Placed in `diagnostics.rs` as recommended |
| 11 | Low | Interactive mode testability | N/A | Deferred to M12 |
| 12 | Low | Batch JSON summary | ⚠️ Partial | Multi-dir text summary exists; JSON outputs flat array (single) or array-of-objects (multi) but no summary object |

## Code Findings

### Finding 1 (Medium): `"E000"` used as unregistered catch-all error code

**Location**: `src/validator.rs:347,359,368`, `src/parser.rs:145,152,172`

Six infrastructure-level failures use the string literal `"E000"` as their error
code — but `E000` is not declared as a constant in `diagnostics.rs` and is not
part of the error code registry. These are:

1. "SKILL.md not found" (validator.rs:347)
2. "IO error: {e}" (validator.rs:359)
3. Parse error pass-through (validator.rs:368)
4. YAML parse error (parser.rs:145)
5. YAML structure error (parser.rs:152)
6. Missing frontmatter (parser.rs:172)

These are not validation errors per se — they are precondition failures that
prevent validation from running. Using an unregistered code means:
- `E000` is not documented or discoverable.
- Users cannot reliably filter/suppress it.
- It could collide with a future registered code.

**Recommendation**: Register `E000` in `diagnostics.rs` with a doc comment like
`/// Infrastructure/precondition error (file not found, IO, parse)`, or define
E019/E020 for these categories. Alternatively, return
`Result<Vec<Diagnostic>, AigentError>` from `validate_with_target()` to
separate precondition failures from validation results — but that would be a
larger refactor.

### Finding 2 (Low): `_body` parameter unused in `lint()` function

**Location**: `src/linter.rs:51`

```rust
pub fn lint(properties: &SkillProperties, _body: &str) -> Vec<Diagnostic>
```

The body parameter is accepted but prefixed with `_` to suppress the unused
warning. The five current lint checks only inspect `SkillProperties`. This is
a forward-looking API design (body-based checks like "check for TODO markers"
or "verify example blocks" are likely in M11/M12), but for now it means every
caller must construct a body string that is never read.

No action needed — this is an intentional API reservation. The `read_body()`
helper in `main.rs` already handles extraction for when body-based checks
arrive.

### Finding 3 (Low): Regex compiled on every fixer call — no caching

**Location**: `src/fixer.rs:95,102,111,112`

The `fix_frontmatter_field()`, `lowercase_name_in_frontmatter()`, and
`strip_xml_from_description()` functions call `Regex::new()` on every
invocation. The linter module uses `LazyLock<Regex>` for its pattern (line 42),
but the fixer does not follow the same pattern.

For a tool invoked once per CLI call this is functionally fine — regex
compilation is ~microseconds. But the inconsistency between modules is notable.
The linter's approach (`static LazyLock`) is the idiomatic Rust pattern for
compiled-once regexes.

**Recommendation**: Consider migrating to `LazyLock` for consistency with
`linter.rs`. Low priority — no functional impact.

### Finding 4 (Low): `E008` reserved but no constant defined

**Location**: `src/diagnostics.rs:117`

The comment says `// E008 reserved for name XML tags (caught by E003 character
validation).` but no `pub const E008` is defined. This means the reservation
exists only as a comment — there is no compile-time enforcement. If a future
change adds a new error code and accidentally uses "E008", the comment would
be the only safeguard.

**Recommendation**: Define `pub const E008: &str = "E008";` with a doc comment
marking it as reserved. This costs nothing and makes the reservation visible in
code completion and `cargo doc`.

### Finding 5 (Low): Multi-dir text summary counts "warnings" inconsistently

**Location**: `src/main.rs:200–203`

```rust
let warnings = all_diags
    .iter()
    .filter(|(_, d)| {
        d.iter().any(|d| d.is_warning()) && !d.iter().any(|d| d.is_error())
    })
    .count();
```

A directory that has both errors and warnings is counted only as "errors" — the
warning count reflects directories with warnings *but no errors*. This is
defensible (it's a "worst severity wins" model), but the label "warnings" is
misleading. A directory with 3 errors and 2 warnings shows as an "error" in the
summary, and its warnings are invisible.

**Recommendation**: Either rename to clarify ("warnings-only") or count
warnings independently. Low priority — the current approach matches common CI
tool conventions.

### Finding 6 (Low): `apply_fixes` writes silently — plan review Finding 3 unresolved

**Location**: `src/fixer.rs:72–74`, `src/main.rs:148–159`

The `--apply-fixes` flag applies fixes and writes to SKILL.md with only an
stderr count message (`"Applied {count} fix(es) to ..."`). There is no:
- `--dry-run` flag to preview changes
- File backup before overwriting
- Confirmation prompt
- Diff output showing what changed

This was flagged as plan review Finding 3 (Medium). The implementation does
re-validate after fixing (main.rs:152), which is good — but the user cannot
preview what will change before it happens.

**Recommendation**: Add `--dry-run` in a follow-up (M11 or M12). For now,
document that `--apply-fixes` modifies files in-place and recommend version
control before use.

## Observations

1. **Core migration is complete**: Zero hits for `starts_with("warning: ")` in
   `src/`. The entire codebase now uses `Diagnostic` for structured output. This
   was the highest-value change and it is thoroughly implemented.

2. **Error code registry is well-organized**: The E001–E018 / W001–W002 scheme
   with doc comments on each constant makes the codes self-documenting. The
   separation of lint codes (I001–I005) into `linter.rs` is a good modularity
   choice — lint codes are orthogonal to validation codes.

3. **`KNOWN_KEYS` approach is simpler than planned**: The plan called for
   renaming `KNOWN_KEYS` → `STANDARD_KEYS` with a deprecated alias. The
   implementation keeps `KNOWN_KEYS` unchanged and adds `CLAUDE_CODE_KEYS` in
   `validator.rs`, combined via `known_keys_for(target)`. This avoids a breaking
   change entirely — a better outcome than the plan's approach.

4. **`ValidationTarget` placement matches recommendation**: Plan review Finding
   10 recommended placing it in `diagnostics.rs`. The implementation does
   exactly this, keeping it alongside `Severity` and `Diagnostic`.

5. **Test coverage is comprehensive**: 85 new tests cover all new code paths.
   The CLI integration tests exercise format, target, lint, recursive, and
   apply-fixes flags. The fixer tests cover all 4 fixable error codes plus edge
   cases (no suggestion, empty diagnostics, missing SKILL.md).

6. **`discover_skills()` is a well-scoped utility**: It walks directories
   recursively, skips hidden directories (`.git`, `.vscode`, etc.), and returns
   sorted paths. The sorted output ensures deterministic test behavior across
   platforms.

7. **JSON output follows sensible conventions**: Single directory → flat array
   of diagnostics. Multiple directories → array of `{path, diagnostics}`
   objects. This avoids wrapping single-result queries in unnecessary structure.

8. **The `Lint` subcommand is appropriately separate**: Rather than overloading
   `validate --lint`, there is also a standalone `aigent lint <dir>` command.
   This gives users a clean interface for quality-only checks without validation
   noise. The `--lint` flag on `validate` combines both for CI use cases.

9. **Builder integration is minimal and correct**: `builder/mod.rs` changes are
   limited to replacing `starts_with("warning: ")` with `d.is_error()`. The
   builder does not need to understand diagnostic codes — it only cares about
   pass/fail.

## Verdict

**Ready to merge** — with one advisory.

The structured diagnostics migration (X1/#66) is the central deliverable and it
is thoroughly implemented. The `Diagnostic` type, error code registry, and
`ValidationTarget` tiering provide a solid foundation for M11/M12 features. All
plan review findings that apply to M10's scope are addressed (Finding 1: split,
Finding 9: simpler approach, Finding 10: correct placement).

The advisory concern is Finding 1 (`"E000"` catch-all): these 6 infrastructure
errors should be registered or categorized before M11 adds more diagnostic
consumers. This is not a blocker — the current behavior is correct — but the
unregistered code is a minor inconsistency in an otherwise well-organized
registry.

Plan review Finding 3 (`--apply-fixes` safety) remains unaddressed. This should
be tracked for M11 or M12.

### Checklist

- [x] `cargo fmt --check` passes
- [x] `cargo clippy -- -D warnings` passes
- [x] All 268 tests pass
- [x] `cargo doc --no-deps` clean
- [x] Core migration complete: `starts_with("warning: ")` → 0 hits
- [x] Error code registry (E001–E018, W001–W002, I001–I005) documented
- [x] `ValidationTarget` in `diagnostics.rs` (plan review Finding 10)
- [x] `KNOWN_KEYS` unchanged, `CLAUDE_CODE_KEYS` added (plan review Finding 9)
- [x] Plan split into M10/M11/M12 (plan review Finding 1)
- [x] No new `unwrap()` in library code (convention)
- [x] `#[must_use]` on `Diagnostic` helpers
- [x] All public items have doc comments
- [ ] Register `"E000"` or categorize infrastructure errors (advisory, non-blocking)
- [ ] Add `--dry-run` for `--apply-fixes` (deferred, plan review Finding 3)

## Additional Code Review (2026-02-20)

### Findings

1. Medium: `--format json` changes schema based on input count, which makes machine consumers brittle.
   - References: `src/main.rs:211`, `src/main.rs:216`
   - Single-dir output is `Vec<Diagnostic>`, multi-dir output is `Vec<{path, diagnostics}>`. A caller that always expects one shape will break when validating 1 vs N paths.
   - Recommendation: Always emit one stable envelope for JSON (for example `[{path, diagnostics}]` even for one path), or add an explicit compatibility flag.

2. Medium: `validate --recursive <path/to/SKILL.md>` fails with a usage error instead of validating that skill.
   - References: `src/main.rs:358`, `src/main.rs:359`, `src/main.rs:135`, `src/validator.rs:404`
   - `resolve_dirs()` passes file paths directly to `discover_skills()`, which expects directories (`read_dir` on file returns error, then no results). This is a surprising UX regression because non-recursive mode accepts SKILL.md file paths.
   - Recommendation: In recursive mode, detect file inputs and resolve them via `resolve_skill_dir()` before discovery.

3. Low: `--apply-fixes` reports inflated fix counts when multiple diagnostics map to the same edit.
   - References: `src/fixer.rs:33`, `src/fixer.rs:52`, `src/fixer.rs:53`, `src/fixer.rs:72`
   - For a name like `ABC`, validator emits multiple `E003` diagnostics; `apply_fixes()` lowercases the same field repeatedly and increments `fix_count` for each diagnostic, even though only one effective change is applied.
   - Recommendation: Count unique field-level edits or increment only when `modified` actually changes.

### Residual Testing Gaps

1. No CLI integration test for recursive mode with an input file path (`SKILL.md`) to prevent the regression above.
2. No CLI integration test that locks JSON output shape across one-path and multi-path validation invocations.
3. No fixer test asserting count semantics when multiple diagnostics target the same field.
